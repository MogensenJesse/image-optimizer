use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use sysinfo::System;
use tracing::debug;
use crate::core::OptimizationResult;
use crate::worker::ImageTask;
use crate::utils::{OptimizerError, OptimizerResult, validation::validate_task};
use super::{config::BatchSizeConfig, metrics::BatchMemoryMetrics};
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
        -> OptimizerResult<(Vec<OptimizationResult>, BatchMemoryMetrics)> {
        let batch_size = self.calculate_batch_size(&tasks);
        let pool_size = self.pool.get_max_size();
        
        // Adjust batch size based on pool size to optimize process utilization
        let adjusted_batch_size = (batch_size / pool_size).max(1) * pool_size;
        debug!("Calculated optimal batch size: {} (adjusted from {} for {} processes)", 
            adjusted_batch_size, batch_size, pool_size);
        
        let mut results = Vec::with_capacity(tasks.len());
        let available_mem = self.get_available_memory();
        
        // Initialize memory metrics
        let mut memory_metrics = BatchMemoryMetrics::new(available_mem);
        
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
            
            // Record pre-processing memory state
            let pre_mem = self.get_available_memory();
            
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
            
            // Record post-processing memory state and calculate metrics
            let post_mem = self.get_available_memory();
            let used_memory = pre_mem.saturating_sub(post_mem);
            memory_metrics.record_usage(used_memory, pre_mem);
            
            results.extend(chunk_results);
        }
        
        // Get final process metrics for benchmarking
        let final_metrics = self.pool.get_metrics().await;
        debug!("Final process pool metrics - Active: {}, Total Spawns: {}", 
            final_metrics.active_processes.last().unwrap_or(&0),
            final_metrics.total_spawns
        );
        
        Ok((results, memory_metrics))
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
        debug!("Starting parallel validation of {} tasks", tasks.len());
        
        let validation_tasks: Vec<_> = tasks.iter()
            .map(|task| {
                let task = task.clone();
                tokio::spawn(async move {
                    validate_task(&task).await
                })
            })
            .collect();

        // Wait for all validations to complete
        let results = futures::future::try_join_all(validation_tasks).await
            .map_err(|e| OptimizerError::processing(format!("Task validation failed: {}", e)))?;

        // Check results and collect any errors
        let errors: Vec<_> = results
            .into_iter()
            .filter_map(|r| r.err())
            .collect();

        if !errors.is_empty() {
            debug!("Validation failed for {} tasks", errors.len());
            return Err(OptimizerError::processing(format!(
                "Validation failed for {} tasks: {:?}",
                errors.len(),
                errors
            )));
        }

        debug!("All {} tasks validated successfully", tasks.len());
        Ok(())
    }

    fn get_available_memory(&self) -> usize {
        let mut system = System::new();
        system.refresh_memory();
        system.available_memory() as usize
    }

    fn calculate_batch_size(&self, tasks: &[ImageTask]) -> usize {
        let process_count = self.pool.get_max_size();
        
        // Calculate total and average task size
        let total_size: u64 = tasks.iter()
            .filter_map(|t| std::fs::metadata(&t.input_path).ok())
            .map(|m| m.len())
            .sum();
        
        let avg_size = if tasks.is_empty() {
            0
        } else {
            total_size / tasks.len() as u64
        };
        
        // Get available system memory and calculate target
        let available_mem = self.get_available_memory();
        let memory_target = ((available_mem as f32 * self.config.target_memory_percentage) as usize)
            .min(self.config.target_memory_usage);
        
        // Calculate batch sizes based on different criteria
        let memory_based_size = if avg_size == 0 {
            self.config.max_size
        } else {
            memory_target / avg_size as usize
        };
        
        let process_based_size = self.config.tasks_per_process * process_count;
        
        // Use the most conservative size that meets our constraints
        memory_based_size
            .min(process_based_size)
            .min(self.config.max_size)
            .max(self.config.min_size)
    }
} 