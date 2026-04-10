// src-tauri/src/processing/libvips/executor.rs

//! Native libvips executor for batch image optimization.
//!
//! Each image is processed inside a `tokio::task::spawn_blocking` call so the
//! async runtime is never blocked. libvips manages its own internal thread pool
//! for per-image parallelism, so images are dispatched sequentially here while
//! libvips concurrently saturates CPU cores within each image.

use std::path::Path;
use std::time::Instant;
use tauri::AppHandle;
use tauri::Emitter;
use tracing::{debug, warn};

use libvips::VipsImage;
use libvips::ops::Access;

use crate::core::{ImageTask, OptimizationResult};
use crate::utils::{OptimizerError, OptimizerResult, extract_filename, normalize_format};

use super::formats::save_image_as;
use super::resize::{apply_resize, needs_resize, load_and_resize};

/// Executor that processes images directly via libvips with no subprocess overhead.
pub struct NativeExecutor {
    app: AppHandle,
}

impl NativeExecutor {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    /// Processes a chunk of tasks, emitting progress events with **overall** job counts.
    ///
    /// `offset` is the number of tasks already completed in prior chunks so that
    /// `completedTasks` and `totalTasks` in emitted events reflect the full job,
    /// not just this chunk.
    ///
    /// libvips uses its own internal thread pool for within-image parallelism, so
    /// sequential dispatch here is intentional and avoids thread oversubscription.
    pub async fn execute_batch(
        &self,
        tasks: &[ImageTask],
        offset: usize,
        job_total: usize,
        job_start: Instant,
    ) -> OptimizerResult<Vec<OptimizationResult>> {
        let mut results = Vec::with_capacity(tasks.len());

        for (idx, task) in tasks.iter().enumerate() {
            let overall_completed = offset + idx + 1;
            let task_clone = task.clone();

            let result = tokio::task::spawn_blocking(move || optimize_single(&task_clone))
                .await
                .map_err(|e| OptimizerError::processing(format!("Task panicked: {e}")))?;

            match result {
                Ok(opt_result) => {
                    self.emit_progress(overall_completed, job_total, job_start, task, &opt_result);
                    results.push(opt_result);
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    warn!("Image optimization failed for {}: {}", task.input_path, error_msg);

                    self.emit_error_progress(overall_completed, job_total, task, &error_msg);

                    results.push(OptimizationResult {
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
                    });
                }
            }
        }

        Ok(results)
    }

    // ── Progress emission ────────────────────────────────────────────────────────────

    fn emit_progress(
        &self,
        completed: usize,
        total: usize,
        job_start: Instant,
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

        let mut metadata = serde_json::json!({
            "formattedMessage": formatted_msg,
            "fileName": file_name,
            "originalSize": result.original_size,
            "optimizedSize": result.optimized_size,
            "savedBytes": result.saved_bytes,
            "compressionRatio": format!("{:.2}", result.compression_ratio),
        });

        if is_final {
            let duration_secs = job_start.elapsed().as_secs_f64();
            metadata["totalDuration"] = serde_json::json!(format!("{duration_secs:.2}"));
        }

        let payload = serde_json::json!({
            "completedTasks": completed,
            "totalTasks": total,
            "progressPercentage": percentage,
            "status": if is_final { "complete" } else { "processing" },
            "metadata": metadata,
        });

        let _ = self.app.emit("image_optimization_progress", payload);
    }

    fn emit_error_progress(&self, completed: usize, total: usize, task: &ImageTask, error: &str) {
        let percentage = (completed * 100) / total;
        let file_name = extract_filename(&task.input_path).to_string();

        let payload = serde_json::json!({
            "completedTasks": completed,
            "totalTasks": total,
            "progressPercentage": percentage,
            "status": if completed == total { "complete" } else { "error" },
            "metadata": {
                "fileName": file_name,
                "error": error,
            },
        });

        let _ = self.app.emit("image_optimization_progress", payload);
    }
}

// ── Blocking image processing (runs on tokio's blocking thread pool) ──────────────────

/// Optimises one image task synchronously.
///
/// Runs in a blocking thread so libvips can use its internal thread pool freely.
fn optimize_single(task: &ImageTask) -> OptimizerResult<OptimizationResult> {
    let input_path = &task.input_path;
    let settings = &task.settings;

    // Original size before any transformation
    let original_size = std::fs::metadata(input_path)
        .map(|m| m.len())
        .map_err(|e| OptimizerError::processing(format!("Cannot read input file: {e}")))?;

    // Determine the output format
    let output_format = resolve_output_format(input_path, &settings.output_format)?;

    // Adjust the output path extension to match the resolved format
    let output_path = ensure_correct_extension(&task.output_path, input_path, &output_format);

    // Ensure the output directory exists
    if let Some(parent) = Path::new(&output_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            OptimizerError::processing(format!("Cannot create output directory: {e}"))
        })?;
    }

    // When resizing, use file-based thumbnail for shrink-on-load (skips
    // decoding most DCT coefficients on large JPEG downsizes). Otherwise
    // load the full image for compression-only operations.
    let image = if needs_resize(&settings.resize) {
        let img = load_and_resize(input_path, &settings.resize)?;
        debug!(
            "Loaded+resized '{}': {}×{}",
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
            "Loaded '{}': {}×{}",
            extract_filename(input_path),
            img.get_width(),
            img.get_height()
        );
        apply_resize(img, &settings.resize)?
    };

    // Encode and save
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

// ── Helpers ───────────────────────────────────────────────────────────────────────────

/// Resolves "original" to the actual input format and normalizes "jpg" → "jpeg".
fn resolve_output_format(input_path: &str, requested: &str) -> OptimizerResult<String> {
    if requested == "original" {
        let ext = Path::new(input_path)
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| OptimizerError::format("Input file has no extension"))?;

        return Ok(normalize_format(ext));
    }

    Ok(normalize_format(requested))
}

/// Returns `output_path` with the extension corrected to match `format`.
///
/// When the output format differs from the extension already on `output_path`
/// (e.g. converting foo.jpg → webp), the extension is replaced.
fn ensure_correct_extension(output_path: &str, input_path: &str, format: &str) -> String {
    let new_ext = match format {
        "jpeg" => "jpg",
        other => other,
    };

    let path = Path::new(output_path);
    let current_ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if normalize_format(&current_ext) == format {
        return output_path.to_string();
    }

    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let parent = path.parent().unwrap_or(Path::new(""));

    let stem = if stem.is_empty() {
        Path::new(input_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output")
    } else {
        stem
    };

    parent
        .join(format!("{stem}.{new_ext}"))
        .to_string_lossy()
        .to_string()
}
