use serde::Deserialize;
use tauri::State;
use crate::core::{AppState, ImageSettings, OptimizationResult};
use crate::worker::ImageTask;
use crate::processing::ImageValidator;

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
) -> Result<OptimizationResult, String> {
    let task = ImageTask {
        input_path,
        output_path,
        settings,
    };

    // Validate task
    let validation = ImageValidator::validate_task(&task).await;
    if !validation.is_valid {
        let error = validation.error.unwrap_or_else(|| "Invalid task".to_string());
        return Err(error);
    }

    // Get or initialize worker pool
    let pool = state.get_or_init_worker_pool(app).await;
    
    // Process image
    pool.process(task).await
}

#[tauri::command]
pub async fn optimize_images(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    tasks: Vec<BatchImageTask>,
) -> Result<Vec<OptimizationResult>, String> {
    let mut image_tasks = Vec::with_capacity(tasks.len());
    
    // Convert and validate tasks
    for task in tasks {
        let image_task = ImageTask {
            input_path: task.input_path,
            output_path: task.output_path,
            settings: task.settings,
        };

        // Validate task
        let validation = ImageValidator::validate_task(&image_task).await;
        if !validation.is_valid {
            let error = validation.error.unwrap_or_else(|| "Invalid task".to_string());
            return Err(error);
        }

        image_tasks.push(image_task);
    }

    // Get or initialize worker pool
    let pool = state.get_or_init_worker_pool(app.clone()).await;
    
    // Process images in batch
    pool.process_batch(image_tasks).await
}
