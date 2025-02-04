use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashSet;
use crate::core::OptimizationResult;
use crate::worker::ImageTask;
use crate::utils::OptimizerResult;
use super::{pool::ProcessPool, batch::BatchProcessor};

#[derive(Clone)]
pub struct ImageOptimizer {
    active_tasks: Arc<Mutex<HashSet<String>>>,
    process_pool: ProcessPool,
}

impl ImageOptimizer {
    pub async fn new(app: tauri::AppHandle) -> OptimizerResult<Self> {
        let process_pool = ProcessPool::new(app.clone());
        
        // Create optimizer instance
        let optimizer = Self {
            active_tasks: Arc::new(Mutex::new(HashSet::new())),
            process_pool,
        };
        
        // Warm up the process pool
        optimizer.process_pool.warmup().await?;
        
        Ok(optimizer)
    }

    pub async fn process_batch(&self, tasks: Vec<ImageTask>) 
        -> OptimizerResult<Vec<OptimizationResult>> {
        let processor = BatchProcessor::new(&self.process_pool, self.active_tasks.clone());
        processor.process(tasks).await
    }

    pub async fn get_active_tasks(&self) -> Vec<String> {
        self.active_tasks.lock().await.iter().cloned().collect()
    }
} 