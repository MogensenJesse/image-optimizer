use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tauri::AppHandle;
use crate::core::OptimizationResult;
use crate::worker::ImageTask;
use crate::processing::ImageOptimizer;
use crate::utils::{OptimizerError, OptimizerResult};
use tracing::{debug, warn};

const DEFAULT_WORKERS: usize = 4;

#[derive(Clone)]
pub struct WorkerPool {
    optimizer: ImageOptimizer,
    app: AppHandle,
    active_workers: Arc<Mutex<usize>>,
    semaphore: Arc<Semaphore>,
    worker_count: usize,
}

impl WorkerPool {
    pub fn new(app: AppHandle, worker_count: Option<usize>) -> Self {
        let worker_count = worker_count.unwrap_or(DEFAULT_WORKERS);
        Self {
            optimizer: ImageOptimizer::new(),
            app,
            active_workers: Arc::new(Mutex::new(0)),
            semaphore: Arc::new(Semaphore::new(worker_count)),
            worker_count,
        }
    }

    pub async fn process(&self, task: ImageTask) -> OptimizerResult<OptimizationResult> {
        debug!("Acquiring semaphore for task: {}", task.input_path);
        let _permit = self.semaphore.acquire().await.map_err(|e| {
            warn!("Failed to acquire semaphore: {}", e);
            OptimizerError::worker(format!("Failed to acquire worker: {}", e))
        })?;

        let mut count = self.active_workers.lock().await;
        *count += 1;
        let current_workers = *count;
        let available_permits = self.semaphore.available_permits();
        debug!(
            "Worker started - Active: {}/{}, Available permits: {}, Task: {}", 
            current_workers, self.worker_count, available_permits, task.input_path
        );

        // Process single task as a batch of one
        let result = self.optimizer.process_batch(&self.app, vec![task]).await?;
        let result = result.into_iter().next().ok_or_else(|| {
            OptimizerError::worker("No result returned from batch processing".to_string())
        })?;
        
        *count -= 1;
        debug!(
            "Worker finished - Active: {}/{}, Available permits: {}", 
            count.saturating_sub(1), self.worker_count, available_permits + 1
        );
        
        Ok(result)
    }

    pub async fn process_batch(&self, tasks: Vec<ImageTask>) -> OptimizerResult<Vec<OptimizationResult>> {
        debug!("Processing batch of {} tasks", tasks.len());
        
        // Process tasks in batches using the optimizer's batch processing
        self.optimizer.process_batch(&self.app, tasks).await
    }

    pub async fn get_active_workers(&self) -> usize {
        *self.active_workers.lock().await
    }
} 