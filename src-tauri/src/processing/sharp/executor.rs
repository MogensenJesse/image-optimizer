use crate::processing::pool::ProcessPool;
use crate::core::ImageTask;
use crate::utils::{OptimizerError, OptimizerResult};
use crate::core::OptimizationResult;
use super::types::{SharpResult, ProgressMessage, ProgressType, ProgressUpdate};
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
    fn handle_progress(&self, message: ProgressMessage) {
        let filename = self.extract_filename(&message.task_id);
        
        match message.progress_type {
            ProgressType::Start => {
                debug!(
                    "🔄 Worker {} started processing {}",
                    message.worker_id,
                    filename
                );
            }
            ProgressType::Complete => {
                if let Some(result) = message.result {
                    debug!(
                        "✅ Worker {} completed {} - Saved {} bytes ({}% reduction)",
                        message.worker_id,
                        filename,
                        result.saved_bytes,
                        result.compression_ratio
                    );
                }
            }
            ProgressType::Error => {
                // Keep errors at warn level for visibility
                warn!(
                    "❌ Worker {} failed processing {} - {}",
                    message.worker_id,
                    filename,
                    message.error.unwrap_or_else(|| "Unknown error".to_string())
                );
            }
            _ => {}
        }

        // Only log metrics at debug level
        if let Some(metrics) = message.metrics {
            debug!(
                "📊 Progress: {}/{} tasks completed ({} in queue)",
                metrics.completed_tasks,
                metrics.total_tasks,
                metrics.queue_length
            );
        }
    }

    /// Handles a simplified progress update from the sidecar
    fn handle_progress_update(&self, update: ProgressUpdate) {
        // Always log at debug level
        debug!(
            "Progress: {}% ({}/{} tasks)",
            update.progress_percentage,
            update.completed_tasks,
            update.total_tasks
        );
        
        // Only log at info level for milestone percentages in benchmark mode
        if update.progress_percentage % 25 == 0 || update.progress_percentage == 100 {
            // Check if we're in benchmark mode
            #[cfg(feature = "benchmark")]
            info!(
                "📊 Progress: {}% ({}/{} tasks)",
                update.progress_percentage,
                update.completed_tasks,
                update.total_tasks
            );
        }
        
        // Emit event for frontend progress bar
        if let Some(app) = self.pool.get_app() {
            // Clone the update to avoid borrowing issues
            let update_clone = update.clone();
            
            // Emit the progress event to the frontend
            let _ = app.emit("image_optimization_progress", update_clone);
        }
    }

    pub async fn execute_batch(&self, tasks: &[ImageTask]) 
        -> OptimizerResult<(Vec<OptimizationResult>, Option<WorkerPoolMetrics>)> {
        debug!("Starting batch processing");
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
                        // Try to parse as progress update first (new simplified format)
                        if let Ok(progress_update) = serde_json::from_str::<ProgressUpdate>(&line_str) {
                            debug!("Successfully parsed progress update: {}%", progress_update.progress_percentage);
                            self.handle_progress_update(progress_update);
                        } else {
                            // Try to parse as progress message (detailed format)
                            match serde_json::from_str::<ProgressMessage>(&line_str) {
                                Ok(progress) => {
                                    debug!("Successfully parsed progress message: {:?}", progress.progress_type);
                                    self.handle_progress(progress);
                                }
                                Err(_) => {
                                    // Try to parse as batch output
                                    if let Ok(batch_output) = serde_json::from_str::<BatchOutput>(&line_str) {
                                        debug!("Successfully parsed batch output");
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
                                        final_metrics = batch_output.metrics;
                                    }
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

        // Log final worker metrics
        if let Some(metrics) = &final_metrics {
            debug!(
                "Worker metrics - Workers: {}, Active: {}, Tasks per worker: {:?}",
                metrics.worker_count,
                metrics.active_workers,
                metrics.tasks_per_worker
            );
        }
        
        Ok((results, final_metrics))
    }
} 