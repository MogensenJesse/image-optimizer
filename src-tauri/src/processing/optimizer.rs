use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::Mutex;
use tauri_plugin_shell::{ShellExt, process::Output};
use tauri;
use crate::core::OptimizationResult;
use crate::worker::ImageTask;
use crate::utils::{
    OptimizerError,
    OptimizerResult,
};
use crate::processing::validation::validate_task;
use serde_json;
use tracing::{debug, warn};
use sysinfo::System;

#[derive(Debug, Clone)]
struct BatchSizeConfig {
    min_size: usize,
    max_size: usize,
    target_memory_usage: usize, // in bytes
    target_memory_percentage: f32, // percentage of available memory to use
}

impl Default for BatchSizeConfig {
    fn default() -> Self {
        Self {
            min_size: 5,
            max_size: 75,
            target_memory_usage: 1024 * 1024 * 2048, // Keep 2GB limit as safety net
            target_memory_percentage: 0.5,     // Increased from 0.3 to 0.5 (50% of available memory)
        }
    }
}

#[derive(Debug, Clone)]
pub struct BatchMemoryMetrics {
    pub initial_memory: usize,
    pub avg_batch_memory: usize,
    pub peak_pressure: usize,
    pub memory_distribution: [usize; 3],
}

impl BatchMemoryMetrics {
    fn new(initial_memory: usize) -> Self {
        Self {
            initial_memory,
            avg_batch_memory: 0,
            peak_pressure: 0,
            memory_distribution: [0; 3],
        }
    }

    fn record_usage(&mut self, used_memory: usize, available_memory: usize) {
        // Update average (exponential moving average with alpha=0.2)
        if self.avg_batch_memory == 0 {
            self.avg_batch_memory = used_memory;
        } else {
            self.avg_batch_memory = (used_memory / 5) + (self.avg_batch_memory * 4 / 5);
        }
        
        // Update peak pressure (track highest memory usage)
        self.peak_pressure = self.peak_pressure.max(used_memory);
        
        // Update distribution based on percentage of initial memory used
        let usage_pct = (used_memory as f64 / self.initial_memory as f64) * 100.0;
        let index = (usage_pct / 33.33).min(2.0) as usize;
        self.memory_distribution[index] += 1;

        debug!(
            "Memory usage recorded - Used: {}MB, Available: {}MB, Usage: {:.1}%, Index: {}", 
            used_memory / (1024 * 1024),
            available_memory / (1024 * 1024),
            usage_pct,
            index
        );
    }
}

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

    pub async fn process_batch(&self, app: &tauri::AppHandle, tasks: Vec<ImageTask>) -> OptimizerResult<(Vec<OptimizationResult>, BatchMemoryMetrics)> {
        let batch_size = self.calculate_batch_size(&tasks);
        debug!("Calculated optimal batch size: {}", batch_size);
        
        let mut results = Vec::with_capacity(tasks.len());
        let available_mem = self.get_available_memory();
        
        // Initialize memory metrics
        let mut memory_metrics = BatchMemoryMetrics::new(available_mem);
        
        for (chunk_index, chunk) in tasks.chunks(batch_size).enumerate() {
            debug!(
                "Processing chunk {}/{} ({} tasks)", 
                chunk_index + 1, 
                (tasks.len() + batch_size - 1) / batch_size,
                chunk.len()
            );
            
            // Record pre-processing memory state
            let pre_mem = self.get_available_memory();
            
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
            
            // Record post-processing memory state and calculate metrics
            let post_mem = self.get_available_memory();
            let used_memory = pre_mem.saturating_sub(post_mem);
            memory_metrics.record_usage(used_memory, pre_mem);
            
            results.extend(chunk_results);
        }
        
        // Log memory metrics summary
        debug!(
            "Memory metrics - Avg: {}MB, Initial: {}MB, Peak Pressure: {}MB", 
            memory_metrics.avg_batch_memory / (1024 * 1024),
            memory_metrics.initial_memory / (1024 * 1024),
            memory_metrics.peak_pressure / (1024 * 1024)
        );
        
        Ok((results, memory_metrics))
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

    fn get_available_memory(&self) -> usize {
        let mut system = System::new();
        system.refresh_memory();
        system.available_memory() as usize
    }

    fn calculate_batch_size(&self, tasks: &[ImageTask]) -> usize {
        let config = BatchSizeConfig::default();
        
        // Calculate total and average task size
        let total_size: u64 = tasks.iter()
            .filter_map(|t| std::fs::metadata(&t.input_path).ok())
            .map(|m| m.len())
            .sum();
        let avg_size = total_size / tasks.len() as u64;
        
        // Get available system memory and calculate target
        let available_mem = self.get_available_memory();
        let memory_target = ((available_mem as f32 * config.target_memory_percentage) as usize)
            .min(config.target_memory_usage);
        
        // Calculate batch size based on average task size and memory target
        let memory_based_size = memory_target / avg_size as usize;
        
        // Also consider total tasks - don't make batches larger than needed
        let task_based_size = tasks.len();
        
        // Take the minimum of memory-based and task-based sizes
        let calculated_size = memory_based_size.min(task_based_size);
        
        // Clamp between min and max sizes
        let batch_size = calculated_size.clamp(config.min_size, config.max_size);
        
        debug!(
            "Batch size calculation: avg_size={}MB, memory_target={}MB, memory_based={}, task_based={}, final={}",
            avg_size / (1024 * 1024),
            memory_target / (1024 * 1024),
            memory_based_size,
            task_based_size,
            batch_size
        );
        
        batch_size
    }
} 