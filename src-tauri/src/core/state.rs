use std::sync::Arc;
use tokio::sync::Mutex;
use crate::processing::sharp::DirectExecutor;
use tracing::debug;
use crate::utils::OptimizerError;

#[derive(Clone)]
pub struct AppState {
    pub(crate) app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            app_handle: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn set_app_handle(&self, app: tauri::AppHandle) {
        let mut handle = self.app_handle.lock().await;
        *handle = Some(app);
    }

    pub async fn get_app_handle(&self) -> Result<tauri::AppHandle, OptimizerError> {
        let handle = self.app_handle.lock().await;
        handle.clone().ok_or_else(|| OptimizerError::processing("App handle not initialized"))
    }

    pub async fn create_executor(&self) -> Result<DirectExecutor, OptimizerError> {
        let app = self.get_app_handle().await?;
        Ok(DirectExecutor::new(app))
    }

    /// Initialize and warm up the executor
    /// This reduces the cold start penalty for the first real task
    pub async fn warmup_executor(&self) -> Result<(), OptimizerError> {
        debug!("Initializing and warming up executor...");
        
        // Create and warm up the executor
        let executor = self.create_executor().await?;
        executor.warmup().await?;
        
        debug!("Executor warmup completed successfully");
        Ok(())
    }

    /// Attempt to gracefully shutdown
    pub async fn shutdown(&self) {
        debug!("Initiating AppState shutdown");
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        debug!("AppState is being dropped");
        
        // Create a new runtime for cleanup
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            self.shutdown().await;
        });
    }
} 