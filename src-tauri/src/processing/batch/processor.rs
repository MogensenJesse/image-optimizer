use crate::core::{ImageTask, OptimizationResult};
use crate::utils::OptimizerResult;
use crate::processing::pool::ProcessPool;
use crate::processing::sharp::SharpExecutor;
use tracing::{debug, warn, info};

/// Represents the progress of a batch processing operation
#[derive(Debug, Clone)]
pub struct BatchProgress {
    pub total_files: usize,
    pub processed_files: usize,
    pub current_chunk: usize,
    pub total_chunks: usize,
    pub failed_tasks: Vec<(String, String)>, // (file_path, error_message)
}

/// Handles batch processing of image optimization tasks
pub struct BatchProcessor {
    chunk_size: usize,
    pool: ProcessPool,
}

impl BatchProcessor {
    /// Creates a new BatchProcessor with a fixed chunk size of 75
    pub async fn new(pool: ProcessPool) -> Self {
        const CHUNK_SIZE: usize = 75;
        debug!("Creating BatchProcessor with chunk size of {}", CHUNK_SIZE);
        pool.set_batch_size(CHUNK_SIZE).await;
        Self {
            chunk_size: CHUNK_SIZE,
            pool,
        }
    }

    /// Creates chunks of tasks for batch processing
    fn create_chunks(&self, tasks: Vec<ImageTask>) -> Vec<Vec<ImageTask>> {
        tasks.chunks(self.chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect()
    }

    /// Processes a single chunk of tasks
    async fn process_chunk(&self, chunk: Vec<ImageTask>) -> OptimizerResult<Vec<OptimizationResult>> {
        // Let the executor log the details
        let executor = SharpExecutor::new(&self.pool);
        executor.execute_batch(&chunk).await
    }

    /// Processes a batch of tasks with progress tracking and error handling
    pub async fn process_batch(
        &self,
        tasks: Vec<ImageTask>,
        progress_callback: impl Fn(BatchProgress) + Send + 'static,
    ) -> OptimizerResult<Vec<OptimizationResult>> {
        let total_tasks = tasks.len();
        info!("Processing batch of {} tasks in {} chunks", 
            total_tasks, 
            (total_tasks + self.chunk_size - 1) / self.chunk_size
        );
        
        let chunks = self.create_chunks(tasks);
        let mut processed_count = 0;
        let mut all_results = Vec::new();
        let mut failed_tasks = Vec::new();
        
        for (chunk_index, chunk) in chunks.into_iter().enumerate() {
            // Only log at important milestones to reduce noise
            if chunk_index == 0 || chunk_index == chunks.len() - 1 || chunk_index % 5 == 0 {
                debug!("Processing chunk {}/{}", chunk_index + 1, chunks.len());
            }
            
            match self.process_chunk(chunk.clone()).await {
                Ok(results) => {
                    processed_count += results.len();
                    all_results.extend(results);
                },
                Err(e) => {
                    warn!("Failed to process chunk {}: {}", chunk_index + 1, e);
                    // Store failed tasks for reporting
                    failed_tasks.extend(chunk.iter().map(|task| (task.input_path.clone(), e.to_string())));
                    processed_count += chunk.len();
                }
            }
            
            // Update progress
            progress_callback(BatchProgress {
                total_files: total_tasks,
                processed_files: processed_count,
                current_chunk: chunk_index + 1,
                total_chunks: chunks.len(),
                failed_tasks: failed_tasks.clone(),
            });
        }
        
        if !failed_tasks.is_empty() {
            warn!(
                "Batch processing completed with {} failed tasks out of {}",
                failed_tasks.len(),
                total_tasks
            );
        } else {
            info!("Batch processing completed successfully: {} files processed", processed_count);
        }
        
        Ok(all_results)
    }
} 