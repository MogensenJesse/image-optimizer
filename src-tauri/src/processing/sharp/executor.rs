use crate::processing::pool::ProcessPool;
use crate::worker::ImageTask;
use crate::utils::{OptimizerError, OptimizerResult};
use crate::core::OptimizationResult;
use super::types::SharpResult;
use tauri_plugin_shell::process::Output;
use tracing::debug;
use serde_json;

pub struct SharpExecutor<'a> {
    pool: &'a ProcessPool,
}

impl<'a> SharpExecutor<'a> {
    pub fn new(pool: &'a ProcessPool) -> Self {
        Self { pool }
    }

    pub async fn execute_batch(&self, tasks: &[ImageTask]) 
        -> OptimizerResult<Vec<OptimizationResult>> {
        debug!("Processing batch with Sharp sidecar - {} tasks", tasks.len());
        let (sharp_result, output) = self.run_sharp_process(&tasks).await?;

        match sharp_result {
            Ok(_) => {
                debug!("Collecting results for {} tasks", tasks.len());
                // Parse Sharp output to get results
                let stdout = String::from_utf8_lossy(&output.stdout);
                let sharp_results: Vec<SharpResult> = serde_json::from_str(&stdout)
                    .map_err(|e| OptimizerError::processing(format!(
                        "Failed to parse Sharp output: {}", e
                    )))?;

                // Collect results for each task
                let mut results = Vec::with_capacity(tasks.len());
                let mut total_original = 0;
                let mut total_optimized = 0;
                
                for (task, result) in tasks.iter().zip(sharp_results) {
                    total_original += result.original_size;
                    total_optimized += result.optimized_size;

                    results.push(OptimizationResult {
                        original_path: task.input_path.clone(),
                        optimized_path: result.path,
                        original_size: result.original_size,
                        optimized_size: result.optimized_size,
                        success: result.success,
                        error: result.error,
                        saved_bytes: result.saved_bytes,
                        compression_ratio: result.compression_ratio.parse().unwrap_or(0.0),
                    });
                }
                
                let total_saved_percentage = ((total_original - total_optimized) as f64 / total_original as f64 * 100.0).round();
                debug!(
                    "Batch completed - Tasks: {}, Total Original: {}, Total Optimized: {}, Average Savings: {}%",
                    results.len(),
                    total_original,
                    total_optimized,
                    total_saved_percentage
                );
                Ok(results)
            }
            Err(e) => Err(e),
        }
    }

    async fn run_sharp_process(&self, tasks: &[ImageTask]) 
        -> OptimizerResult<(OptimizerResult<()>, Output)> {
        // Acquire a process from the pool
        let cmd = self.pool.acquire().await?;
        
        // Create batch task data
        let batch_data = tasks.iter().map(|task| {
            serde_json::json!({
                "input": task.input_path,
                "output": task.output_path,
                "settings": task.settings
            })
        }).collect::<Vec<_>>();

        debug!("Prepared {} tasks for Sharp processing", tasks.len());

        let batch_json = serde_json::to_string(&batch_data)
            .map_err(|e| OptimizerError::processing(format!("Failed to serialize batch settings: {}", e)))?;
        
        // Run the command
        let output = cmd
            .args(&["optimize-batch", &batch_json])
            .output()
            .await
            .map_err(|e| OptimizerError::sidecar(format!("Failed to run Sharp: {}", e)))?;
        
        // Release the process back to the pool
        self.pool.release().await;
        
        // Check for success and validate output
        if output.status.success() {
            debug!("Sharp sidecar command completed successfully");
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            if !stdout.is_empty() {
                // Parse Sharp output JSON
                let sharp_results: Vec<serde_json::Value> = serde_json::from_str(&stdout)
                    .map_err(|e| OptimizerError::processing(format!(
                        "Failed to parse Sharp output: {}", e
                    )))?;
                
                debug!("Sharp sidecar processed {} results", sharp_results.len());
                
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
                        debug!("{}", error_message);
                        return Ok((Err(OptimizerError::processing(error_message)), output));
                    }
                    
                    // Verify the output file exists
                    match tokio::fs::metadata(output_path).await {
                        Ok(_) => {},
                        Err(e) => {
                            let error_message = format!(
                                "Output file missing: {} (Error: {})", output_path, e
                            );
                            debug!("{}", error_message);
                            return Ok((Err(OptimizerError::processing(error_message)), output));
                        }
                    }
                }
                
                Ok((Ok(()), output))
            } else {
                let error_message = "Empty output from Sharp sidecar".to_string();
                debug!("{}", error_message);
                Ok((Err(OptimizerError::processing(error_message)), output))
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let error_message = format!("Sharp process failed: {}", stderr);
            debug!("{}", error_message);
            Ok((Err(OptimizerError::sidecar(error_message)), output))
        }
    }
} 