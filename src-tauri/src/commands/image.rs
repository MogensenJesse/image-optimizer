use serde::Deserialize;
use tauri::State;
use tauri::Emitter;
use tracing::debug;
use crate::core::{AppState, ImageSettings, OptimizationResult};
use crate::core::ImageTask;
use crate::utils::{OptimizerResult, validate_task};

#[derive(Debug, Deserialize)]
pub struct BatchImageTask {
    pub input_path: String,
    pub output_path: String,
    pub settings: ImageSettings,
}

#[tauri::command]
pub async fn get_active_tasks(
    _app: tauri::AppHandle,
    _state: State<'_, AppState>,
) -> OptimizerResult<Vec<String>> {
    // Without a process pool, we don't track active tasks anymore
    // Just return an empty vector
    Ok(Vec::new())
}

#[tauri::command]
pub async fn optimize_image(
    _app: tauri::AppHandle,
    state: State<'_, AppState>,
    input_path: String,
    output_path: String,
    settings: ImageSettings,
) -> OptimizerResult<OptimizationResult> {
    debug!("Received optimize_image command for: {}", input_path);
    
    let task = ImageTask {
        input_path,
        output_path,
        settings,
    };

    // Validate task
    validate_task(&task).await?;

    // Create executor and process the image
    let executor = state.create_executor();
    
    debug!("Starting image optimization");
    let results = executor.execute_batch(&[task]).await?;
    debug!("Image optimization completed");
    
    // Return the single result
    Ok(results.into_iter().next().unwrap())
}

#[tauri::command]
pub async fn optimize_images(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    tasks: Vec<BatchImageTask>,
) -> OptimizerResult<Vec<OptimizationResult>> {
    let task_count = tasks.len();
    debug!("Received optimize_images command for {} images", task_count);
    
    let mut image_tasks = Vec::with_capacity(task_count);
    
    // Convert and validate tasks
    for task in tasks {
        let image_task = ImageTask {
            input_path: task.input_path,
            output_path: task.output_path,
            settings: task.settings,
        };

        // Validate task
        validate_task(&image_task).await?;
        image_tasks.push(image_task);
    }

    // Process in chunks to avoid overwhelming the system
    // Increased from 75 to 500 now that we're using memory-mapped files
    // and no longer limited by command line length
    const CHUNK_SIZE: usize = 500;
    let chunks: Vec<_> = image_tasks.chunks(CHUNK_SIZE).collect();
    debug!("Processing {} images in {} chunks of size {}", task_count, chunks.len(), CHUNK_SIZE);
    
    let mut all_results = Vec::with_capacity(image_tasks.len());
    
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
