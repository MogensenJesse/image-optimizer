use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri_plugin_shell::ShellExt;
use crate::core::OptimizationResult;
use crate::worker::ImageTask;
use serde_json;

#[derive(Clone)]
pub struct ImageOptimizer {
    active_tasks: Arc<Mutex<Vec<String>>>,
}

impl ImageOptimizer {
    pub fn new() -> Self {
        Self {
            active_tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn process_image(
        &self,
        app: &tauri::AppHandle,
        task: ImageTask,
    ) -> Result<OptimizationResult, String> {
        let input_path = Path::new(&task.input_path);
        let output_path = Path::new(&task.output_path);

        // Validate paths
        if !input_path.exists() {
            return Err(format!("Input file does not exist: {}", task.input_path));
        }
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| format!("Failed to create output directory: {}", e))?;
            }
        }

        // Get original file size
        let original_size = tokio::fs::metadata(&task.input_path)
            .await
            .map_err(|e| format!("Failed to get original file size: {}", e))?
            .len();

        // Track active task
        {
            let mut tasks = self.active_tasks.lock().await;
            tasks.push(task.input_path.clone());
        }

        // Process image using Sharp sidecar
        let result = self.run_sharp_process(app, &task).await;

        // Remove from active tasks
        {
            let mut tasks = self.active_tasks.lock().await;
            if let Some(pos) = tasks.iter().position(|x| x == &task.input_path) {
                tasks.remove(pos);
            }
        }

        // Get optimized file size and create result
        match result {
            Ok(_) => {
                let optimized_size = tokio::fs::metadata(&task.output_path)
                    .await
                    .map_err(|e| format!("Failed to get optimized file size: {}", e))?
                    .len();

                // Calculate metrics
                let bytes_saved = original_size as i64 - optimized_size as i64;
                let compression_ratio = if original_size > 0 {
                    (bytes_saved as f64 / original_size as f64) * 100.0
                } else {
                    0.0
                };

                Ok(OptimizationResult {
                    original_path: task.input_path,
                    optimized_path: task.output_path,
                    original_size,
                    optimized_size,
                    success: true,
                    error: None,
                    saved_bytes: bytes_saved,
                    compression_ratio,
                })
            }
            Err(e) => Err(e),
        }
    }

    async fn run_sharp_process(&self, app: &tauri::AppHandle, task: &ImageTask) -> Result<(), String> {
        let command = app.shell()
            .sidecar("sharp-sidecar")
            .map_err(|e| format!("Failed to create Sharp sidecar: {}", e))?;

        // Serialize settings to JSON
        let settings_json = serde_json::to_string(&task.settings)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        let output = command
            .args(&[
                "optimize",
                &task.input_path,
                &task.output_path,
                &settings_json,
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to run Sharp process: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Sharp process failed: {}", stderr))
        } else {
            Ok(())
        }
    }

    pub async fn get_active_tasks(&self) -> Vec<String> {
        self.active_tasks.lock().await.clone()
    }
} 