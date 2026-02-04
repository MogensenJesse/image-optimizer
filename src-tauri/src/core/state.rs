//! Application state management for Tauri.

use std::sync::Arc;
use crate::processing::sharp::MemoryMapExecutor;
use tracing::debug;
use crate::utils::OptimizerError;

/// Application state managed by Tauri.
///
/// Holds the app handle and provides methods to create executors
/// for image processing operations.
#[derive(Clone)]
pub struct AppState {
    app_handle: Arc<tauri::AppHandle>,
}

impl AppState {
    /// Creates a new application state with the given Tauri app handle.
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            app_handle: Arc::new(app),
        }
    }

    /// Creates a new memory-mapped executor for batch processing.
    ///
    /// Each call creates a fresh executor instance that can process
    /// a batch of image optimization tasks.
    pub fn create_executor(&self) -> MemoryMapExecutor {
        MemoryMapExecutor::new((*self.app_handle).clone())
    }

    /// Warms up the executor to reduce cold start latency.
    ///
    /// This processes a minimal dummy image to initialize the Sharp sidecar
    /// and its dependencies before real tasks are submitted.
    pub async fn warmup_executor(&self) -> Result<(), OptimizerError> {
        debug!("Initializing and warming up executor...");
        
        // Create and warm up the executor
        let executor = self.create_executor();
        executor.warmup().await?;
        
        debug!("Executor warmup completed successfully");
        Ok(())
    }
} 