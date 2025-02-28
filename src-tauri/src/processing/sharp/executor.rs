use crate::processing::pool::ProcessPool;
use crate::core::ImageTask;
use crate::utils::{OptimizerError, OptimizerResult};
use crate::core::OptimizationResult;
use crate::core::{Progress, ProgressType, ProgressReporter};
use super::types::SharpResult;
#[cfg(feature = "benchmarking")]
use crate::benchmarking::metrics::WorkerPoolMetrics;
use tauri_plugin_shell::process::{CommandEvent, TerminatedPayload};
use tracing::{debug, info, warn};
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

pub struct SharpExecutor<'a> {
    pool: &'a ProcessPool,
}

impl<'a> SharpExecutor<'a> {
    pub fn new(pool: &'a ProcessPool) -> Self {
        Self { pool }
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
        let progress = message.to_core_progress();
        
        // Report progress using the trait
        self.report_progress(&progress);
    }

    /// Handles a simplified progress update from the sidecar
    fn handle_progress_update(&self, update: super::types::ProgressUpdate) {
        // Convert from the processing-specific type to the core progress type
        let progress = update.to_core_progress();
        
        // Report progress using the trait
        self.report_progress(&progress);
    }

    #[cfg(feature = "benchmarking")]
    pub async fn execute_batch(&self, tasks: &[ImageTask]) 
        -> OptimizerResult<(Vec<OptimizationResult>, Option<WorkerPoolMetrics>)> {
        // Single log entry for batch processing start - use INFO level
        info!("Processing batch of {} tasks", tasks.len());
        
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

        let batch_json = serde_json::to_string(&batch_data)
            .map_err(|e| OptimizerError::processing(format!("Failed to serialize batch settings: {}", e)))?;
        
        // Run the command and capture output stream
        let (mut rx, _child) = cmd
            .args(&["optimize-batch", &batch_json])
            .spawn()
            .map_err(|e| OptimizerError::sidecar(format!("Failed to spawn Sharp process: {}", e)))?;

        let mut results = Vec::new();
        let mut final_metrics = None;

        // Helper function to process output lines
        fn process_line(line: &[u8]) -> Option<String> {
            from_utf8(line).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        }

        // Process output events in real-time
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) | CommandEvent::Stderr(line) => {
                    if let Some(line_str) = process_line(&line) {
                        if line_str.contains("\"progressType\"") || line_str.contains("\"status\"") {
                            // Try to parse as Progress type from core module first
                            if let Ok(progress) = serde_json::from_str::<crate::core::Progress>(&line_str) {
                                self.report_progress(&progress);
                            } 
                            // Try to parse as progress update (simplified format)
                            else if let Ok(update) = serde_json::from_str::<super::types::ProgressUpdate>(&line_str) {
                                self.handle_progress_update(update);
                            } 
                            // Try to parse as legacy progress message
                            else if let Ok(message) = serde_json::from_str::<super::types::ProgressMessage>(&line_str) {
                                self.handle_progress(message);
                            }
                        } else {
                            // Try to parse as batch output
                            if let Ok(batch_output) = serde_json::from_str::<BatchOutput>(&line_str) {
                                // Process final batch output
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
                                final_metrics = batch_output.metrics;
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
        
        // Release the process back to the pool
        self.pool.release().await;

        // Log worker metrics once at the end with useful information
        if let Some(metrics) = &final_metrics {
            debug!("Worker metrics summary: {} workers with avg {:.1} tasks/worker",
                metrics.worker_count,
                metrics.tasks_per_worker.iter().sum::<usize>() as f64 / metrics.worker_count as f64
            );
        }
        
        Ok((results, final_metrics))
    }
    
    #[cfg(not(feature = "benchmarking"))]
    pub async fn execute_batch(&self, tasks: &[ImageTask]) 
        -> OptimizerResult<(Vec<OptimizationResult>, Option<()>)> {
        // Single log entry for batch processing
        info!("Processing batch of {} tasks", tasks.len());
        
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

        let batch_json = serde_json::to_string(&batch_data)
            .map_err(|e| OptimizerError::processing(format!("Failed to serialize batch settings: {}", e)))?;
        
        // Run the command and capture output stream
        let (mut rx, _child) = cmd
            .args(&["optimize-batch", &batch_json])
            .spawn()
            .map_err(|e| OptimizerError::sidecar(format!("Failed to spawn Sharp process: {}", e)))?;

        let mut results = Vec::new();

        // Helper function to process output lines
        fn process_line(line: &[u8]) -> Option<String> {
            from_utf8(line).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        }

        // Process output events in real-time
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) | CommandEvent::Stderr(line) => {
                    if let Some(line_str) = process_line(&line) {
                        if line_str.contains("\"progressType\"") || line_str.contains("\"status\"") {
                            // Try to parse as Progress type from core module first
                            if let Ok(progress) = serde_json::from_str::<crate::core::Progress>(&line_str) {
                                self.report_progress(&progress);
                            } 
                            // Try to parse as progress update (simplified format)
                            else if let Ok(update) = serde_json::from_str::<super::types::ProgressUpdate>(&line_str) {
                                self.handle_progress_update(update);
                            } 
                            // Try to parse as legacy progress message
                            else if let Ok(message) = serde_json::from_str::<super::types::ProgressMessage>(&line_str) {
                                self.handle_progress(message);
                            }
                        } else {
                            // Try to parse as batch output
                            if let Ok(batch_output) = serde_json::from_str::<BatchOutput>(&line_str) {
                                // Process final batch output
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
        
        // Release the process back to the pool
        self.pool.release().await;
        
        Ok((results, None))
    }
}

impl<'a> ProgressReporter for SharpExecutor<'a> {
    fn report_progress(&self, progress: &Progress) {
        // Only log certain progress events to reduce verbosity
        match progress.progress_type {
            ProgressType::Start => {
                // For worker start events, only log at trace level to reduce noise
                #[cfg(debug_assertions)]
                if let (Some(task_id), Some(worker_id)) = (&progress.task_id, progress.worker_id) {
                    let filename = self.extract_filename(task_id);
                    debug!("🔄 Worker {} started processing {}", worker_id, filename);
                }
            }
            ProgressType::Complete => {
                if let Some(result) = &progress.result {
                    // Only log completions with significant compression or at lower frequency
                    match (progress.task_id.as_ref(), progress.worker_id) {
                        (Some(task_id), _) => {
                            // Calculate if we should log this item (based on compression or sampling)
                            let significant_compression = result.saved_bytes > 100_000; // 100KB savings
                            let sample_log = progress.completed_tasks % 10 == 0; // Log every 10th item
                            
                            if significant_compression || sample_log {
                                let filename = self.extract_filename(task_id);
                                if let Some(worker_id) = progress.worker_id {
                                    debug!(
                                        "✅ Worker {} completed {} - Saved {} bytes ({}% reduction)",
                                        worker_id,
                                        filename,
                                        result.saved_bytes,
                                        result.compression_ratio
                                    );
                                } else {
                                    debug!(
                                        "✅ Completed {} - Saved {} bytes ({}% reduction)",
                                        filename,
                                        result.saved_bytes,
                                        result.compression_ratio
                                    );
                                }
                            }
                        }
                        _ => {}
                    };
                }
            }
            ProgressType::Error => {
                // Always log errors at warning level
                if let Some(task_id) = &progress.task_id {
                    let filename = self.extract_filename(task_id);
                    
                    if let Some(error) = &progress.error {
                        warn!("❌ Error processing {}: {}", filename, error);
                    } else {
                        warn!("❌ Unknown error processing {}", filename);
                    }
                }
            }
            ProgressType::Progress => {
                // Log progress updates at regular intervals (every 10%)
                if progress.progress_percentage % 10 == 0 || progress.progress_percentage == 25 || 
                   progress.progress_percentage == 50 || progress.progress_percentage == 75 {
                    // Use INFO level for progress to make it more visible
                    info!(
                        "📊 Progress: {}% ({}/{})",
                        progress.progress_percentage,
                        progress.completed_tasks,
                        progress.total_tasks
                    );
                } else {
                    // Other progress updates at debug level
                    debug!(
                        "📊 Progress: {}% ({}/{})",
                        progress.progress_percentage,
                        progress.completed_tasks,
                        progress.total_tasks
                    );
                }
            }
        }
        
        // Emit event for frontend progress bar
        if let Some(app) = self.pool.get_app() {
            // Convert to ProgressUpdate for frontend compatibility
            let update = progress.to_progress_update();
            
            // Emit the progress event to the frontend
            let _ = app.emit("image_optimization_progress", update);
        }
    }
} 