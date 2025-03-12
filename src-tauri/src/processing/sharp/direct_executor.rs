use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandEvent, TerminatedPayload};
use crate::utils::{OptimizerError, OptimizerResult};
use crate::core::{ImageTask, OptimizationResult};
use crate::core::{Progress, ProgressReporter};
use super::types::{SharpResult, DetailedProgressUpdate};
use super::progress_handler::ProgressHandler;
#[cfg(feature = "benchmarking")]
use crate::benchmarking::metrics::WorkerPoolMetrics;
use tracing::{debug, warn};
use serde_json;
use serde::Deserialize;
use std::str::from_utf8;
use tauri::async_runtime::Receiver;

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
    progress_handler: ProgressHandler,
}

impl DirectExecutor {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app: app.clone(),
            progress_handler: ProgressHandler::new(app),
        }
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
    
    /// Prepares the batch data for processing
    fn prepare_batch_data(&self, tasks: &[ImageTask]) -> OptimizerResult<String> {
        // Create batch task data
        let batch_data = tasks.iter().map(|task| {
            serde_json::json!({
                "input": task.input_path,
                "output": task.output_path,
                "settings": task.settings
            })
        }).collect::<Vec<_>>();

        serde_json::to_string(&batch_data)
            .map_err(|e| OptimizerError::processing(format!("Failed to serialize batch settings: {}", e)))
    }
    
    /// Creates the sidecar command for execution
    fn create_sidecar_command(&self) -> OptimizerResult<tauri_plugin_shell::process::Command> {
        self.app.shell()
            .sidecar("sharp-sidecar")
            .map_err(|e| OptimizerError::sidecar(format!("Sidecar spawn failed: {}", e)))
    }
    
    /// Converts SharpResults to OptimizationResults
    fn convert_to_optimization_results(
        &self, 
        tasks: &[ImageTask], 
        results: Vec<SharpResult>
    ) -> Vec<OptimizationResult> {
        tasks.iter()
            .zip(results)
            .map(|(task, result)| OptimizationResult {
                original_path: task.input_path.clone(),
                optimized_path: result.path,
                original_size: result.original_size,
                optimized_size: result.optimized_size,
                success: result.success,
                error: result.error,
                saved_bytes: result.saved_bytes,
                compression_ratio: result.compression_ratio.parse().unwrap_or(0.0),
            })
            .collect()
    }
    
    /// Process a line of output from the sidecar
    #[cfg(feature = "benchmarking")]
    fn process_output_line(
        &self, 
        line_str: &str, 
        batch_json_buffer: &mut String,
        capturing_batch_result: bool,
        tasks: &[ImageTask],
        results: &mut Vec<OptimizationResult>,
        final_metrics: &mut Option<WorkerPoolMetrics>,
    ) -> bool {
        // Return value indicates if we're capturing batch result
        let mut is_capturing = capturing_batch_result;
        
        // Check for batch result markers
        if line_str == "BATCH_RESULT_START" {
            debug!("Received BATCH_RESULT_START marker");
            is_capturing = true;
            batch_json_buffer.clear();
        } else if line_str == "BATCH_RESULT_END" {
            debug!("Received BATCH_RESULT_END marker");
            is_capturing = false;
            
            // Parse the batch result JSON
            debug!("Parsing batch result JSON (buffer size: {} bytes)", batch_json_buffer.len());
            if let Ok(batch_output) = serde_json::from_str::<BatchOutput>(&batch_json_buffer) {
                debug!("Received batch output from sidecar - results count: {}", batch_output.results.len());
                
                // Convert results
                let optimization_results = self.convert_to_optimization_results(tasks, batch_output.results);
                results.extend(optimization_results);
                
                // Store metrics in benchmark mode
                *final_metrics = batch_output.metrics;
            } else {
                warn!("Failed to parse batch result JSON");
            }
        } else if is_capturing {
            // If we're capturing batch result JSON, add to buffer
            batch_json_buffer.push_str(line_str);
            batch_json_buffer.push('\n');
        } else {
            // Try to parse as various progress message types
            if let Ok(progress) = serde_json::from_str::<super::types::ProgressMessage>(line_str) {
                self.progress_handler.handle_progress(progress);
            } else if let Ok(update) = serde_json::from_str::<super::types::ProgressUpdate>(line_str) {
                self.progress_handler.handle_progress_update(update);
            } else if let Ok(detailed) = serde_json::from_str::<DetailedProgressUpdate>(line_str) {
                self.progress_handler.handle_detailed_progress_update(detailed);
            } else {
                // Try to parse as batch output (old format - kept for backward compatibility)
                if let Ok(batch_output) = serde_json::from_str::<BatchOutput>(line_str) {
                    debug!("Received batch output from sidecar (old format) - results count: {}", batch_output.results.len());
                    
                    // Convert and add results
                    let optimization_results = self.convert_to_optimization_results(tasks, batch_output.results);
                    results.extend(optimization_results);
                    
                    // Store metrics in benchmark mode
                    *final_metrics = batch_output.metrics;
                }
            }
        }
        
        is_capturing
    }
    
    /// Process a line of output from the sidecar (non-benchmarking version)
    #[cfg(not(feature = "benchmarking"))]
    fn process_output_line(
        &self, 
        line_str: &str, 
        batch_json_buffer: &mut String,
        capturing_batch_result: bool,
        tasks: &[ImageTask],
        results: &mut Vec<OptimizationResult>,
    ) -> bool {
        // Return value indicates if we're capturing batch result
        let mut is_capturing = capturing_batch_result;
        
        // Check for batch result markers
        if line_str == "BATCH_RESULT_START" {
            debug!("Received BATCH_RESULT_START marker");
            is_capturing = true;
            batch_json_buffer.clear();
        } else if line_str == "BATCH_RESULT_END" {
            debug!("Received BATCH_RESULT_END marker");
            is_capturing = false;
            
            // Parse the batch result JSON
            debug!("Parsing batch result JSON (buffer size: {} bytes)", batch_json_buffer.len());
            if let Ok(batch_output) = serde_json::from_str::<BatchOutput>(&batch_json_buffer) {
                debug!("Received batch output from sidecar - results count: {}", batch_output.results.len());
                
                // Convert results
                let optimization_results = self.convert_to_optimization_results(tasks, batch_output.results);
                results.extend(optimization_results);
            } else {
                warn!("Failed to parse batch result JSON");
            }
        } else if is_capturing {
            // If we're capturing batch result JSON, add to buffer
            batch_json_buffer.push_str(line_str);
            batch_json_buffer.push('\n');
        } else {
            // Try to parse as various progress message types
            if let Ok(progress) = serde_json::from_str::<super::types::ProgressMessage>(line_str) {
                self.progress_handler.handle_progress(progress);
            } else if let Ok(update) = serde_json::from_str::<super::types::ProgressUpdate>(line_str) {
                self.progress_handler.handle_progress_update(update);
            } else if let Ok(detailed) = serde_json::from_str::<DetailedProgressUpdate>(line_str) {
                self.progress_handler.handle_detailed_progress_update(detailed);
            } else {
                // Try to parse as batch output (old format - kept for backward compatibility)
                if let Ok(batch_output) = serde_json::from_str::<BatchOutput>(line_str) {
                    debug!("Received batch output from sidecar (old format) - results count: {}", batch_output.results.len());
                    
                    // Convert and add results
                    let optimization_results = self.convert_to_optimization_results(tasks, batch_output.results);
                    results.extend(optimization_results);
                }
            }
        }
        
        is_capturing
    }
    
    /// Helper function to process output lines
    fn process_line(line: &[u8]) -> Option<String> {
        from_utf8(line).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    }
    
    /// Handles command events from the sidecar process
    async fn handle_sidecar_events(
        &self,
        tasks: &[ImageTask],
        mut rx: Receiver<CommandEvent>,
    ) -> OptimizerResult<Vec<OptimizationResult>> {
        let mut results = Vec::new();
        #[cfg(feature = "benchmarking")]
        let mut final_metrics = None;
        let mut batch_json_buffer = String::new();
        let mut capturing_batch_result = false;
        
        // Process output events in real-time
        debug!("Starting to process output events from sidecar");
        
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) | CommandEvent::Stderr(line) => {
                    if let Some(line_str) = Self::process_line(&line) {
                        #[cfg(feature = "benchmarking")]
                        {
                            capturing_batch_result = self.process_output_line(
                                &line_str,
                                &mut batch_json_buffer,
                                capturing_batch_result,
                                tasks,
                                &mut results,
                                &mut final_metrics,
                            );
                        }
                        
                        #[cfg(not(feature = "benchmarking"))]
                        {
                            capturing_batch_result = self.process_output_line(
                                &line_str,
                                &mut batch_json_buffer,
                                capturing_batch_result,
                                tasks,
                                &mut results,
                            );
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
        
        #[cfg(feature = "benchmarking")]
        {
            // Log worker metrics once at the end with useful information
            if let Some(metrics) = &final_metrics {
                debug!("Worker metrics summary: {} workers with avg {:.1} tasks/worker",
                    metrics.worker_count,
                    metrics.tasks_per_worker.iter().sum::<usize>() as f64 / metrics.worker_count as f64
                );
            }
        }
        
        debug!("Batch processing completed, returning {} results", results.len());
        Ok(results)
    }

    #[cfg(feature = "benchmarking")]
    pub async fn execute_batch(&self, tasks: &[ImageTask]) 
        -> OptimizerResult<Vec<OptimizationResult>> {
        debug!("Processing batch of {} tasks", tasks.len());
        
        // Create sidecar command
        let cmd = self.create_sidecar_command()?;
        
        // Prepare batch data
        let batch_json = self.prepare_batch_data(tasks)?;
        
        // Run the command and capture output stream
        let (rx, _child) = cmd
            .args(&["optimize-batch", &batch_json])
            .spawn()
            .map_err(|e| OptimizerError::sidecar(format!("Failed to spawn Sharp process: {}", e)))?;

        // Handle sidecar events and return results
        self.handle_sidecar_events(tasks, rx).await
    }
    
    #[cfg(not(feature = "benchmarking"))]
    pub async fn execute_batch(&self, tasks: &[ImageTask]) 
        -> OptimizerResult<Vec<OptimizationResult>> {
        debug!("Processing batch of {} tasks", tasks.len());
        
        // Create sidecar command
        debug!("Creating sidecar command for batch processing");
        let cmd = self.create_sidecar_command()?;
        
        // Prepare batch data
        debug!("Serializing batch task data");
        let batch_json = self.prepare_batch_data(tasks)?;
        
        // Run the command and capture output stream
        debug!("Spawning Sharp sidecar process for batch optimization");
        let (rx, _child) = cmd
            .args(&["optimize-batch", &batch_json])
            .spawn()
            .map_err(|e| OptimizerError::sidecar(format!("Failed to spawn Sharp process: {}", e)))?;

        debug!("Sidecar process started, waiting for results");
        
        // Handle sidecar events and return results
        self.handle_sidecar_events(tasks, rx).await
    }
}

impl ProgressReporter for DirectExecutor {
    fn report_progress(&self, progress: &Progress) {
        self.progress_handler.report_progress(progress);
    }
} 