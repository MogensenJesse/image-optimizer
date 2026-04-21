// src-tauri/src/processing/libvips/executor.rs

//! Native executor for batch image optimization.
//!
//! Raster images are processed via libvips; SVG files are dispatched to the
//! [`crate::processing::svg`] module. Each task runs inside a
//! `tokio::task::spawn_blocking` call so the async runtime is never blocked.

use std::path::Path;
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{debug, warn};

use libvips::VipsImage;
use libvips::ops::Access;

use crate::core::{ImageTask, OptimizationResult};
use crate::utils::{
    ImageFormat, OptimizerError, OptimizerResult,
    extract_filename, format_from_extension,
    resolve_output_format, ensure_correct_extension,
};

use super::formats::save_image_as;
use super::resize::{needs_resize, load_and_resize};
use crate::processing::svg::optimize_svg;

/// Executor that processes images directly via libvips with no subprocess overhead.
pub struct NativeExecutor {
    app: AppHandle,
    max_concurrent: usize,
}

impl NativeExecutor {
    pub fn new(app: AppHandle, max_concurrent: usize) -> Self {
        Self { app, max_concurrent }
    }

    /// Processes a chunk of tasks with bounded concurrency, emitting progress
    /// events with **overall** job counts.
    ///
    /// `offset` is the number of tasks already completed in prior chunks so that
    /// `completedTasks` and `totalTasks` in emitted events reflect the full job,
    /// not just this chunk.
    ///
    /// Progress events are emitted **as each image finishes** (not after the
    /// entire batch), so the frontend sees a smooth incremental bar.
    pub async fn execute_batch(
        &self,
        tasks: &[ImageTask],
        offset: usize,
        job_total: usize,
    ) -> OptimizerResult<Vec<OptimizationResult>> {
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        let mut join_set = JoinSet::new();

        // Spawn all tasks up-front; each task acquires a semaphore permit
        // internally so that at most MAX_CONCURRENT run at once.  This lets
        // the spawning loop finish immediately so we reach the join_next()
        // collection loop — which emits progress events — right away.
        for (idx, task) in tasks.iter().enumerate() {
            let sem = semaphore.clone();
            let task_clone = task.clone();

            join_set.spawn(async move {
                let _permit = sem.acquire_owned().await;
                let result =
                    tokio::task::spawn_blocking(move || optimize_single(&task_clone)).await;
                (idx, result)
            });
        }

        let mut indexed_results: Vec<(usize, OptimizationResult)> =
            Vec::with_capacity(tasks.len());
        let mut completed_count = 0usize;

        while let Some(join_result) = join_set.join_next().await {
            let (idx, inner) = join_result
                .map_err(|e| OptimizerError::processing(format!("Task panicked: {e}")))?;

            completed_count += 1;
            let overall_completed = offset + completed_count;
            let task = &tasks[idx];

            match inner {
                Ok(Ok(opt_result)) => {
                    self.emit_progress(overall_completed, job_total, task, &opt_result);
                    indexed_results.push((idx, opt_result));
                }
                Ok(Err(e)) => {
                    let error_msg = e.to_string();
                    warn!("Optimization failed for {}: {}", task.input_path, error_msg);
                    self.emit_error_progress(overall_completed, job_total, task, &error_msg);

                    indexed_results.push((idx, OptimizationResult {
                        original_path: task.input_path.clone(),
                        optimized_path: task.output_path.clone(),
                        original_size: std::fs::metadata(&task.input_path)
                            .map(|m| m.len())
                            .unwrap_or(0),
                        optimized_size: 0,
                        success: false,
                        error: Some(error_msg),
                        saved_bytes: 0,
                        compression_ratio: 0.0,
                    }));
                }
                Err(e) => {
                    let error_msg = format!("Task panicked: {e}");
                    warn!("Optimization failed for {}: {}", task.input_path, error_msg);
                    self.emit_error_progress(overall_completed, job_total, task, &error_msg);

                    indexed_results.push((idx, OptimizationResult {
                        original_path: task.input_path.clone(),
                        optimized_path: task.output_path.clone(),
                        original_size: std::fs::metadata(&task.input_path)
                            .map(|m| m.len())
                            .unwrap_or(0),
                        optimized_size: 0,
                        success: false,
                        error: Some(error_msg),
                        saved_bytes: 0,
                        compression_ratio: 0.0,
                    }));
                }
            }
        }

        indexed_results.sort_by_key(|(idx, _)| *idx);
        let results = indexed_results.into_iter().map(|(_, r)| r).collect();
        Ok(results)
    }

    // -- Progress emission ----------------------------------------------------

    fn emit_progress(
        &self,
        completed: usize,
        total: usize,
        task: &ImageTask,
        result: &OptimizationResult,
    ) {
        let percentage = (completed * 100) / total;
        let is_final = completed == total;
        let file_name = extract_filename(&task.input_path).to_string();
        let saved_kb = result.saved_bytes as f64 / 1024.0;

        let formatted_msg = format!(
            "{file_name} optimized ({saved_kb:.2} KB saved / {:.0}% compression)",
            result.compression_ratio
        );

        debug!("{formatted_msg}");

        let metadata = serde_json::json!({
            "formattedMessage": formatted_msg,
            "fileName": file_name,
            "originalSize": result.original_size,
            "optimizedSize": result.optimized_size,
            "savedBytes": result.saved_bytes,
            "compressionRatio": format!("{:.2}", result.compression_ratio),
        });

        let payload = serde_json::json!({
            "completedTasks": completed,
            "totalTasks": total,
            "progressPercentage": percentage,
            "status": if is_final { "complete" } else { "processing" },
            "metadata": metadata,
        });

        if let Err(e) = self.app.emit("image_optimization_progress", payload) {
            warn!("Failed to emit progress event: {e}");
        }
    }

    fn emit_error_progress(&self, completed: usize, total: usize, task: &ImageTask, error: &str) {
        let percentage = (completed * 100) / total;
        let file_name = extract_filename(&task.input_path).to_string();

        let payload = serde_json::json!({
            "completedTasks": completed,
            "totalTasks": total,
            "progressPercentage": percentage,
            "status": "error",
            "metadata": {
                "fileName": file_name,
                "error": error,
            },
        });

        if let Err(e) = self.app.emit("image_optimization_progress", payload) {
            warn!("Failed to emit error progress event: {e}");
        }
    }
}

// -- Blocking processing (runs on tokio's blocking thread pool) ---------------

/// Optimises one task synchronously -- dispatches to SVG or raster pipeline.
fn optimize_single(task: &ImageTask) -> OptimizerResult<OptimizationResult> {
    let format = format_from_extension(&task.input_path)?;
    if format == ImageFormat::SVG {
        return optimize_svg(task);
    }
    optimize_raster(task)
}

// -- Raster image optimization ------------------------------------------------

/// Optimises one raster image task synchronously via libvips.
fn optimize_raster(task: &ImageTask) -> OptimizerResult<OptimizationResult> {
    let input_path = &task.input_path;
    let settings = &task.settings;

    let original_size = std::fs::metadata(input_path)
        .map(|m| m.len())
        .map_err(|e| OptimizerError::processing(format!("Cannot read input file: {e}")))?;

    let output_format = resolve_output_format(input_path, &settings.output_format)?;
    let output_path = ensure_correct_extension(&task.output_path, input_path, &output_format);

    if let Some(parent) = Path::new(&output_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            OptimizerError::processing(format!("Cannot create output directory: {e}"))
        })?;
    }

    let image = if needs_resize(&settings.resize) {
        let img = load_and_resize(input_path, &settings.resize)?;
        debug!(
            "Loaded+resized '{}': {}x{}",
            extract_filename(input_path),
            img.get_width(),
            img.get_height()
        );
        img
    } else {
        let img = VipsImage::new_from_file_access(input_path, Access::Sequential, false)
            .map_err(|_| OptimizerError::processing(format!(
                "Failed to load '{input_path}': {}",
                super::vips_error_buffer_string()
            )))?;
        debug!(
            "Loaded '{}': {}x{}",
            extract_filename(input_path),
            img.get_width(),
            img.get_height()
        );
        img
    };

    save_image_as(&image, &output_path, &output_format, &settings.quality)?;

    let optimized_size = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    let saved_bytes = original_size as i64 - optimized_size as i64;
    let compression_ratio = if original_size > 0 {
        saved_bytes as f64 / original_size as f64 * 100.0
    } else {
        0.0
    };

    debug!(
        "'{}' → {} bytes saved ({:.1}%)",
        extract_filename(input_path),
        saved_bytes,
        compression_ratio
    );

    Ok(OptimizationResult {
        original_path: input_path.clone(),
        optimized_path: output_path,
        original_size,
        optimized_size,
        success: true,
        error: None,
        saved_bytes,
        compression_ratio,
    })
}
