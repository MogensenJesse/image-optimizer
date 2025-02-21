use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::{Mutex, Semaphore};
use tauri_plugin_shell::{ShellExt, process::Command};
use crate::utils::{OptimizerError, OptimizerResult};
use crate::benchmarking::metrics::{Duration, BenchmarkMetrics, Benchmarkable};
use crate::benchmarking::reporter::BenchmarkReporter;
use crate::core::ImageTask;
use crate::processing::sharp::SharpExecutor;
use crate::core::OptimizationResult;
use tracing::{debug, info};
use num_cpus;
use std::time::Instant;

/// Task queue entry with timing information
#[derive(Debug)]
struct QueuedTask {
    task: ImageTask,
}

impl QueuedTask {
    fn new(task: ImageTask) -> Self {
        Self {
            task,
        }
    }
}

#[derive(Clone)]
pub struct ProcessPool {
    semaphore: Arc<Semaphore>,
    app: tauri::AppHandle,
    max_size: usize,
    batch_size: Arc<Mutex<usize>>,
    active_count: Arc<Mutex<usize>>,
    task_queue: Arc<Mutex<VecDeque<QueuedTask>>>,
    benchmark_mode: Arc<Mutex<bool>>,
}

impl ProcessPool {
    fn calculate_optimal_processes() -> usize {
        let cpu_count = num_cpus::get();
        // Use 90% of CPUs with no upper limit, minimum of 2 processes
        ((cpu_count * 9) / 10).max(2)
    }

    pub fn new(app: tauri::AppHandle) -> Self {
        let size = Self::calculate_optimal_processes();
        debug!("Creating process pool with {} processes (based on {} CPU cores)", size, num_cpus::get());
        Self::new_with_size(app, size)
    }

    pub fn new_with_size(app: tauri::AppHandle, size: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(size)),
            app,
            max_size: size,
            batch_size: Arc::new(Mutex::new(75)), // Default batch size
            active_count: Arc::new(Mutex::new(0)),
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
            benchmark_mode: Arc::new(Mutex::new(false)),
        }
    }

    /// Sets the batch size for task processing
    #[allow(dead_code)]  // False positive - used in BatchProcessor initialization
    pub async fn set_batch_size(&self, size: usize) {
        debug!("Setting batch size to {}", size);
        let mut batch_size = self.batch_size.lock().await;
        *batch_size = size;
    }
    
    /// Enqueues a task for processing
    pub async fn enqueue_task(&self, task: ImageTask) {
        let queued_task = QueuedTask::new(task);
        let mut queue = self.task_queue.lock().await;
        queue.push_back(queued_task);
    }
    
    /// Gets the current queue length
    pub async fn queue_length(&self) -> usize {
        self.task_queue.lock().await.len()
    }
    
    pub async fn acquire(&self) -> OptimizerResult<Command> {
        let _permit = self.semaphore.acquire().await.map_err(|e| 
            OptimizerError::sidecar(format!("Pool acquisition failed: {}", e))
        )?;
        
        // Update active count
        {
            let mut count = self.active_count.lock().await;
            *count += 1;
        }
        
        // Create the sidecar command
        self.app.shell()
            .sidecar("sharp-sidecar")
            .map_err(|e| OptimizerError::sidecar(format!("Sidecar spawn failed: {}", e)))
    }
    
    pub async fn release(&self) {
        let mut count = self.active_count.lock().await;
        *count = count.saturating_sub(1);
    }
    
    pub fn get_max_size(&self) -> usize {
        self.max_size
    }
    
    /// Enable or disable benchmark mode
    pub async fn set_benchmark_mode(&self, enabled: bool) {
        let mut mode = self.benchmark_mode.lock().await;
        *mode = enabled;
    }

    /// Check if benchmark mode is enabled
    pub async fn is_benchmark_mode(&self) -> bool {
        *self.benchmark_mode.lock().await
    }

    pub async fn warmup(&self) -> OptimizerResult<()> {
        debug!("Starting process pool warmup with {} processes", self.max_size);
        let warmup_count = self.max_size;
        let mut handles = Vec::with_capacity(warmup_count);
        
        // Spawn warmup processes
        for i in 0..warmup_count {
            let handle = tokio::spawn({
                let pool = self.clone();
                async move {
                    debug!("Warming up process {}/{}", i + 1, warmup_count);
                    let cmd = pool.acquire().await?;
                    
                    // Run a minimal operation to ensure process is ready
                    cmd.output()
                        .await
                        .map_err(|e| OptimizerError::sidecar(format!("Process warmup command failed: {}", e)))?;
                    
                    pool.release().await;
                    Ok::<_, OptimizerError>(())
                }
            });
            handles.push(handle);
        }
        
        // Wait for all warmup processes
        futures::future::try_join_all(handles)
            .await
            .map_err(|e| OptimizerError::sidecar(format!("Process warmup failed: {}", e)))?
            .into_iter()
            .collect::<OptimizerResult<Vec<_>>>()?;

        debug!("Process pool warmup completed successfully");
        Ok(())
    }
    
    /// Processes a batch of tasks using the available processes
    pub async fn process_batch(&self, tasks: Vec<ImageTask>) -> OptimizerResult<Vec<OptimizationResult>> {
        let mut benchmark_metrics = if self.is_benchmark_mode().await {
            let mut metrics = BenchmarkMetrics::new(tasks.len());
            metrics.reset(); // Reset metrics before starting new batch
            metrics.start_benchmarking();
            Some(metrics)
        } else {
            None
        };

        // Enqueue all tasks
        for task in tasks {
            self.enqueue_task(task).await;
        }
        
        let queue_length = self.queue_length().await;
        debug!("Processing batch of {} tasks", queue_length);
        
        let mut results = Vec::new();
        let executor = SharpExecutor::new(self);
        
        // Process tasks in chunks to maximize throughput
        while let Some(chunk) = self.dequeue_chunk().await {
            let start_time = Instant::now();
            
            // Record batch metrics if enabled
            if let Some(metrics) = &mut benchmark_metrics {
                metrics.record_batch(chunk.len());
            }
            
            // Execute the chunk using Sharp
            let (chunk_results, worker_metrics) = match executor.execute_batch(&chunk).await {
                Ok((results, metrics)) => (results, metrics),
                Err(e) => return Err(e)
            };

            // Record metrics for each result
            if let Some(metrics) = &mut benchmark_metrics {
                for result in &chunk_results {
                    metrics.record_compression(result.original_size, result.optimized_size);
                }
                
                // Record processing time
                let duration = Duration::new_unchecked(start_time.elapsed().as_secs_f64());
                metrics.add_processing_time(duration);
                
                // Record pool metrics
                let active_count = *self.active_count.lock().await;
                metrics.record_pool_metrics(active_count, queue_length);

                // Record worker pool metrics if available
                if let Some(worker_metrics) = worker_metrics {
                    metrics.worker_pool = Some(worker_metrics.clone());
                    debug!(
                        "Worker metrics - Workers: {}, Active: {}, Tasks per worker: {:?}",
                        worker_metrics.worker_count,
                        worker_metrics.active_workers,
                        worker_metrics.tasks_per_worker
                    );
                }
            }

            results.extend(chunk_results);
        }

        // Finalize benchmarking if enabled
        if let Some(mut metrics) = benchmark_metrics {
            let final_metrics = metrics.finalize_benchmarking();
            
            // Display benchmark report
            let report = BenchmarkReporter::from_metrics(final_metrics);
            info!("\n=== Batch Processing Report ===\n{}", report);
        }
        
        Ok(results)
    }
    
    /// Gets a chunk of tasks from the queue for batch processing
    async fn dequeue_chunk(&self) -> Option<Vec<ImageTask>> {
        let mut queue = self.task_queue.lock().await;
        if queue.is_empty() {
            return None;
        }

        let batch_size = *self.batch_size.lock().await;
        let mut chunk = Vec::with_capacity(batch_size);
        
        for _ in 0..batch_size {
            if let Some(queued_task) = queue.pop_front() {
                chunk.push(queued_task.task);
            } else {
                break;
            }
        }
        
        if chunk.is_empty() {
            None
        } else {
            Some(chunk)
        }
    }

    pub async fn get_active_tasks(&self) -> Vec<String> {
        let mut active_tasks = Vec::new();
        let queue = self.task_queue.lock().await;
        for task in queue.iter() {
            active_tasks.push(task.task.input_path.clone());
        }
        active_tasks
    }
} 