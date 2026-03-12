//! Tauri command handlers for image optimization.

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
/// Per-image progress events (`image_optimization_progress`) are emitted by
/// the executor; this function only handles chunking and aggregation.
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
    let task_count = tasks.len();
    debug!("Received optimize_images command for {} images", task_count);
    
    for task in &tasks {
        validate_task(task).await?;
    }

    const CHUNK_SIZE: usize = 500;
    let chunks: Vec<_> = tasks.chunks(CHUNK_SIZE).collect();
    debug!("Processing {} images in {} chunks of size {}", task_count, chunks.len(), CHUNK_SIZE);
    
    let mut all_results = Vec::with_capacity(tasks.len());
    let executor = state.create_executor();
    
    for (i, chunk) in chunks.iter().enumerate() {
        debug!("Processing chunk {}/{} ({} images)", i + 1, chunks.len(), chunk.len());
        let results = executor.execute_batch(chunk).await?;
        all_results.extend(results);
        debug!("Completed chunk {}/{} - Overall progress: {}% ({}/{})",
            i + 1, chunks.len(), 
            ((i + 1) * 100) / chunks.len(),
            (i + 1) * chunk.len().min(CHUNK_SIZE),
            task_count
        );
    }
    
    debug!("All chunks processed, returning {} results", all_results.len());
    Ok(all_results)
}
