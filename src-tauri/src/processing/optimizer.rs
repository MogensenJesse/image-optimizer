use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::Mutex;
use tauri_plugin_shell::ShellExt;
use crate::core::{OptimizationResult, ImageTask};
use crate::utils::{
    OptimizerError,
    OptimizerResult,
    get_file_size,
    ensure_parent_dir,
    validate_input_path,
    validate_output_path,
};
use crate::benchmarking::{BenchmarkMetrics, ProcessingStage, Duration};
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
    ) -> OptimizerResult<(Vec<OptimizationResult>, Option<(Duration, Duration, Duration)>)> {
        let mut results = Vec::with_capacity(tasks.len());
        let mut stage_times = None;
        
        // Process tasks in chunks to reduce process spawning
        for chunk in tasks.chunks(BATCH_SIZE) {
            let (chunk_results, times) = self.process_chunk(app, chunk.to_vec()).await?;
            results.extend(chunk_results);
            // Only keep the last chunk's timing info
            stage_times = Some(times);
        }
        
        Ok((results, stage_times))
    }

    async fn process_chunk(
        &self,
        app: &tauri::AppHandle,
        tasks: Vec<ImageTask>,
    ) -> OptimizerResult<(Vec<OptimizationResult>, (Duration, Duration, Duration))> {
        let mut results = Vec::with_capacity(tasks.len());
        let mut original_sizes = Vec::with_capacity(tasks.len());

        // Start loading phase timing
        let loading_start = std::time::Instant::now();
        debug!("Starting loading phase");

        // Validate paths and get original sizes
        for task in &tasks {
            // Validate input and output paths
            validate_input_path(&task.input_path).await?;
            validate_output_path(&task.output_path).await?;
            
            // Ensure output directory exists
            ensure_parent_dir(&task.output_path).await?;

            // Get original file size
            let original_size = get_file_size(&task.input_path).await?;
            original_sizes.push(original_size);

            // Track active task
            let mut active = self.active_tasks.lock().await;
            active.insert(task.input_path.clone());
        }

        let loading_time = Duration::new_unchecked(loading_start.elapsed().as_secs_f64());
        debug!("Loading phase completed in {} (validation and file reading)", loading_time);

        // Start optimization phase timing
        let optimization_start = std::time::Instant::now();
        debug!("Starting optimization phase");

        // Process the batch using Sharp sidecar
        let result = self.run_sharp_process_batch(app, &tasks).await;

        let optimization_time = Duration::new_unchecked(optimization_start.elapsed().as_secs_f64());
        debug!("Optimization phase completed in {} (Sharp processing)", optimization_time);

        // Start saving phase timing
        let saving_start = std::time::Instant::now();
        debug!("Starting saving phase");

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

        let saving_time = Duration::new_unchecked(saving_start.elapsed().as_secs_f64());
        debug!("Saving phase completed in {} (result collection and verification)", saving_time);

        match saving_result {
            Ok(_) => Ok((results, (loading_time, optimization_time, saving_time))),
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
            .map_err(|e| OptimizerError::sidecar(format!("Failed to run Sharp process: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(OptimizerError::sidecar(format!("Sharp process failed: {}", stderr)))
        } else {
            Ok(())
        }
    }

    pub async fn get_active_tasks(&self) -> Vec<String> {
        self.active_tasks.lock().await.iter().cloned().collect()
    }
} 