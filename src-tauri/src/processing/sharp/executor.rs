use crate::processing::pool::ProcessPool;
use crate::core::ImageTask;
use crate::utils::{OptimizerError, OptimizerResult};
use crate::core::OptimizationResult;
use crate::core::{Progress, ProgressType, ProgressReporter};
use super::types::{SharpResult, DetailedProgressUpdate};
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

    /// Handles a detailed progress update with file-specific optimization metrics
    fn handle_detailed_progress_update(&self, update: DetailedProgressUpdate) {
        // Create a progress object with the file-specific optimization data
        let mut progress = Progress::new(
            ProgressType::Progress,
            update.batch_metrics.completed_tasks,
            update.batch_metrics.total_tasks,
            "processing"
        );
        
        // Set the task ID (clone it to avoid borrowing issues)
        let task_id = update.task_id.clone();
        progress.task_id = Some(task_id.clone());
        
        // Convert optimization metrics to SharpResult
        let result = SharpResult {
            path: task_id, // Use the cloned task_id
            original_size: update.optimization_metrics.original_size,
            optimized_size: update.optimization_metrics.optimized_size,
            saved_bytes: (update.optimization_metrics.original_size as i64) - (update.optimization_metrics.optimized_size as i64),
            compression_ratio: update.optimization_metrics.compression_ratio.clone(),
            format: update.optimization_metrics.format.clone(),
            success: true,
            error: None,
        };
        
        // Copy the necessary values before moving result
        let saved_bytes = result.saved_bytes;
        let compression_ratio = result.compression_ratio.clone();
        
        // Set the result object in the progress
        progress.result = Some(result);
        
        // Add the formatted message if available
        if let Some(msg) = update.formatted_message {
            let metadata = serde_json::json!({
                "formattedMessage": msg,
                "fileName": update.file_name
            });
            progress.metadata = Some(metadata);
        } else {
            // Create a default formatted message using the copied values
            let saved_kb = saved_bytes as f64 / 1024.0;
            let formatted_msg = format!(
                "{} optimized ({:.2} KB saved / {}% compression) - Progress: {}% ({}/{})",
                update.file_name,
                saved_kb,
                compression_ratio,
                update.batch_metrics.progress_percentage,
                update.batch_metrics.completed_tasks,
                update.batch_metrics.total_tasks
            );
            
            let metadata = serde_json::json!({
                "formattedMessage": formatted_msg,
                "fileName": update.file_name
            });
            progress.metadata = Some(metadata);
        }
        
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
                            // Process the complete batch JSON
                            if !_batch_json_buffer.is_empty() {
                                debug!("Processing complete batch result JSON");
                                if let Ok(batch_output) = serde_json::from_str::<BatchOutput>(&_batch_json_buffer) {
                                    // Process final batch output
                                    debug!("Received batch output from sidecar - results count: {}", batch_output.results.len());
                                    
                                    // Log a summary instead of each individual result
                                    if !batch_output.results.is_empty() {
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
                                    
                                    // Store metrics without redundant logging
                                    #[cfg(feature = "benchmarking")]
                                    {
                                        final_metrics = batch_output.metrics;
                                    }
                                } else {
                                    warn!("Failed to parse batch result JSON");
                                }
                            }
                            continue;
                        }

                        // If we're capturing batch result JSON, add to buffer
                        if _capturing_batch_result {
                            _batch_json_buffer.push_str(&line_str);
                            continue;
                        }

                        // Process other types of messages
                        if line_str.contains("\"progressType\"") || line_str.contains("\"status\"") || 
                           line_str.contains("\"type\":\"progress_detail\"") || line_str.contains("\"type\":\"detailed_progress\"") {
                            // Try to parse as Progress type from core module first
                            if let Ok(progress) = serde_json::from_str::<crate::core::Progress>(&line_str) {
                                self.report_progress(&progress);
                            } 
                            // Try to parse as progress update (simplified format)
                            else if let Ok(update) = serde_json::from_str::<super::types::ProgressUpdate>(&line_str) {
                                self.handle_progress_update(update);
                            } 
                            // Try to parse as detailed progress update with file-specific metrics
                            else if let Ok(detailed_update) = serde_json::from_str::<DetailedProgressUpdate>(&line_str) {
                                self.handle_detailed_progress_update(detailed_update);
                            }
                            // Try to parse as legacy progress message
                            else if let Ok(message) = serde_json::from_str::<super::types::ProgressMessage>(&line_str) {
                                self.handle_progress(message);
                            }
                            // If none of the above parsers succeed, log the message but don't error
                            else {
                                debug!("Could not parse progress message: {}", line_str);
                            }
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
        
        // Release the process back to the pool
        self.pool.release().await;

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
        
        #[cfg(feature = "benchmarking")]
        {
            return Ok((results, final_metrics));
        }
        #[cfg(not(feature = "benchmarking"))]
        {
            return Ok((results, None));
        }
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
                        if line_str.contains("\"progressType\"") || line_str.contains("\"status\"") || 
                           line_str.contains("\"type\":\"progress_detail\"") || line_str.contains("\"type\":\"detailed_progress\"") {
                            // Try to parse as Progress type from core module first
                            if let Ok(progress) = serde_json::from_str::<crate::core::Progress>(&line_str) {
                                self.report_progress(&progress);
                            } 
                            // Try to parse as progress update (simplified format)
                            else if let Ok(update) = serde_json::from_str::<super::types::ProgressUpdate>(&line_str) {
                                self.handle_progress_update(update);
                            } 
                            // Try to parse as detailed progress update with file-specific metrics
                            else if let Ok(detailed_update) = serde_json::from_str::<DetailedProgressUpdate>(&line_str) {
                                self.handle_detailed_progress_update(detailed_update);
                            }
                            // Try to parse as legacy progress message
                            else if let Ok(message) = serde_json::from_str::<super::types::ProgressMessage>(&line_str) {
                                self.handle_progress(message);
                            }
                            // If none of the above parsers succeed, log the message but don't error
                            else {
                                debug!("Could not parse progress message: {}", line_str);
                            }
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
                // Check if we have detailed optimization metrics in the metadata
                let has_detailed_metrics = progress.metadata.as_ref()
                    .and_then(|m| m.get("formattedMessage"))
                    .is_some();
                
                if has_detailed_metrics {
                    // Extract and log the formatted message with detailed metrics
                    if let Some(formatted_msg) = progress.metadata.as_ref()
                        .and_then(|m| m.get("formattedMessage"))
                        .and_then(|m| m.as_str()) 
                    {
                        // Use INFO level for significant progress points
                        if progress.progress_percentage % 10 == 0 || 
                           progress.progress_percentage == 25 || 
                           progress.progress_percentage == 50 || 
                           progress.progress_percentage == 75 ||
                           progress.progress_percentage >= 100 {
                            info!("📊 {}", formatted_msg);
                        } else {
                            debug!("📊 {}", formatted_msg);
                        }
                    }
                } else {
                    // Log regular progress updates (original behavior)
                    if progress.progress_percentage % 10 == 0 || 
                       progress.progress_percentage == 25 || 
                       progress.progress_percentage == 50 || 
                       progress.progress_percentage == 75 {
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