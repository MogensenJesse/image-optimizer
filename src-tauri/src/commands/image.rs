use serde::{Deserialize, Serialize};
use tauri_plugin_shell::ShellExt;
use crate::worker_pool::{WorkerPool, ImageTask};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::env;

lazy_static::lazy_static! {
    static ref WORKER_POOL: Arc<Mutex<Option<WorkerPool>>> = Arc::new(Mutex::new(None));
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OptimizationResult {
    path: String,
    #[serde(rename = "originalSize")]
    original_size: u64,
    #[serde(rename = "optimizedSize")]
    optimized_size: u64,
    #[serde(rename = "savedBytes")]
    saved_bytes: i64,
    #[serde(rename = "compressionRatio")]
    compression_ratio: String,
    format: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResizeSettings {
    width: Option<u32>,
    height: Option<u32>,
    #[serde(rename = "maintainAspect")]
    maintain_aspect: bool,
    mode: String,
    size: Option<u32>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QualitySettings {
    global: u32,
    jpeg: Option<u32>,
    png: Option<u32>,
    webp: Option<u32>,
    avif: Option<u32>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageSettings {
    quality: QualitySettings,
    resize: ResizeSettings,
    #[serde(rename = "outputFormat")]
    output_format: String
}

#[tauri::command]
pub async fn optimize_image(
    app: tauri::AppHandle,
    input_path: String,
    output_path: String,
    settings: ImageSettings,
) -> Result<OptimizationResult, String> {
    let pool = {
        let mut pool_guard = WORKER_POOL.lock().await;
        if pool_guard.is_none() {
            let num_workers = num_cpus::get().min(4); // Cap at 4 workers
            *pool_guard = Some(WorkerPool::new(num_workers, app.clone()));
        }
        pool_guard.as_mut().unwrap().clone()
    };

    let task = ImageTask {
        input_path,
        output_path,
        settings,
        priority: 0,
    };

    pool.process(task).await
}

#[tauri::command]
pub async fn get_active_tasks() -> Result<usize, String> {
    let pool_guard = WORKER_POOL.lock().await;
    if let Some(pool) = pool_guard.as_ref() {
        Ok(pool.active_tasks().await)
    } else {
        Ok(0)
    }
}

#[tauri::command]
pub async fn optimize_images(
    app: tauri::AppHandle,
    tasks: Vec<(String, String, ImageSettings)>,
) -> Result<Vec<OptimizationResult>, String> {
    let pool = {
        let mut pool_guard = WORKER_POOL.lock().await;
        if pool_guard.is_none() {
            let num_workers = num_cpus::get().min(4);
            *pool_guard = Some(WorkerPool::new(num_workers, app.clone()));
        }
        pool_guard.as_mut().unwrap().clone()
    };

    let tasks = tasks.into_iter()
        .map(|(input, output, settings)| ImageTask {
            input_path: input,
            output_path: output,
            settings,
            priority: 0,
        })
        .collect();

    pool.process_batch(tasks).await
}
