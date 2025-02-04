use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashSet;
use crate::core::OptimizationResult;
use crate::worker::ImageTask;
use crate::utils::OptimizerResult;
use super::{pool::ProcessPool, batch::{BatchProcessor, BatchMemoryMetrics}};

#[derive(Clone)]
pub struct ImageOptimizer {
    active_tasks: Arc<Mutex<HashSet<String>>>,
    process_pool: ProcessPool,
}

impl ImageOptimizer {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            active_tasks: Arc::new(Mutex::new(HashSet::new())),
            process_pool: ProcessPool::new(app),
        }
    }

    pub async fn process_batch(&self, tasks: Vec<ImageTask>) 
        -> OptimizerResult<(Vec<OptimizationResult>, BatchMemoryMetrics)> {
        let processor = BatchProcessor::new(&self.process_pool, self.active_tasks.clone());
        processor.process(tasks).await
    }

    pub async fn get_active_tasks(&self) -> Vec<String> {
        self.active_tasks.lock().await.iter().cloned().collect()
    }
} 