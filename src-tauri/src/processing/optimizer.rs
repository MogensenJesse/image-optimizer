use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::Mutex;
use tauri_plugin_shell::{ShellExt, process::Output};
use crate::core::OptimizationResult;
use crate::worker::ImageTask;
use crate::utils::{
    OptimizerError,
    OptimizerResult,
};
use crate::processing::validation::validate_task;
use serde_json;
use tracing::{debug, warn};

const BATCH_SIZE: usize = 10;

#[derive(Clone)]
pub struct ImageOptimizer {
    active_tasks: Arc<Mutex<HashSet<String>>>,
}

impl ImageOptimizer {
    pub fn new() -> Self {
        Self {
            active_tasks: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub async fn process_batch(
        &self,
        app: &tauri::AppHandle,
        tasks: Vec<ImageTask>,
    ) -> OptimizerResult<Vec<OptimizationResult>> {
        let mut results = Vec::with_capacity(tasks.len());
        
        debug!("Starting batch processing of {} tasks", tasks.len());
        debug!("Using chunk size: {}", BATCH_SIZE);
        
        // Process tasks in chunks to reduce process spawning
        for (chunk_index, chunk) in tasks.chunks(BATCH_SIZE).enumerate() {
            debug!(
                "Processing chunk {}/{} ({} tasks)", 
                chunk_index + 1, 
                (tasks.len() + BATCH_SIZE - 1) / BATCH_SIZE,
                chunk.len()
            );
            
            let chunk_results = match self.process_chunk(app, chunk.to_vec()).await {
                Ok(res) => {
                    debug!("Chunk {} processed successfully", chunk_index + 1);
                    res
                },
                Err(e) => {
                    warn!("Chunk {} failed: {}", chunk_index + 1, e);
                    return Err(e);
                }
            };
            results.extend(chunk_results);
        }
        
        debug!("Batch processing completed. Processed {} tasks", results.len());
        Ok(results)
    }

    async fn process_chunk(
        &self,
        app: &tauri::AppHandle,
        tasks: Vec<ImageTask>,
    ) -> OptimizerResult<Vec<OptimizationResult>> {
        let mut results = Vec::with_capacity(tasks.len());

        debug!("Validating {} tasks in chunk", tasks.len());
        // Validate paths and get original sizes
        for task in &tasks {
            // Validate task (includes path and settings validation)
            validate_task(task).await?;
            
            // Track active task
            let mut active = self.active_tasks.lock().await;
            active.insert(task.input_path.clone());
            debug!("Task validated: {}", task.input_path);
        }

        // Process the batch using Sharp sidecar
        debug!("Sending tasks to Sharp sidecar for processing");
        let result = self.run_sharp_process_batch(app, &tasks).await?;
        let (sharp_result, output) = result;

        // Remove from active tasks and collect results
        let mut active = self.active_tasks.lock().await;
        for task in &tasks {
            active.remove(&task.input_path);
        }

        match sharp_result {
            Ok(_) => {
                debug!("Collecting results for {} tasks", tasks.len());
                // Parse Sharp output to get results
                let stdout = String::from_utf8_lossy(&output.stdout);
                let sharp_results: Vec<serde_json::Value> = serde_json::from_str(&stdout)
                    .map_err(|e| OptimizerError::processing(format!(
                        "Failed to parse Sharp output: {}", e
                    )))?;

                // Collect results for each task
                for (task, result) in tasks.into_iter().zip(sharp_results) {
                    let output_path = result["path"].as_str()
                        .ok_or_else(|| OptimizerError::processing("Missing path in Sharp output".to_string()))?
                        .to_string();
                    
                    let optimized_size = result["optimizedSize"].as_u64()
                        .ok_or_else(|| OptimizerError::processing("Missing optimizedSize in Sharp output".to_string()))?;

                    let original_size = result["originalSize"].as_u64()
                        .ok_or_else(|| OptimizerError::processing("Missing originalSize in Sharp output".to_string()))?;

                    let bytes_saved = result["savedBytes"].as_i64()
                        .ok_or_else(|| OptimizerError::processing("Missing savedBytes in Sharp output".to_string()))?;

                    let compression_ratio = result["compressionRatio"].as_str()
                        .ok_or_else(|| OptimizerError::processing("Missing compressionRatio in Sharp output".to_string()))?
                        .parse::<f64>()
                        .unwrap_or(0.0);

                    debug!(
                        "Task completed - Path: {}, Original: {}, Optimized: {}, Saved: {}%",
                        task.input_path, original_size, optimized_size, compression_ratio
                    );

                    results.push(OptimizationResult {
                        original_path: task.input_path,
                        optimized_path: output_path,
                        original_size,
                        optimized_size,
                        success: true,
                        error: None,
                        saved_bytes: bytes_saved,
                        compression_ratio,
                    });
                }
                Ok(results)
            }
            Err(e) => {
                warn!("Error saving results: {}", e);
                Err(e)
            }
        }
    }

    async fn run_sharp_process_batch(&self, app: &tauri::AppHandle, tasks: &[ImageTask]) -> OptimizerResult<(OptimizerResult<()>, Output)> {
        let command = app.shell()
            .sidecar("sharp-sidecar")
            .map_err(|e| OptimizerError::sidecar(format!("Failed to create Sharp sidecar: {}", e)))?;

        // Create batch task data
        let batch_data = tasks.iter().map(|task| {
            debug!("Preparing task - Input: {}, Output: {}", task.input_path, task.output_path);
            serde_json::json!({
                "input": task.input_path,
                "output": task.output_path,
                "settings": task.settings
            })
        }).collect::<Vec<_>>();

        let batch_json = serde_json::to_string(&batch_data)
            .map_err(|e| OptimizerError::processing(format!("Failed to serialize batch settings: {}", e)))?;

        debug!("Executing Sharp sidecar command with {} tasks", tasks.len());
        let output = command
            .args(&[
                "optimize-batch",
                &batch_json,
            ])
            .output()
            .await
            .map_err(|e| {
                let error_message = format!("Failed to run Sharp process: {}", e);
                warn!("Sharp process error: {}", error_message);
                OptimizerError::sidecar(error_message)
            })?;

        let result = if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let error_message = format!("Sharp process failed: {}", stderr);
            warn!("Sharp process error: {}", error_message);
            Err(OptimizerError::sidecar(error_message))
        } else {
            debug!("Sharp sidecar command completed successfully");
            // Parse and validate Sharp output
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !stdout.is_empty() {
                debug!("Sharp sidecar output: {}", stdout);
                
                // Parse Sharp output JSON
                let sharp_results: Vec<serde_json::Value> = serde_json::from_str(&stdout)
                    .map_err(|e| OptimizerError::processing(format!(
                        "Failed to parse Sharp output: {}", e
                    )))?;
                
                // Verify each result matches a task and the file exists
                for (task, result) in tasks.iter().zip(sharp_results.iter()) {
                    let output_path = result["path"].as_str().ok_or_else(|| 
                        OptimizerError::processing("Missing path in Sharp output".to_string())
                    )?;
                    
                    let success = result["success"].as_bool().unwrap_or(false);
                    if !success {
                        let error = result["error"].as_str().unwrap_or("Unknown error");
                        let error_message = format!(
                            "Sharp processing failed for {}: {}", 
                            task.input_path, error
                        );
                        warn!("{}", error_message);
                        return Ok((Err(OptimizerError::processing(error_message)), output));
                    }
                    
                    // Verify the output file exists
                    match tokio::fs::metadata(output_path).await {
                        Ok(_) => {
                            debug!("Output file exists: {}", output_path);
                        },
                        Err(e) => {
                            let error_message = format!(
                                "Output file missing: {} (Error: {})", output_path, e
                            );
                            warn!("{}", error_message);
                            return Ok((Err(OptimizerError::processing(error_message)), output));
                        }
                    }
                }
            }
            Ok(())
        };

        Ok((result, output))
    }

    pub async fn get_active_tasks(&self) -> Vec<String> {
        self.active_tasks.lock().await.iter().cloned().collect()
    }
} 