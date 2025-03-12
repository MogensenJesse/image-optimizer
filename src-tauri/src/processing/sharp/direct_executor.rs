use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandEvent, TerminatedPayload};
use crate::utils::{OptimizerError, OptimizerResult};
use crate::core::{ImageTask, OptimizationResult};
use crate::core::{Progress, ProgressType, ProgressReporter};
use super::types::{SharpResult, DetailedProgressUpdate};
#[cfg(feature = "benchmarking")]
use crate::benchmarking::metrics::WorkerPoolMetrics;
use tracing::{debug, warn};
use serde_json;
use serde::Deserialize;
use std::str::from_utf8;
use tauri::Emitter;

#[derive(Debug, Deserialize)]
struct BatchOutput {
    results: Vec<SharpResult>,
    #[cfg(feature = "benchmarking")]
    metrics: Option<WorkerPoolMetrics>,
}

/// Direct executor that spawns a Sharp sidecar process for each batch
/// without maintaining a pool of processes
pub struct DirectExecutor {
    app: AppHandle,
}

impl DirectExecutor {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    /// Warms up the executor by processing a minimal image task
    /// This helps reduce the cold start penalty for the first real task
    pub async fn warmup(&self) -> OptimizerResult<()> {
        debug!("Warming up DirectExecutor...");
        
        // Create a minimal task that will initialize the Sharp pipeline
        // but requires minimal processing time
        let dummy_task = ImageTask::create_warmup_task()?;
        
        // Execute the task but don't care about the result
        // Just need to initialize the Sharp sidecar
        let _ = self.execute_batch(&[dummy_task]).await?;
        
        debug!("DirectExecutor warmup completed successfully");
        Ok(())
    }

    /// Extract filename from a path
    fn extract_filename<'b>(&self, path: &'b str) -> &'b str {
        std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path)
    }

    /// Handles a progress message from the sidecar
    fn handle_progress(&self, message: super::types::ProgressMessage) {
        // Convert from the processing-specific type to the core progress type
        let mut progress = message.to_core_progress();
        
        // Add metadata for optimization statistics if a result is available
        if let Some(result) = &progress.result {
            let file_name = self.extract_filename(&result.path);
            let saved_kb = result.saved_bytes as f64 / 1024.0;
            
            let formatted_msg = format!(
                "{} optimized ({:.2} KB saved / {}% compression)",
                file_name,
                saved_kb,
                result.compression_ratio
            );
            
            let metadata = serde_json::json!({
                "formattedMessage": formatted_msg,
                "fileName": file_name,
                "originalSize": result.original_size,
                "optimizedSize": result.optimized_size,
                "savedBytes": result.saved_bytes,
                "compressionRatio": result.compression_ratio
            });
            
            progress.metadata = Some(metadata);
        }
        
        // Report progress using the trait
        self.report_progress(&progress);
    }

    /// Handles a simplified progress update from the sidecar
    fn handle_progress_update(&self, update: super::types::ProgressUpdate) {
        // Convert to core progress type
        let progress = update.to_core_progress();
        
        // Simplified updates already have metadata from the Sharp sidecar
        // Just pass them through to the frontend
        
        // Report progress
        self.report_progress(&progress);
    }

    /// Handles a detailed progress update from the sidecar
    fn handle_detailed_progress_update(&self, update: DetailedProgressUpdate) {
        // Create a progress object from the detailed update
        let progress_type = ProgressType::Complete;
        let completed_tasks = update.batch_metrics.completed_tasks;
        let total_tasks = update.batch_metrics.total_tasks;
        
        let mut progress = Progress::new(
            progress_type,
            completed_tasks,
            total_tasks,
            "complete"
        );
        
        // Set task ID
        progress.task_id = Some(update.task_id.clone());
        
        // Calculate saved bytes and retrieve other metrics
        let saved_bytes = update.optimization_metrics.saved_bytes;
        let compression_ratio = update.optimization_metrics.compression_ratio.clone();
        let file_name = self.extract_filename(&update.task_id);
        
        // Create a result object
        let result = SharpResult {
            path: update.task_id.clone(),
            original_size: update.optimization_metrics.original_size,
            optimized_size: update.optimization_metrics.optimized_size,
            saved_bytes: saved_bytes as i64,
            compression_ratio: compression_ratio.clone(),
            format: update.optimization_metrics.format.clone(),
            success: true,
            error: None,
        };
        
        progress.result = Some(result);
        
        // Create formatted message and metadata for the frontend
        let saved_kb = saved_bytes as f64 / 1024.0;
        let formatted_msg = format!(
            "{} optimized ({:.2} KB saved / {}% compression) - Progress: {}% ({}/{})",
            file_name,
            saved_kb,
            compression_ratio,
            update.batch_metrics.progress_percentage,
            update.batch_metrics.completed_tasks,
            update.batch_metrics.total_tasks
        );
        
        // Add detailed metadata for the frontend
        let metadata = serde_json::json!({
            "formattedMessage": formatted_msg,
            "fileName": file_name,
            "originalSize": update.optimization_metrics.original_size,
            "optimizedSize": update.optimization_metrics.optimized_size,
            "savedBytes": saved_bytes,
            "compressionRatio": compression_ratio
        });
        
        progress.metadata = Some(metadata);
        
        // Report progress
        self.report_progress(&progress);
    }

    #[cfg(feature = "benchmarking")]
    pub async fn execute_batch(&self, tasks: &[ImageTask]) 
        -> OptimizerResult<Vec<OptimizationResult>> {
        debug!("Processing batch of {} tasks", tasks.len());
        
        // Create sidecar command
        let cmd = self.app.shell()
            .sidecar("sharp-sidecar")
            .map_err(|e| OptimizerError::sidecar(format!("Sidecar spawn failed: {}", e)))?;
        
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
        
        // Run the command and capture output stream
        let (mut rx, _child) = cmd
            .args(&["optimize-batch", &batch_json])
            .spawn()
            .map_err(|e| OptimizerError::sidecar(format!("Failed to spawn Sharp process: {}", e)))?;

        let mut results = Vec::new();
        #[cfg(feature = "benchmarking")]
        let mut final_metrics = None;
        let mut _batch_json_buffer = String::new();
        let mut _capturing_batch_result = false;

        // Helper function to process output lines
        fn process_line(line: &[u8]) -> Option<String> {
            from_utf8(line).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        }

        // Process output events in real-time
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) | CommandEvent::Stderr(line) => {
                    if let Some(line_str) = process_line(&line) {
                        // Check for batch result markers
                        if line_str == "BATCH_RESULT_START" {
                            _capturing_batch_result = true;
                            _batch_json_buffer.clear();
                            continue;
                        } else if line_str == "BATCH_RESULT_END" {
                            _capturing_batch_result = false;
                            
                            // Parse the batch result JSON
                            if let Ok(batch_output) = serde_json::from_str::<BatchOutput>(&_batch_json_buffer) {
                                debug!("Received batch output from sidecar - results count: {}", batch_output.results.len());
                                
                                // Add the results to our output collection
                                for (task, result) in tasks.iter().zip(batch_output.results) {
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
                                
                                #[cfg(feature = "benchmarking")]
                                {
                                    final_metrics = batch_output.metrics;
                                }
                            }
                            continue;
                        }
                        
                        // If we're capturing batch result JSON, add to buffer
                        if _capturing_batch_result {
                            _batch_json_buffer.push_str(&line_str);
                            _batch_json_buffer.push('\n');
                            continue;
                        }
                        
                        // Try to parse as progress message
                        if let Ok(progress) = serde_json::from_str::<super::types::ProgressMessage>(&line_str) {
                            self.handle_progress(progress);
                        } else if let Ok(update) = serde_json::from_str::<super::types::ProgressUpdate>(&line_str) {
                            self.handle_progress_update(update);
                        } else if let Ok(detailed) = serde_json::from_str::<DetailedProgressUpdate>(&line_str) {
                            self.handle_detailed_progress_update(detailed);
                        } else {
                            // Try to parse as batch output (old format - kept for backward compatibility)
                            if let Ok(batch_output) = serde_json::from_str::<BatchOutput>(&line_str) {
                                // Process final batch output
                                debug!("Received batch output from sidecar (old format) - results count: {}", batch_output.results.len());
                                
                                // Add the results to our output collection without verbose logging
                                for (task, result) in tasks.iter().zip(batch_output.results) {
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
                                
                                // Store metrics without redundant logging
                                #[cfg(feature = "benchmarking")]
                                {
                                    final_metrics = batch_output.metrics;
                                }
                            }
                        }
                    }
                }
                CommandEvent::Error(err) => {
                    return Err(OptimizerError::sidecar(format!("Sharp process error: {}", err)));
                }
                CommandEvent::Terminated(TerminatedPayload { code, .. }) => {
                    if code.unwrap_or(-1) != 0 {
                        return Err(OptimizerError::sidecar(format!("Sharp process failed with status: {:?}", code)));
                    }
                    break;
                }
                _ => {} // Handle any future CommandEvent variants
            }
        }

        // Log worker metrics once at the end with useful information
        #[cfg(feature = "benchmarking")]
        {
            if let Some(metrics) = &final_metrics {
                debug!("Worker metrics summary: {} workers with avg {:.1} tasks/worker",
                    metrics.worker_count,
                    metrics.tasks_per_worker.iter().sum::<usize>() as f64 / metrics.worker_count as f64
                );
            }
        }
        
        Ok(results)
    }
    
    #[cfg(not(feature = "benchmarking"))]
    pub async fn execute_batch(&self, tasks: &[ImageTask]) 
        -> OptimizerResult<Vec<OptimizationResult>> {
        debug!("Processing batch of {} tasks", tasks.len());
        
        // Create sidecar command
        debug!("Creating sidecar command for batch processing");
        let cmd = self.app.shell()
            .sidecar("sharp-sidecar")
            .map_err(|e| OptimizerError::sidecar(format!("Sidecar spawn failed: {}", e)))?;
        
        // Create batch task data
        debug!("Serializing batch task data");
        let batch_data = tasks.iter().map(|task| {
            serde_json::json!({
                "input": task.input_path,
                "output": task.output_path,
                "settings": task.settings
            })
        }).collect::<Vec<_>>();

        let batch_json = serde_json::to_string(&batch_data)
            .map_err(|e| OptimizerError::processing(format!("Failed to serialize batch settings: {}", e)))?;
        
        // Run the command and capture output stream
        debug!("Spawning Sharp sidecar process for batch optimization");
        let (mut rx, _child) = cmd
            .args(&["optimize-batch", &batch_json])
            .spawn()
            .map_err(|e| OptimizerError::sidecar(format!("Failed to spawn Sharp process: {}", e)))?;

        debug!("Sidecar process started, waiting for results");
        let mut results = Vec::new();
        let mut _batch_json_buffer = String::new();
        let mut _capturing_batch_result = false;

        // Helper function to process output lines
        fn process_line(line: &[u8]) -> Option<String> {
            from_utf8(line).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        }

        // Process output events in real-time
        debug!("Starting to process output events from sidecar");
        let mut line_count = 0;
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) | CommandEvent::Stderr(line) => {
                    line_count += 1;
                    if line_count % 50 == 0 {
                        debug!("Processed {} lines of output from sidecar", line_count);
                    }
                    
                    if let Some(line_str) = process_line(&line) {
                        // Check for batch result markers
                        if line_str == "BATCH_RESULT_START" {
                            debug!("Received BATCH_RESULT_START marker");
                            _capturing_batch_result = true;
                            _batch_json_buffer.clear();
                            continue;
                        } else if line_str == "BATCH_RESULT_END" {
                            debug!("Received BATCH_RESULT_END marker");
                            _capturing_batch_result = false;
                            
                            // Parse the batch result JSON
                            debug!("Parsing batch result JSON (buffer size: {} bytes)", _batch_json_buffer.len());
                            if let Ok(batch_output) = serde_json::from_str::<BatchOutput>(&_batch_json_buffer) {
                                debug!("Received batch output from sidecar - results count: {}", batch_output.results.len());
                                
                                // Add the results to our output collection
                                for (task, result) in tasks.iter().zip(batch_output.results) {
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
                            } else {
                                warn!("Failed to parse batch result JSON");
                            }
                            continue;
                        }
                        
                        // If we're capturing batch result JSON, add to buffer
                        if _capturing_batch_result {
                            _batch_json_buffer.push_str(&line_str);
                            _batch_json_buffer.push('\n');
                            continue;
                        }
                        
                        // Try to parse as progress message
                        if let Ok(progress) = serde_json::from_str::<super::types::ProgressMessage>(&line_str) {
                            self.handle_progress(progress);
                        } else if let Ok(update) = serde_json::from_str::<super::types::ProgressUpdate>(&line_str) {
                            self.handle_progress_update(update);
                        } else if let Ok(detailed) = serde_json::from_str::<DetailedProgressUpdate>(&line_str) {
                            self.handle_detailed_progress_update(detailed);
                        } else {
                            // Try to parse as batch output (old format - kept for backward compatibility)
                            if let Ok(batch_output) = serde_json::from_str::<BatchOutput>(&line_str) {
                                // Process final batch output
                                debug!("Received batch output from sidecar (old format) - results count: {}", batch_output.results.len());
                                
                                // Add the results to our output collection without verbose logging
                                for (task, result) in tasks.iter().zip(batch_output.results) {
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
                            }
                        }
                    }
                }
                CommandEvent::Error(err) => {
                    warn!("Sharp sidecar process error: {}", err);
                    return Err(OptimizerError::sidecar(format!("Sharp process error: {}", err)));
                }
                CommandEvent::Terminated(TerminatedPayload { code, .. }) => {
                    debug!("Sharp sidecar process terminated with code: {:?}", code);
                    if code.unwrap_or(-1) != 0 {
                        return Err(OptimizerError::sidecar(format!("Sharp process failed with status: {:?}", code)));
                    }
                    break;
                }
                _ => {} // Handle any future CommandEvent variants
            }
        }
        
        debug!("Batch processing completed, returning {} results", results.len());
        Ok(results)
    }
}

impl ProgressReporter for DirectExecutor {
    fn report_progress(&self, progress: &Progress) {
        // Create the progress update for the frontend
        let progress_update = progress.to_progress_update();
        
        // Log formatted message if available in metadata
        if let Some(metadata) = &progress.metadata {
            // Log formatted message if available
            if let Some(msg) = metadata.get("formattedMessage") {
                if let Some(msg_str) = msg.as_str() {
                    debug!("{}", msg_str);
                }
            }
        }
        
        match progress.progress_type {
            ProgressType::Start => {
                // Emit event without logging
                let _ = self.app.emit("image_optimization_progress", progress_update);
            }
            ProgressType::Error => {
                warn!("Optimization error: {}", progress.status);
                let _ = self.app.emit("image_optimization_progress", progress_update);
            }
            _ => {
                // Emit event without logging progress status
                let _ = self.app.emit("image_optimization_progress", progress_update);
            }
        }
    }
} 