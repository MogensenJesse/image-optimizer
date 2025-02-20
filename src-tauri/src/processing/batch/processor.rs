use crate::core::{ImageTask, OptimizationResult};
use crate::utils::OptimizerResult;
use crate::processing::pool::ProcessPool;
use crate::processing::sharp::SharpExecutor;
use crate::benchmarking::metrics::{BenchmarkMetrics, Duration, Benchmarkable};
use tauri::AppHandle;
use tauri::Emitter;
use tracing::{debug, warn};
use serde::Serialize;
use std::time::Instant;

/// Represents the progress of a batch processing operation
#[derive(Debug, Clone, Serialize)]
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
    app_handle: AppHandle,
    metrics: Option<BenchmarkMetrics>,
}

impl BatchProcessor {
    /// Creates a new BatchProcessor with a fixed chunk size of 75
    pub async fn new(pool: ProcessPool, app_handle: AppHandle) -> Self {
        const CHUNK_SIZE: usize = 75;
        debug!("Creating BatchProcessor with chunk size of {}", CHUNK_SIZE);
        pool.set_batch_size(CHUNK_SIZE).await;
        Self {
            chunk_size: CHUNK_SIZE,
            pool,
            app_handle,
            metrics: None,
        }
    }

    /// Initializes benchmarking if enabled
    pub async fn init_benchmarking(&mut self, task_count: usize) {
        if self.pool.is_benchmark_mode().await {
            let mut metrics = BenchmarkMetrics::new(task_count);
            metrics.reset();
            metrics.start_benchmarking();
            self.metrics = Some(metrics);
        }
    }

    /// Processes a single chunk of tasks
    async fn process_chunk(&mut self, chunk: Vec<ImageTask>) -> OptimizerResult<Vec<OptimizationResult>> {
        debug!("Processing chunk of {} tasks", chunk.len());
        let start_time = Instant::now();
        let executor = SharpExecutor::new(&self.pool);
        let (results, executor_metrics) = executor.execute_batch(&chunk).await?;

        // Record metrics if in benchmark mode
        if let Some(metrics) = &mut self.metrics {
            let duration = Duration::new_unchecked(start_time.elapsed().as_secs_f64());
            metrics.add_processing_time(duration);
            
            // Record compression metrics
            for result in &results {
                metrics.record_compression(result.original_size, result.optimized_size);
            }

            // Record batch metrics
            metrics.record_batch(chunk.len());

            // Record pool metrics - prefer executor metrics if available
            if let Some(worker_metrics) = executor_metrics {
                metrics.record_pool_metrics(
                    worker_metrics.active_workers,
                    worker_metrics.total_tasks
                );
            } else {
                // Fallback to collecting current metrics
                let worker_metrics = self.pool.collect_worker_metrics().await;
                metrics.record_pool_metrics(
                    worker_metrics.active_workers,
                    worker_metrics.total_tasks
                );
            }

            // Update worker count in metrics
            if let Some(worker_pool) = &mut metrics.worker_pool {
                worker_pool.worker_count = self.pool.get_max_size();
                // For tasks_per_worker, we'll use the actual active workers
                let active_workers = worker_pool.active_workers;
                if active_workers > 0 {
                    // Distribute tasks evenly among active workers
                    let base_tasks = chunk.len() / active_workers;
                    let extra_tasks = chunk.len() % active_workers;
                    
                    // Create distribution vector
                    let mut distribution = vec![base_tasks; self.pool.get_max_size()];
                    // Add extra tasks to the first few workers
                    for i in 0..extra_tasks {
                        distribution[i] += 1;
                    }
                    worker_pool.tasks_per_worker = distribution;
                } else {
                    worker_pool.tasks_per_worker = vec![0; self.pool.get_max_size()];
                }
            }
        }

        Ok(results)
    }

    /// Processes a batch of tasks with progress tracking and error handling
    pub async fn process_batch(
        &mut self,
        tasks: Vec<ImageTask>,
    ) -> OptimizerResult<Vec<OptimizationResult>> {
        let total_tasks = tasks.len();
        debug!("Starting batch processing of {} tasks", total_tasks);
        
        // Initialize benchmarking if enabled
        self.init_benchmarking(total_tasks).await;
        
        // Enqueue all tasks
        for task in tasks {
            self.pool.enqueue_task(task).await;
        }
        
        let mut processed_count = 0;
        let mut all_results = Vec::new();
        let mut failed_tasks = Vec::new();
        
        // Process tasks in chunks from the queue
        while let Some(chunk) = self.pool.dequeue_chunk().await {
            let chunk_index = processed_count / self.chunk_size;
            debug!("Processing chunk {}/{}", chunk_index + 1, (total_tasks + self.chunk_size - 1) / self.chunk_size);
            
            match self.process_chunk(chunk.clone()).await {
                Ok(results) => {
                    processed_count += results.len();
                    all_results.extend(results);
                },
                Err(e) => {
                    warn!("Failed to process chunk {}: {}", chunk_index + 1, e);
                    failed_tasks.extend(chunk.iter().map(|task| (task.input_path.clone(), e.to_string())));
                    processed_count += chunk.len();
                }
            }
            
            // Emit progress event
            let progress = BatchProgress {
                total_files: total_tasks,
                processed_files: processed_count,
                current_chunk: chunk_index + 1,
                total_chunks: (total_tasks + self.chunk_size - 1) / self.chunk_size,
                failed_tasks: failed_tasks.clone(),
            };

            if let Err(e) = self.app_handle.emit_to("main", "optimization_progress", progress) {
                warn!("Failed to emit progress event: {}", e);
            }
        }
        
        if !failed_tasks.is_empty() {
            warn!(
                "Batch processing completed with {} failed tasks out of {}",
                failed_tasks.len(),
                total_tasks
            );
        } else {
            debug!("Batch processing completed successfully. Processed {} files", processed_count);
        }
        
        Ok(all_results)
    }

    /// Gets the collected benchmark metrics, if any
    pub fn take_metrics(&mut self) -> Option<BenchmarkMetrics> {
        self.metrics.take()
    }
} 