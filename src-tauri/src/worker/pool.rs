use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tauri::AppHandle;
use crate::processing::ImageOptimizer;
use crate::worker::types::ImageTask;
use crate::core::OptimizationResult;
use num_cpus;
use tracing::{info, debug, warn};

pub struct WorkerPool {
    optimizer: ImageOptimizer,
    app: AppHandle,
    active_workers: Arc<Mutex<usize>>,
    semaphore: Arc<Semaphore>,
    worker_count: usize,
}

impl WorkerPool {
    pub fn new(app: AppHandle) -> Self {
        let cpu_count = num_cpus::get();
        let worker_count = cpu_count.max(2).min(8); // At least 2, at most 8 workers
        info!("Initializing worker pool - Available CPUs: {}, Worker count: {}", cpu_count, worker_count);
        
        Self {
            optimizer: ImageOptimizer::new(),
            app,
            active_workers: Arc::new(Mutex::new(0)),
            semaphore: Arc::new(Semaphore::new(worker_count)),
            worker_count,
        }
    }

    pub async fn process(&self, task: ImageTask) -> Result<OptimizationResult, String> {
        debug!("Acquiring semaphore for task: {}", task.input_path);
        let _permit = self.semaphore.acquire().await.map_err(|e| {
            warn!("Failed to acquire semaphore: {}", e);
            e.to_string()
        })?;

        let mut count = self.active_workers.lock().await;
        *count += 1;
        let current_workers = *count;
        let available_permits = self.semaphore.available_permits();
        debug!(
            "Worker started - Active: {}/{}, Available permits: {}, Task: {}", 
            current_workers, self.worker_count, available_permits, task.input_path
        );

        let result = self.optimizer.process_image(&self.app, task).await;
        
        *count -= 1;
        debug!(
            "Worker finished - Active: {}/{}, Available permits: {}", 
            count.saturating_sub(1), self.worker_count, available_permits + 1
        );
        
        result
    }

    pub async fn process_batch(&self, tasks: Vec<ImageTask>) -> Result<Vec<OptimizationResult>, String> {
        info!("Starting batch processing of {} tasks with {} workers", tasks.len(), self.worker_count);
        let task_count = tasks.len();
        let mut handles = Vec::with_capacity(task_count);
        
        // Spawn all tasks immediately
        for (index, task) in tasks.into_iter().enumerate() {
            debug!("Spawning task {}/{}: {}", index + 1, task_count, task.input_path);
            let self_clone = self.clone();
            handles.push(tokio::spawn(async move {
                self_clone.process(task).await
            }));
        }

        // Collect results in order of completion
        let mut results = Vec::with_capacity(task_count);
        let mut successful = 0;
        let mut failed = 0;

        for (index, handle) in handles.into_iter().enumerate() {
            match handle.await {
                Ok(Ok(result)) => {
                    debug!("Task {}/{} completed successfully", index + 1, task_count);
                    successful += 1;
                    results.push(result);
                }
                Ok(Err(e)) => {
                    warn!("Task {}/{} failed: {}", index + 1, task_count, e);
                    failed += 1;
                }
                Err(e) => {
                    warn!("Task {}/{} panicked: {}", index + 1, task_count, e);
                    failed += 1;
                }
            }
        }

        info!(
            "Batch processing completed - Total: {}, Successful: {}, Failed: {}", 
            successful + failed, successful, failed
        );

        Ok(results)
    }

    pub async fn get_active_workers(&self) -> usize {
        *self.active_workers.lock().await
    }
}

impl Clone for WorkerPool {
    fn clone(&self) -> Self {
        Self {
            optimizer: self.optimizer.clone(),
            app: self.app.clone(),
            active_workers: self.active_workers.clone(),
            semaphore: self.semaphore.clone(),
            worker_count: self.worker_count,
        }
    }
} 