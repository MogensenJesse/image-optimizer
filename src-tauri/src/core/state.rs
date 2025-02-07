use std::sync::Arc;
use tokio::sync::Mutex;
use crate::processing::ProcessPool;
use tracing::{info, warn};
use crate::utils::OptimizerError;

lazy_static::lazy_static! {
    pub(crate) static ref PROCESS_POOL: Arc<Mutex<Option<ProcessPool>>> = Arc::new(Mutex::new(None));
}

#[derive(Clone)]
pub struct AppState {
    pub(crate) process_pool: Arc<Mutex<Option<ProcessPool>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            process_pool: Arc::clone(&PROCESS_POOL),
        }
    }

    pub async fn get_or_init_process_pool(&self, app: tauri::AppHandle) -> Result<ProcessPool, OptimizerError> {
        let mut pool = self.process_pool.lock().await;
        if pool.is_none() {
            let new_pool = ProcessPool::new(app);
            // Warm up the pool
            new_pool.warmup().await?;
            *pool = Some(new_pool);
        }
        Ok(pool.as_ref().unwrap().clone())
    }

    /// Attempt to gracefully shutdown
    pub async fn shutdown(&self) {
        info!("Initiating AppState shutdown");
        if let Ok(mut pool) = self.process_pool.try_lock() {
            if pool.take().is_some() {
                info!("Process pool shutdown complete");
            }
        } else {
            warn!("Could not acquire lock for process pool during shutdown");
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