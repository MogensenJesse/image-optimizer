use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::{Mutex, Semaphore};
use tauri_plugin_shell::{ShellExt, process::Command};
use crate::utils::{OptimizerError, OptimizerResult};
use crate::benchmarking::metrics::Benchmarkable;
use crate::benchmarking::reporter::BenchmarkReporter;
use crate::core::ImageTask;
use crate::core::OptimizationResult;
use tracing::{debug, info};
use num_cpus;
use tauri::AppHandle;
use crate::processing::BatchProcessor;

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
    app_handle: AppHandle,
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

    pub fn new(app_handle: AppHandle) -> Self {
        let size = Self::calculate_optimal_processes();
        debug!("Creating process pool with {} processes (based on {} CPU cores)", size, num_cpus::get());
        Self::new_with_size(app_handle, size)
    }

    pub fn new_with_size(app_handle: AppHandle, size: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(size)),
            app_handle,
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
        self.app_handle.shell()
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
        // Create batch processor with progress tracking
        let mut processor = BatchProcessor::new(self.clone(), self.app_handle.clone()).await;
        let results = processor.process_batch(tasks).await?;

        // Handle benchmarking if enabled
        if self.is_benchmark_mode().await {
            if let Some(mut metrics) = processor.take_metrics() {
                let final_metrics = metrics.finalize_benchmarking();
                
                // Display benchmark report
                let report = BenchmarkReporter::from_metrics(final_metrics);
                info!("\n=== Batch Processing Report ===\n{}", report);
            }
        }

        Ok(results)
    }
    
    /// Gets a chunk of tasks from the queue for batch processing
    pub async fn dequeue_chunk(&self) -> Option<Vec<ImageTask>> {
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

    /// Collects current worker pool metrics
    pub async fn collect_worker_metrics(&self) -> crate::benchmarking::metrics::WorkerPoolMetrics {
        let active_count = *self.active_count.lock().await;
        let queue_len = self.queue_length().await;
        let total_tasks = queue_len + active_count;

        crate::benchmarking::metrics::WorkerPoolMetrics {
            worker_count: self.max_size,
            tasks_per_worker: vec![total_tasks / self.max_size; self.max_size], // Approximate distribution
            active_workers: active_count,
            total_tasks,
        }
    }
} 