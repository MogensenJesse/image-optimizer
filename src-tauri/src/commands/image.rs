use serde::Deserialize;
use tauri::State;
use tracing::{info, debug};
use crate::core::{AppState, ImageSettings, OptimizationResult};
use crate::worker::ImageTask;
use crate::utils::{OptimizerResult, validate_task};

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
    let pool = state.get_or_init_worker_pool(app).await;
    
    // Process image
    info!("Starting image optimization");
    let result = pool.process(task).await;
    debug!("Image optimization completed");
    
    result
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

    // Get or initialize worker pool and keep it alive
    let pool = state.get_or_init_worker_pool(app.clone()).await;
    
    // Process images in batch
    info!("Starting batch optimization of {} images", image_tasks.len());
    let results = pool.process_batch(image_tasks).await?;
    debug!("Batch optimization completed");
    
    Ok(results)
}
