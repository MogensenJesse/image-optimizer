//! Tauri command handlers for image optimization.

use std::path::Path;
use tauri::State;
use tauri::Emitter;
use tracing::debug;
use crate::core::{AppState, ImageSettings, OptimizationResult, BenchmarkResult};
use crate::core::ImageTask;
use crate::utils::{OptimizerResult, OptimizerError, validate_task, validate_input_path};

/// Optimizes a single image with the given settings.
///
/// This is a convenience wrapper around [`optimize_images`] for single-image operations.
///
/// # Arguments
/// * `app` - Tauri app handle for event emission
/// * `state` - Application state containing the executor
/// * `input_path` - Path to the source image
/// * `output_path` - Path for the optimized output
/// * `settings` - Optimization settings (quality, resize, format)
///
/// # Returns
/// The optimization result with file sizes and compression statistics.
#[tauri::command]
pub async fn optimize_image(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input_path: String,
    output_path: String,
    settings: ImageSettings,
) -> OptimizerResult<OptimizationResult> {
    optimize_images(
        app,
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
/// Images are processed in chunks of 500 to avoid overwhelming the system.
/// Progress events are emitted to the frontend via Tauri events.
///
/// # Arguments
/// * `app` - Tauri app handle for event emission
/// * `state` - Application state containing the executor
/// * `tasks` - Vector of image tasks to process
///
/// # Returns
/// Vector of optimization results, one per input task.
///
/// # Events Emitted
/// * `batch-progress` - Overall batch progress updates
/// * `image_optimization_progress` - Per-image progress updates
#[tauri::command]
pub async fn optimize_images(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    tasks: Vec<ImageTask>,
) -> OptimizerResult<Vec<OptimizationResult>> {
    let task_count = tasks.len();
    debug!("Received optimize_images command for {} images", task_count);
    
    // Validate tasks
    for task in &tasks {
        validate_task(task).await?;
    }

    // Process in chunks so the frontend receives batch-progress events
    // between chunks, keeping the UI responsive for very large batches.
    const CHUNK_SIZE: usize = 500;
    let chunks: Vec<_> = tasks.chunks(CHUNK_SIZE).collect();
    debug!("Processing {} images in {} chunks of size {}", task_count, chunks.len(), CHUNK_SIZE);
    
    let mut all_results = Vec::with_capacity(tasks.len());
    
    // Create executor
    let executor = state.create_executor();
    
    // Track overall progress for the frontend
    let mut completed_tasks = 0;
    let total_tasks = task_count;
    
    // Process each chunk
    for (i, chunk) in chunks.iter().enumerate() {
        debug!("Processing chunk {}/{} ({} images)", i + 1, chunks.len(), chunk.len());
        let results = executor.execute_batch(chunk).await?;
        
        // Update completed count
        completed_tasks += results.len();
        
        // Report overall progress to the frontend
        let progress_percentage = (completed_tasks as f64 / total_tasks as f64 * 100.0) as u32;
        let progress_update = serde_json::json!({
            "completed": completed_tasks,
            "total": total_tasks,
            "percentage": progress_percentage,
            "status": "processing"
        });
        
        // Send progress update
        let _ = app.emit("batch-progress", progress_update);
        
        all_results.extend(results);
        debug!("Completed chunk {}/{} - Overall progress: {}% ({}/{})",
            i + 1, chunks.len(), 
            ((i + 1) * 100) / chunks.len(),
            (i + 1) * chunk.len().min(CHUNK_SIZE),
            task_count
        );
    }
    
    // Send final progress update
    let final_progress = serde_json::json!({
        "completed": total_tasks,
        "total": total_tasks,
        "percentage": 100,
        "status": "complete"
    });
    let _ = app.emit("batch-progress", final_progress);
    
    debug!("All chunks processed, returning {} results", all_results.len());
    Ok(all_results)
}

/// Benchmarks optimization performance on a set of images.
///
/// Runs the same optimization pipeline as [`optimize_images`] but redirects
/// all output to a temporary directory, times the entire batch, and returns
/// throughput statistics without touching the original output paths.
///
/// # Arguments
/// * `state` - Application state containing the executor
/// * `tasks` - Vector of image tasks whose input files will be benchmarked
///
/// # Returns
/// A [`BenchmarkResult`] with total time, per-image average, throughput, and byte counts.
#[tauri::command]
pub async fn benchmark_optimization(
    state: State<'_, AppState>,
    tasks: Vec<ImageTask>,
) -> OptimizerResult<BenchmarkResult> {
    if tasks.is_empty() {
        return Err(OptimizerError::processing("No tasks provided for benchmark"));
    }

    let task_count = tasks.len();
    debug!("Starting benchmark for {} images", task_count);

    // Temp directory scoped to this benchmark run
    let bench_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let temp_dir = std::env::temp_dir().join(format!("image-optimizer-benchmark-{bench_id}"));
    tokio::fs::create_dir_all(&temp_dir).await?;

    // Validate inputs and remap outputs to temp paths
    let mut bench_tasks: Vec<ImageTask> = Vec::with_capacity(task_count);
    for task in &tasks {
        validate_input_path(&task.input_path).await?;

        let filename = Path::new(&task.input_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        bench_tasks.push(ImageTask {
            input_path: task.input_path.clone(),
            output_path: temp_dir.join(&filename).to_string_lossy().to_string(),
            settings: task.settings.clone(),
        });
    }

    // Measure total input size before processing
    let total_input_bytes: u64 = bench_tasks
        .iter()
        .filter_map(|t| std::fs::metadata(&t.input_path).ok())
        .map(|m| m.len())
        .sum();

    // Time the full batch execution (single executor call, matching real-world usage)
    let executor = state.create_executor();
    let start = std::time::Instant::now();
    let results = executor.execute_batch(&bench_tasks).await?;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    // Collect output sizes from results
    let total_output_bytes: u64 = results.iter().map(|r| r.optimized_size).sum();

    // Clean up temp directory (best effort)
    let _ = tokio::fs::remove_dir_all(&temp_dir).await;

    let avg_per_image_ms = elapsed_ms as f64 / task_count as f64;
    let throughput_images_per_sec = if elapsed_ms > 0 {
        task_count as f64 / (elapsed_ms as f64 / 1000.0)
    } else {
        f64::INFINITY
    };

    debug!(
        "Benchmark complete: {} images in {}ms ({:.2} img/s, {:.1} ms/img)",
        task_count, elapsed_ms, throughput_images_per_sec, avg_per_image_ms
    );

    Ok(BenchmarkResult {
        total_time_ms: elapsed_ms,
        avg_per_image_ms,
        throughput_images_per_sec,
        image_count: task_count,
        total_input_bytes,
        total_output_bytes,
    })
}
