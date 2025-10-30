use memmap2::MmapOptions;
use std::fs::OpenOptions;
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandEvent, TerminatedPayload};
use crate::utils::{OptimizerError, OptimizerResult};
use crate::core::{ImageTask, OptimizationResult};
use super::types::{SharpResult, DetailedProgressUpdate};
use super::progress_handler::ProgressHandler;
use tracing::{debug, warn};
use serde_json;
use serde::Deserialize;
use std::str::from_utf8;
use tauri::async_runtime::Receiver;

#[derive(Debug, Deserialize)]
pub struct BatchOutput {
    pub results: Vec<SharpResult>,
    // Ignore metrics field from sidecar (not used by backend, kept for deserialization compatibility)
    #[serde(default)]
    #[allow(dead_code)]
    pub metrics: Option<serde_json::Value>,
}

/// Memory-mapped file executor that uses shared memory for batch data transfer
pub struct MemoryMapExecutor {
    app: AppHandle,
    progress_handler: ProgressHandler,
}

impl MemoryMapExecutor {
    pub fn new(app: AppHandle) -> Self {
        let app_clone = app.clone();
        Self {
            app: app_clone.clone(),
            progress_handler: ProgressHandler::new(app_clone),
        }
    }
    
    /// Warms up the executor by processing a minimal image task
    pub async fn warmup(&self) -> OptimizerResult<()> {
        debug!("Warming up MemoryMapExecutor...");
        
        // Create a minimal task that will initialize the Sharp pipeline
        let dummy_task = ImageTask::create_warmup_task()?;
        
        // Execute the task but don't care about the result
        let _ = self.execute_batch(&[dummy_task]).await?;
        
        debug!("MemoryMapExecutor warmup completed successfully");
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
    
    /// Handles command events from the sidecar process - reuse from DirectExecutor
    async fn handle_sidecar_events(
        &self,
        tasks: &[ImageTask],
        mut rx: Receiver<CommandEvent>,
    ) -> OptimizerResult<Vec<OptimizationResult>> {
        let mut results = Vec::new();
        let mut batch_json_buffer = String::new();
        let mut capturing_batch_result = false;
        
        // Process output events in real-time
        debug!("Starting to process output events from sidecar");
        
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) | CommandEvent::Stderr(line) => {
                    if let Some(line_str) = Self::process_line(&line) {
                        capturing_batch_result = self.process_output_line(
                            &line_str,
                            &mut batch_json_buffer,
                            capturing_batch_result,
                            tasks,
                            &mut results,
                        );
                    }
                }
                CommandEvent::Terminated(payload) => {
                    let payload = payload as TerminatedPayload;
                    if let Some(code) = payload.code {
                        if code != 0 {
                            return Err(OptimizerError::sidecar(format!("Sharp process exited with code {}", code)));
                        }
                    }
                }
                _ => {}
            }
        }
        
        // Validate results
        if results.is_empty() {
            return Err(OptimizerError::processing("No results received from sidecar".to_string()));
        }
        
        Ok(results)
    }
    
    /// Helper function to process output lines
    fn process_line(line: &[u8]) -> Option<String> {
        from_utf8(line).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    }
    
    /// Process a line of output from the sidecar
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
    
    /// Clean up temporary file, ignoring errors
    fn cleanup_temp_file(&self, file_path: &std::path::Path) {
        if file_path.exists() {
            if let Err(e) = std::fs::remove_file(file_path) {
                warn!("Failed to clean up temporary file {}: {}", file_path.display(), e);
            } else {
                debug!("Successfully cleaned up temporary file: {}", file_path.display());
            }
        }
    }
    
    /// Execute a batch of tasks using memory-mapped file for data transfer
    pub async fn execute_batch(&self, tasks: &[ImageTask]) 
        -> OptimizerResult<Vec<OptimizationResult>> {
        debug!("Processing batch of {} tasks using memory-mapped file", tasks.len());
        
        // Generate a unique temporary file path
        let temp_file_path = std::env::temp_dir().join(format!("image_optimizer_mmap_{}.dat", 
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()));
        
        debug!("Using temporary file for memory mapping: {:?}", temp_file_path);
        
        // Prepare batch data
        let batch_json = self.prepare_batch_data(tasks)?;
        let data_len = batch_json.len();
        
        debug!("Prepared batch data: {} bytes for {} tasks", data_len, tasks.len());
        
        // Use a block to ensure resources are properly dropped
        let results = {
            // Create and size the file
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&temp_file_path)
                .map_err(|e| OptimizerError::processing(format!("Failed to create memory map file: {}", e)))?;
            
            file.set_len(data_len as u64)
                .map_err(|e| OptimizerError::processing(format!("Failed to set memory map file size: {}", e)))?;
            
            // Map the file into memory
            // SAFETY: We've properly created and sized the file, and it will remain valid
            // for the lifetime of the mmap. We also ensure exclusive access.
            let mut mmap = unsafe { 
                MmapOptions::new().map_mut(&file)
                    .map_err(|e| OptimizerError::processing(format!("Failed to map file to memory: {}", e)))?
            };
            
            // Write data to memory-mapped region
            mmap.copy_from_slice(batch_json.as_bytes());
            mmap.flush()
                .map_err(|e| OptimizerError::processing(format!("Failed to flush memory map: {}", e)))?;
            
            // Create sidecar command
            debug!("Creating sidecar command for batch processing via memory-mapped file");
            let cmd = self.create_sidecar_command()?;
            
            // Run the command with the memory-mapped file path
            debug!("Spawning Sharp sidecar process with memory-mapped file");
            let (rx, _child) = cmd
                .args(&["optimize-batch-mmap", &temp_file_path.to_string_lossy()])
                .spawn()
                .map_err(|e| OptimizerError::sidecar(format!("Failed to spawn Sharp process: {}", e)))?;
            
            debug!("Sidecar process started, waiting for results");
            
            // Handle sidecar events and return results
            let results = self.handle_sidecar_events(tasks, rx).await?;
            
            // Explicitly unmap before dropping to ensure resources are released properly
            drop(mmap);
            
            // Close file handle explicitly
            drop(file);
            
            results
        }; // End of block - all resources are dropped here
        
        // Clean up the temporary file
        // Note: The sidecar should also try to clean up the file after reading
        self.cleanup_temp_file(&temp_file_path);
        
        debug!("Batch processing completed, returning {} results", results.len());
        Ok(results)
    }
} 