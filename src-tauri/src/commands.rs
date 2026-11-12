//! Tauri commands invoked from the React frontend.

use tauri::{AppHandle, Emitter};

use crate::error::{OptimizerError, OptimizerResult};
use crate::optimize::run_batch;
use crate::types::{BatchSummary, ImageSettings};

/// Optimizes a batch of images and returns the totals.
///
/// Each result is written to an `optimized` folder next to its source file.
/// A file that fails is reported in the summary and does not stop the batch.
/// While it runs, `image_optimization_progress` events carry the running totals.
///
/// # Arguments
/// * `paths` - Source image paths
/// * `settings` - Quality, resize and format settings, shared by every file
#[tauri::command]
pub async fn optimize_images(
    app: AppHandle,
    paths: Vec<String>,
    settings: ImageSettings,
) -> OptimizerResult<BatchSummary> {
    if !(1..=100).contains(&settings.quality) {
        return Err(OptimizerError::validation(format!(
            "Invalid quality: {}. Must be between 1 and 100",
            settings.quality
        )));
    }

    // CPU-bound and blocking: keep it off the async runtime.
    tokio::task::spawn_blocking(move || {
        run_batch(&paths, &settings, |progress| {
            let _ = app.emit("image_optimization_progress", progress);
        })
    })
    .await
    .map_err(|e| OptimizerError::processing(format!("Batch crashed: {e}")))
}
