use serde::{Deserialize, Serialize};
use crate::worker_pool::{WorkerPool, WorkerMetrics, ImageTask};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Emitter;
use sysinfo::*;

fn get_optimal_worker_count() -> usize {
    let sys = System::new_all();
    let cpu_count = num_cpus::get();
    let total_memory_mb = sys.total_memory() / 1024;
    
    // Get CPU frequency and brand
    let cpus = sys.cpus();
    let avg_freq = cpus.iter()
        .map(|cpu| cpu.frequency())
        .sum::<u64>() / cpus.len() as u64;

    // Adjust worker count based on system specs
    let base_workers = if avg_freq > 3000 { // 3GHz
        cpu_count.min(6)
    } else {
        cpu_count.min(4)
    };

    // Further adjust based on available memory
    if total_memory_mb < 8192 { // Less than 8GB
        base_workers.min(2)
    } else if total_memory_mb < 16384 { // Less than 16GB
        base_workers.min(4)
    } else {
        base_workers
    }
}

lazy_static::lazy_static! {
    static ref WORKER_POOL: Arc<Mutex<Option<WorkerPool>>> = Arc::new(Mutex::new(None));
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OptimizationResult {
    pub path: String,
    #[serde(rename = "originalSize")]
    pub original_size: u64,
    #[serde(rename = "optimizedSize")]
    pub optimized_size: u64,
    #[serde(rename = "savedBytes")]
    pub saved_bytes: i64,
    #[serde(rename = "compressionRatio")]
    pub compression_ratio: String,
    pub format: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResizeSettings {
    width: Option<u32>,
    height: Option<u32>,
    #[serde(rename = "maintainAspect")]
    maintain_aspect: bool,
    mode: String,
    size: Option<u32>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QualitySettings {
    global: u32,
    jpeg: Option<u32>,
    png: Option<u32>,
    webp: Option<u32>,
    avif: Option<u32>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
            let num_workers = get_optimal_worker_count();
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

    let app_handle = app.clone();
    pool.process_batch(tasks, move |progress| {
        let _ = app_handle.emit("optimization_progress", progress);
    }).await
}

#[tauri::command]
pub async fn get_worker_metrics() -> Result<Vec<WorkerMetrics>, String> {
    let pool_guard = WORKER_POOL.lock().await;
    if let Some(pool) = pool_guard.as_ref() {
        Ok(pool.get_metrics().await)
    } else {
        Ok(Vec::new())
    }
}
