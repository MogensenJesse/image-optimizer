//! Tauri command handlers for image optimization.

use std::time::Instant;
use tauri::State;
use tracing::debug;
use crate::core::{AppState, ImageSettings, OptimizationResult};
use crate::core::ImageTask;
use crate::utils::{OptimizerResult, validate_task};

/// Optimizes a single image with the given settings.
///
/// Convenience wrapper around [`optimize_images`] for single-image operations.
#[tauri::command]
pub async fn optimize_image(
    state: State<'_, AppState>,
    input_path: String,
    output_path: String,
    settings: ImageSettings,
) -> OptimizerResult<OptimizationResult> {
    optimize_images(
        state,
        vec![ImageTask {
            input_path,
            output_path,
            settings,
        }]
    )
    .await
    .and_then(|results| results.into_iter().next().ok_or_else(|| {
        crate::utils::OptimizerError::processing("No result returned".to_string())
    }))
}

/// Optimizes multiple images in batch with progress tracking.
///
/// Images are processed in chunks of 500 to keep memory bounded.
/// Progress events use **overall** job counts (not per-chunk) so the
/// frontend receives a simple monotonic stream from 1..N.
///
/// # Arguments
/// * `state` - Application state containing the executor
/// * `tasks` - Vector of image tasks to process
///
/// # Returns
/// Vector of optimization results, one per input task.
#[tauri::command]
pub async fn optimize_images(
    state: State<'_, AppState>,
    tasks: Vec<ImageTask>,
) -> OptimizerResult<Vec<OptimizationResult>> {
    let job_total = tasks.len();
    let job_start = Instant::now();
    debug!("Received optimize_images command for {} images", job_total);
    
    for task in &tasks {
        validate_task(task).await?;
    }

    const CHUNK_SIZE: usize = 500;
    let chunks: Vec<_> = tasks.chunks(CHUNK_SIZE).collect();
    debug!("Processing {} images in {} chunks of size {}", job_total, chunks.len(), CHUNK_SIZE);
    
    let mut all_results = Vec::with_capacity(job_total);
    let executor = state.create_executor();
    let mut offset = 0;
    
    for (i, chunk) in chunks.iter().enumerate() {
        debug!("Processing chunk {}/{} ({} images)", i + 1, chunks.len(), chunk.len());
        let results = executor.execute_batch(chunk, offset, job_total, job_start).await?;
        offset += chunk.len();
        all_results.extend(results);
        debug!("Completed chunk {}/{} ({}/{})", i + 1, chunks.len(), offset, job_total);
    }
    
    debug!("All chunks processed, returning {} results", all_results.len());
    Ok(all_results)
}
