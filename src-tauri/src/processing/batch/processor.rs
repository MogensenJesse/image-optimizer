use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;
use crate::core::OptimizationResult;
use crate::worker::ImageTask;
use crate::utils::{OptimizerResult, validation::validate_task};
use super::config::BatchSizeConfig;
use crate::processing::{pool::ProcessPool, sharp::SharpExecutor};

pub struct BatchProcessor<'a> {
    pool: &'a ProcessPool,
    config: BatchSizeConfig,
    active_tasks: Arc<Mutex<HashSet<String>>>,
}

impl<'a> BatchProcessor<'a> {
    pub fn new(pool: &'a ProcessPool, active_tasks: Arc<Mutex<HashSet<String>>>) -> Self {
        Self {
            pool,
            config: BatchSizeConfig::default(),
            active_tasks,
        }
    }

    pub async fn process(&self, tasks: Vec<ImageTask>) 
        -> OptimizerResult<Vec<OptimizationResult>> {
        let batch_size = self.calculate_batch_size(&tasks);
        let pool_size = self.pool.get_max_size();
        
        // Adjust batch size based on pool size to optimize process utilization
        let adjusted_batch_size = (batch_size / pool_size).max(1) * pool_size;
        debug!("Calculated optimal batch size: {} (adjusted from {} for {} processes)", 
            adjusted_batch_size, batch_size, pool_size);
        
        let mut results = Vec::with_capacity(tasks.len());
        
        // Get initial process metrics for benchmarking
        let process_metrics = self.pool.get_metrics().await;
        debug!("Initial process pool metrics - Active: {}, Total Spawns: {}", 
            process_metrics.active_processes.last().unwrap_or(&0),
            process_metrics.total_spawns
        );
        
        for (chunk_index, chunk) in tasks.chunks(adjusted_batch_size).enumerate() {
            debug!(
                "Processing chunk {}/{} ({} tasks)", 
                chunk_index + 1, 
                (tasks.len() + adjusted_batch_size - 1) / adjusted_batch_size,
                chunk.len()
            );
            
            let chunk_results = match self.process_chunk(chunk.to_vec()).await {
                Ok(res) => {
                    debug!("Chunk {} processed successfully", chunk_index + 1);
                    res
                },
                Err(e) => {
                    debug!("Chunk {} failed: {}", chunk_index + 1, e);
                    return Err(e);
                }
            };
            
            results.extend(chunk_results);
        }
        
        // Get final process metrics for benchmarking
        let final_metrics = self.pool.get_metrics().await;
        debug!("Final process pool metrics - Active: {}, Total Spawns: {}", 
            final_metrics.active_processes.last().unwrap_or(&0),
            final_metrics.total_spawns
        );
        
        Ok(results)
    }

    async fn process_chunk(&self, tasks: Vec<ImageTask>) 
        -> OptimizerResult<Vec<OptimizationResult>> {
        debug!("Validating {} tasks in chunk", tasks.len());
        // Validate all tasks in parallel
        self.validate_tasks(&tasks).await?;
        
        // Track active tasks
        {
            let mut active = self.active_tasks.lock().await;
            for task in &tasks {
                active.insert(task.input_path.clone());
            }
            debug!("Tracked {} tasks for processing", tasks.len());
        }

        // Process using Sharp executor
        let executor = SharpExecutor::new(self.pool);
        let results = executor.execute_batch(&tasks).await?;

        // Remove from active tasks
        {
            let mut active = self.active_tasks.lock().await;
            for task in &tasks {
                active.remove(&task.input_path);
            }
        }

        Ok(results)
    }

    async fn validate_tasks(&self, tasks: &[ImageTask]) -> OptimizerResult<()> {
        const VALIDATION_CHUNK_SIZE: usize = 50;
        
        for chunk in tasks.chunks(VALIDATION_CHUNK_SIZE) {
            let futures: Vec<_> = chunk.iter()
                .map(|task| validate_task(task))
                .collect();
            futures::future::try_join_all(futures).await?;
        }
        
        debug!("All {} tasks validated successfully", tasks.len());
        Ok(())
    }

    fn calculate_batch_size(&self, #[allow(unused_variables)] tasks: &[ImageTask]) -> usize {
        let process_count = self.pool.get_max_size();
        
        // Calculate optimal batch size based on process count
        let process_based_size = self.config.tasks_per_process * process_count;
        
        // Use the most conservative size that meets our constraints
        process_based_size
            .min(self.config.max_size)
            .max(self.config.min_size)
    }
} 