use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::Mutex;
use tauri_plugin_shell::ShellExt;
use crate::core::OptimizationResult;
use crate::worker::ImageTask;
use crate::utils::{
    OptimizerError,
    OptimizerResult,
    get_file_size,
};
use crate::processing::validation::validate_task;
use serde_json;
use tracing::debug;

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
        
        // Process tasks in chunks to reduce process spawning
        for chunk in tasks.chunks(BATCH_SIZE) {
            let chunk_results = self.process_chunk(app, chunk.to_vec()).await?;
            results.extend(chunk_results);
        }
        
        Ok(results)
    }

    async fn process_chunk(
        &self,
        app: &tauri::AppHandle,
        tasks: Vec<ImageTask>,
    ) -> OptimizerResult<Vec<OptimizationResult>> {
        let mut results = Vec::with_capacity(tasks.len());
        let mut original_sizes = Vec::with_capacity(tasks.len());

        // Validate paths and get original sizes
        for task in &tasks {
            // Validate task (includes path and settings validation)
            validate_task(task).await?;
            
            // Get original file size
            let original_size = get_file_size(&task.input_path).await?;
            original_sizes.push(original_size);

            // Track active task
            let mut active = self.active_tasks.lock().await;
            active.insert(task.input_path.clone());
        }

        // Process the batch using Sharp sidecar
        let result = self.run_sharp_process_batch(app, &tasks).await;

        // Remove from active tasks and collect results
        let saving_result = async {
            let mut active = self.active_tasks.lock().await;
            for task in &tasks {
                active.remove(&task.input_path);
            }

            match result {
                Ok(_) => {
                    // Collect results for each task
                    for (task, original_size) in tasks.into_iter().zip(original_sizes) {
                        let optimized_size = get_file_size(&task.output_path).await?;

                        let bytes_saved = original_size as i64 - optimized_size as i64;
                        let compression_ratio = if original_size > 0 {
                            (bytes_saved as f64 / original_size as f64) * 100.0
                        } else {
                            0.0
                        };

                        results.push(OptimizationResult {
                            original_path: task.input_path,
                            optimized_path: task.output_path,
                            original_size,
                            optimized_size,
                            success: true,
                            error: None,
                            saved_bytes: bytes_saved,
                            compression_ratio,
                        });
                    }
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }.await;

        match saving_result {
            Ok(_) => Ok(results),
            Err(e) => Err(e),
        }
    }

    async fn run_sharp_process_batch(&self, app: &tauri::AppHandle, tasks: &[ImageTask]) -> OptimizerResult<()> {
        let command = app.shell()
            .sidecar("sharp-sidecar")
            .map_err(|e| OptimizerError::sidecar(format!("Failed to create Sharp sidecar: {}", e)))?;

        // Create batch task data
        let batch_data = tasks.iter().map(|task| {
            serde_json::json!({
                "input": task.input_path,
                "output": task.output_path,
                "settings": task.settings
            })
        }).collect::<Vec<_>>();

        let batch_json = serde_json::to_string(&batch_data)
            .map_err(|e| OptimizerError::processing(format!("Failed to serialize batch settings: {}", e)))?;

        let output = command
            .args(&[
                "optimize-batch",
                &batch_json,
            ])
            .output()
            .await
            .map_err(|e| {
                let error_message = format!("Failed to run Sharp process: {}", e);
                debug!("Error during conversion: {}", error_message);
                OptimizerError::sidecar(error_message)
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let error_message = format!("Sharp process failed: {}", stderr);
            debug!("Error during conversion: {}", error_message);
            Err(OptimizerError::sidecar(error_message))
        } else {
            Ok(())
        }
    }

    pub async fn get_active_tasks(&self) -> Vec<String> {
        self.active_tasks.lock().await.iter().cloned().collect()
    }
} 