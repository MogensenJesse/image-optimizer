// src-tauri/src/processing/libvips/executor.rs

//! Native libvips executor that replaces the Node.js Sharp sidecar.
//!
//! Each image is processed inside a `tokio::task::spawn_blocking` call so the
//! async runtime is never blocked. libvips manages its own internal thread pool
//! for per-image parallelism, so images are dispatched sequentially here while
//! libvips concurrently saturates CPU cores within each image.

use std::path::Path;
use tauri::AppHandle;
use tauri::Emitter;
use tracing::{debug, warn};

use libvips::VipsImage;

use crate::core::{ImageTask, OptimizationResult};
use crate::utils::{OptimizerError, OptimizerResult, extract_filename};

use super::formats::save_image_as;
use super::resize::apply_resize;

/// Executor that processes images directly via libvips with no subprocess overhead.
pub struct NativeExecutor {
    app: AppHandle,
}

impl NativeExecutor {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    /// Processes all `tasks` sequentially, emitting a progress event after each image.
    ///
    /// libvips uses its own internal thread pool for within-image parallelism, so
    /// sequential dispatch here is intentional and avoids thread oversubscription.
    pub async fn execute_batch(
        &self,
        tasks: &[ImageTask],
    ) -> OptimizerResult<Vec<OptimizationResult>> {
        let total = tasks.len();
        let mut results = Vec::with_capacity(total);

        for (idx, task) in tasks.iter().enumerate() {
            let completed = idx + 1;
            let task_clone = task.clone();

            let result = tokio::task::spawn_blocking(move || optimize_single(&task_clone))
                .await
                .map_err(|e| OptimizerError::processing(format!("Task panicked: {e}")))?;

            match result {
                Ok(opt_result) => {
                    self.emit_progress(completed, total, task, &opt_result, None);
                    results.push(opt_result);
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    warn!("Image optimization failed for {}: {}", task.input_path, error_msg);

                    // Emit error progress so the frontend stays in sync
                    self.emit_error_progress(completed, total, task, &error_msg);

                    // Push a failed result so the caller gets one result per task
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
        task: &ImageTask,
        result: &OptimizationResult,
        _worker_id: Option<usize>,
    ) {
        let percentage = (completed * 100) / total;
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
            "status": if completed == total { "complete" } else { "processing" },
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
            "status": "error",
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

    // Load image
    let image = VipsImage::new_from_file(input_path)
        .map_err(|e| OptimizerError::processing(format!("Failed to load '{input_path}': {e}")))?;

    debug!(
        "Loaded '{}': {}×{}",
        extract_filename(input_path),
        image.get_width(),
        image.get_height()
    );

    // Apply resize (no-op when mode is "none")
    let image = apply_resize(image, &settings.resize)?;

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

/// Resolves "original" to the actual input format and normalises "jpg" → "jpeg".
fn resolve_output_format(input_path: &str, requested: &str) -> OptimizerResult<String> {
    if requested == "original" {
        // Derive format from the input file extension
        let ext = Path::new(input_path)
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| OptimizerError::format("Input file has no extension"))?
            .to_lowercase();

        return Ok(normalise_format(&ext));
    }

    Ok(normalise_format(requested))
}

fn normalise_format(fmt: &str) -> String {
    match fmt {
        "jpg" => "jpeg".to_string(),
        other => other.to_lowercase(),
    }
}

/// Returns `output_path` with the extension corrected to match `format`.
///
/// When the output format differs from the extension already on `output_path`
/// (e.g. converting foo.jpg → webp), the extension is replaced. When the
/// output format is "original" the input extension is preserved.
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

    // Also normalise current extension for comparison (jpg == jpeg)
    let current_norm = normalise_format(&current_ext);
    if current_norm == format || (current_ext == "jpg" && format == "jpeg") {
        return output_path.to_string();
    }

    // Replace extension
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let parent = path.parent().unwrap_or(Path::new(""));

    // Fall back to input stem if output stem is empty
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
