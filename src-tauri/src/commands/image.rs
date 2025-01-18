use serde::Deserialize;
use tauri::State;
use tracing::{info, debug};
use crate::core::{AppState, ImageSettings, OptimizationResult};
use crate::worker::ImageTask;
use crate::utils::{OptimizerResult, OptimizerError, validate_task};
use crate::worker::WorkerError;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct BatchImageTask {
    pub input_path: String,
    pub output_path: String,
    pub settings: ImageSettings,
}

#[tauri::command]
pub async fn optimize_image(
    app: tauri::AppHandle,
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

    // Get or initialize worker pool
    let pool = state.get_or_init_worker_pool(app).await?;
    let pool = Arc::try_unwrap(pool).unwrap_or_else(|arc| (*arc).clone());
    
    // Process image
    info!("Starting image optimization");
    let result = pool.process(task).await.map_err(|e| match e {
        WorkerError::OptimizerError(e) => e,
        e => OptimizerError::worker(e.to_string()),
    })?;
    debug!("Image optimization completed");
    
    Ok(result)
}

#[tauri::command]
pub async fn optimize_images(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    tasks: Vec<BatchImageTask>,
) -> OptimizerResult<Vec<OptimizationResult>> {
    info!("Received optimize_images command for {} images", tasks.len());
    let mut image_tasks = Vec::with_capacity(tasks.len());
    
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

    // Get or initialize worker pool and process images
    let pool = state.get_or_init_worker_pool(app).await?;
    let pool = Arc::try_unwrap(pool).unwrap_or_else(|arc| (*arc).clone());
    
    let (results, _duration) = pool.process_batch(image_tasks).await
        .map_err(|e| match e {
            WorkerError::OptimizerError(e) => e,
            e => OptimizerError::worker(e.to_string()),
        })?;
    Ok(results)
}
