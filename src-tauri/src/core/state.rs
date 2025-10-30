use std::sync::Arc;
use crate::processing::sharp::MemoryMapExecutor;
use tracing::debug;
use crate::utils::OptimizerError;

#[derive(Clone)]
pub struct AppState {
    app_handle: Arc<tauri::AppHandle>,
}

impl AppState {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            app_handle: Arc::new(app),
        }
    }

    pub fn create_executor(&self) -> MemoryMapExecutor {
        MemoryMapExecutor::new((*self.app_handle).clone())
    }

    /// Initialize and warm up the executor
    /// This reduces the cold start penalty for the first real task
    pub async fn warmup_executor(&self) -> Result<(), OptimizerError> {
        debug!("Initializing and warming up executor...");
        
        // Create and warm up the executor
        let executor = self.create_executor();
        executor.warmup().await?;
        
        debug!("Executor warmup completed successfully");
        Ok(())
    }
} 