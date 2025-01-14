use std::sync::Arc;
use tokio::sync::Mutex;
use crate::worker::WorkerPool;
use tracing::{info, warn};

lazy_static::lazy_static! {
    pub(crate) static ref WORKER_POOL: Arc<Mutex<Option<WorkerPool>>> = Arc::new(Mutex::new(None));
}

#[derive(Clone)]
pub struct AppState {
    pub(crate) worker_pool: Arc<Mutex<Option<WorkerPool>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            worker_pool: Arc::clone(&WORKER_POOL),
        }
    }

    pub fn get_worker_pool(&self) -> Option<WorkerPool> {
        if let Ok(pool) = self.worker_pool.try_lock() {
            pool.clone()
        } else {
            None
        }
    }

    pub async fn get_or_init_worker_pool(&self, app: tauri::AppHandle) -> Arc<WorkerPool> {
        let mut pool = self.worker_pool.lock().await;
        if pool.is_none() {
            *pool = Some(WorkerPool::new(app, None));
        }
        Arc::new(pool.as_ref().unwrap().clone())
    }

    /// Attempt to gracefully shutdown the worker pool
    pub async fn shutdown(&self) {
        info!("Initiating AppState shutdown");
        if let Ok(mut pool) = self.worker_pool.try_lock() {
            if let Some(worker_pool) = pool.take() {
                // Get active tasks before shutdown
                let active_count = worker_pool.get_active_workers().await;
                if active_count > 0 {
                    warn!("Shutting down with {} active tasks", active_count);
                }
                info!("Worker pool shutdown complete");
            }
        } else {
            warn!("Could not acquire lock for worker pool during shutdown");
        }
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        info!("AppState is being dropped");
        
        // Create a new runtime for cleanup
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            self.shutdown().await;
        });
    }
} 