use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tauri::AppHandle;
use crate::core::OptimizationResult;
use crate::worker::ImageTask;
use crate::processing::ImageOptimizer;
use crate::utils::{OptimizerError, OptimizerResult};
use crate::benchmarking::{BenchmarkMetrics, ProcessingStage, Duration};
use tracing::{debug, warn, info};
use std::sync::Mutex as StdMutex;
use lazy_static;

// More aggressive worker count calculation
fn calculate_optimal_workers() -> usize {
    let cpu_count = num_cpus::get();
    // Use 1.5x CPU cores, with a minimum of 4 and maximum of 12
    let suggested_workers = (cpu_count * 3 / 2).max(4).min(12);
    debug!("System has {} CPU cores, suggesting {} workers", cpu_count, suggested_workers);
    suggested_workers
}

lazy_static::lazy_static! {
    static ref INIT_TIME: StdMutex<Duration> = StdMutex::new(Duration::zero());
}

#[derive(Clone)]
pub struct WorkerPool {
    optimizer: ImageOptimizer,
    app: AppHandle,
    active_workers: Arc<Mutex<usize>>,
    semaphore: Arc<Semaphore>,
    worker_count: usize,
    benchmark_metrics: Arc<Mutex<Option<BenchmarkMetrics>>>,
}

impl WorkerPool {
    fn safe_div(numerator: f64, denominator: f64) -> f64 {
        if denominator == 0.0 {
            0.0
        } else {
            numerator / denominator
        }
    }

    pub fn new(app: AppHandle, worker_count: Option<usize>) -> Self {
        let worker_count = worker_count.unwrap_or_else(calculate_optimal_workers);
        debug!("Creating worker pool with {} workers", worker_count);
        let start_time = std::time::Instant::now();
        
        let pool = Self {
            optimizer: ImageOptimizer::new(),
            app,
            active_workers: Arc::new(Mutex::new(0)),
            semaphore: Arc::new(Semaphore::new(worker_count)),
            worker_count,
            benchmark_metrics: Arc::new(Mutex::new(None::<BenchmarkMetrics>)),
        };

        let init_time = Duration::new_unchecked(start_time.elapsed().as_secs_f64());
        if let Ok(mut time) = INIT_TIME.lock() {
            *time = init_time;
        }
        debug!("Worker pool initialized in {}", init_time);
        pool
    }

    pub fn get_worker_count(&self) -> usize {
        self.worker_count
    }

    pub fn enable_benchmarking(&self) {
        debug!("Enabling benchmarking for worker pool");
        let mut metrics = self.benchmark_metrics.try_lock()
            .expect("Failed to lock benchmark metrics mutex - this indicates a poisoned lock");
        let mut new_metrics = BenchmarkMetrics::new_with_capacity(self.worker_count, 100); // Default expected tasks
        
        if let Ok(time) = INIT_TIME.lock() {
            new_metrics.worker_init_time = *time;
            debug!("Recorded worker initialization time: {}", new_metrics.worker_init_time);
        } else {
            warn!("Failed to read initialization time - using default value");
        }
            
        *metrics = Some(new_metrics);
        if let Some(ref mut m) = *metrics {
            m.start_benchmark();
            debug!("Benchmark metrics initialized and started");
        }
    }

    pub async fn process(&self, task: ImageTask) -> OptimizerResult<OptimizationResult> {
        let task_path = task.input_path.clone();
        debug!("Processing single task: {}", task_path);
        
        // Record queue length before acquiring permit
        if let Ok(mut metrics) = self.benchmark_metrics.try_lock() {
            if let Some(ref mut m) = *metrics {
                let queue_len = self.get_queue_length().await;
                m.record_queue_length(queue_len);
                debug!("Current queue length: {}", queue_len);
            }
        }
        
        // Try to acquire a permit, record contention if we have to wait
        let permit_start = std::time::Instant::now();
        let _permit = match self.semaphore.try_acquire() {
            Ok(permit) => permit,
            Err(_) => {
                // Record contention and wait for permit
                if let Ok(mut metrics) = self.benchmark_metrics.try_lock() {
                    if let Some(ref mut m) = *metrics {
                        m.record_contention();
                        debug!("Recorded contention event");
                    }
                }
                self.semaphore.acquire().await.map_err(|e| {
                    warn!("Failed to acquire semaphore: {}", e);
                    OptimizerError::worker(format!("Failed to acquire worker: {}", e))
                })?
            }
        };
        
        let mut count = self.active_workers.lock().await;
        let worker_id = *count;
        *count += 1;
        
        // Record worker idle time and update concurrent tasks
        let idle_time = Duration::new_unchecked(permit_start.elapsed().as_secs_f64());
        if let Ok(mut metrics) = self.benchmark_metrics.try_lock() {
            if let Some(ref mut m) = *metrics {
                m.record_worker_idle(worker_id, idle_time);
                m.update_concurrent_tasks(*count);
                debug!("Active workers: {}, Worker {} idle time: {}", *count, worker_id, idle_time);
            }
        }

        let current_workers = *count;
        let available_permits = self.semaphore.available_permits();
        info!(
            "Worker {} started - Active: {}/{}, Available permits: {}, Task: {}", 
            worker_id, current_workers, self.worker_count, available_permits, task_path
        );

        let start_time = std::time::Instant::now();
        
        // Process single task as a batch of one
        let process_result = self.optimizer.process_batch(&self.app, vec![task]).await;
        
        match process_result {
            Ok((results, stage_times)) => {
                let result = results.into_iter().next().ok_or_else(|| {
                    OptimizerError::worker("No result returned from batch processing".to_string())
                })?;

                let processing_time = Duration::new_unchecked(start_time.elapsed().as_secs_f64());
                debug!("Task processed in {}", processing_time);

                // Update benchmark metrics if enabled
                if let Ok(mut metrics) = self.benchmark_metrics.try_lock() {
                    if let Some(ref mut m) = *metrics {
                        m.record_processing_time(processing_time);
                        m.record_worker_busy(worker_id, processing_time);
                        
                        // Record stage times if available
                        if let Some((loading, optimization, saving)) = stage_times {
                            m.record_stage_time(ProcessingStage::Loading, loading);
                            m.record_stage_time(ProcessingStage::Optimization, optimization);
                            m.record_stage_time(ProcessingStage::Saving, saving);
                        }
                        
                        m.add_compression_ratio(result.original_size, result.optimized_size);
                        debug!("Updated benchmark metrics for task: {}", task_path);
                    }
                }
                
                *count -= 1;
                info!(
                    "Worker {} finished - Active: {}/{}, Available permits: {}", 
                    worker_id, count.saturating_sub(1), self.worker_count, available_permits + 1
                );
                
                Ok(result)
            }
            Err(e) => {
                // Record task failure
                if let Ok(mut metrics) = self.benchmark_metrics.try_lock() {
                    if let Some(ref mut m) = *metrics {
                        m.record_task_failure();
                    }
                }
                Err(e)
            }
        }
    }

    async fn get_queue_length(&self) -> usize {
        let active_workers = *self.active_workers.lock().await;
        let available_permits = self.semaphore.available_permits();
        let total_capacity = self.worker_count;
        
        // Queue length is: total tasks waiting = (active workers + unavailable permits) - total capacity
        active_workers.saturating_add(total_capacity - available_permits).saturating_sub(total_capacity)
    }

    pub async fn process_batch(&self, tasks: Vec<ImageTask>) -> OptimizerResult<Vec<OptimizationResult>> {
        info!("Processing batch of {} tasks", tasks.len());
        
        // Reset and start timing for the entire batch
        let start_time = std::time::Instant::now();
        if let Ok(mut metrics) = self.benchmark_metrics.try_lock() {
            if let Some(ref mut m) = *metrics {
                *m = BenchmarkMetrics::new_with_capacity(self.worker_count, tasks.len());
                m.start_benchmark();
            }
        } else {
            warn!("Failed to lock benchmark metrics - metrics will not be recorded");
        }

        let start_overhead = std::time::Instant::now();
        debug!("Starting task management overhead timing");

        // Calculate tasks per worker
        let tasks_per_worker = (Self::safe_div(tasks.len() as f64, self.worker_count as f64)).ceil() as usize;
        debug!("Distributing {} tasks across {} workers ({} tasks per worker)", 
            tasks.len(), self.worker_count, tasks_per_worker);

        // Track initial queue length
        if let Ok(mut metrics) = self.benchmark_metrics.try_lock() {
            if let Some(ref mut m) = *metrics {
                let queue_len = tasks.len();
                m.record_queue_length(queue_len);
                debug!("Initial queue length: {}", queue_len);
            }
        } else {
            warn!("Failed to lock benchmark metrics - queue length will not be recorded");
        }

        // Process tasks in batches using the optimizer's batch processing
        let (results, stage_times) = self.optimizer.process_batch(&self.app, tasks).await?;

        let total_time = Duration::new_unchecked(start_overhead.elapsed().as_secs_f64());
        let processing_time = stage_times
            .map(|(l, o, s)| l + o + s)
            .unwrap_or_else(|| {
                warn!("Stage times not available - using total time for processing time");
                total_time
            });
        let overhead_time = Duration::new_unchecked(total_time.as_secs_f64() - processing_time.as_secs_f64());
        
        debug!("Task management overhead: {:.3}s (total: {:.3}s - processing: {:.3}s)", 
            overhead_time.as_secs_f64(), total_time.as_secs_f64(), processing_time.as_secs_f64());

        // Record worker metrics
        if let Ok(mut metrics) = self.benchmark_metrics.try_lock() {
            if let Some(ref mut m) = *metrics {
                // Record overhead time
                m.record_stage_time(ProcessingStage::Overhead, overhead_time);
                debug!("Recorded overhead time: {:.3}s", overhead_time.as_secs_f64());
                
                // Add compression ratios for all results
                for result in &results {
                    m.add_compression_ratio(result.original_size, result.optimized_size);
                }
                
                // Record worker metrics
                for worker_id in 0..self.worker_count {
                    let tasks_for_this_worker = if worker_id < results.len() / tasks_per_worker {
                        tasks_per_worker
                    } else if worker_id == results.len() / tasks_per_worker {
                        results.len() % tasks_per_worker
                    } else {
                        0
                    };
                    
                    if tasks_for_this_worker > 0 {
                        // Calculate per-task time for this worker
                        let worker_processing_time = Duration::new_unchecked(
                            processing_time.as_secs_f64() * Self::safe_div(tasks_for_this_worker as f64, results.len() as f64)
                        );
                        // Distribute overhead time evenly across workers that had tasks
                        let active_workers = Self::safe_div(results.len() as f64, tasks_per_worker as f64).ceil();
                        let worker_idle_time = Duration::new_unchecked(
                            Self::safe_div(overhead_time.as_secs_f64(), active_workers)
                        );

                        m.record_worker_busy(worker_id, worker_processing_time);
                        m.record_worker_idle(worker_id, worker_idle_time);
                        
                        // Update tasks per worker count
                        m.record_task_for_worker(worker_id);
                    }
                }
                
                // Record stage times
                if let Some((loading, optimization, saving)) = stage_times {
                    m.record_stage_time(ProcessingStage::Loading, loading);
                    m.record_stage_time(ProcessingStage::Optimization, optimization);
                    m.record_stage_time(ProcessingStage::Saving, saving);
                    debug!("Recorded stage times - Loading: {:.3}s, Optimization: {:.3}s, Saving: {:.3}s",
                        loading.as_secs_f64(), optimization.as_secs_f64(), saving.as_secs_f64());
                }
                
                // Finalize metrics
                m.finalize(start_time);
                debug!("Finalized benchmark metrics");
                
                // Generate and log the benchmark report
                drop(metrics);  // Release the lock before getting report
                if let Some(report) = self.get_benchmark_report() {
                    debug!("Generated benchmark report after batch processing");
                }
            }
        } else {
            warn!("Failed to lock benchmark metrics - metrics will not be recorded");
        }

        Ok(results)
    }

    pub async fn get_active_workers(&self) -> usize {
        *self.active_workers.lock().await
    }

    pub fn get_benchmark_report(&self) -> Option<String> {
        if let Ok(metrics) = self.benchmark_metrics.try_lock() {
            debug!("Successfully locked benchmark metrics");
            if let Some(ref m) = *metrics {
                debug!("Found metrics with {} processed tasks", m.compression_ratios.len());
                use crate::benchmarking::BenchmarkReporter;
                let reporter = BenchmarkReporter::from_metrics(m.clone());
                let report = reporter.to_string();
                info!("\n{}", report);
                debug!("Generated benchmark report");
                return Some(report);
            } else {
                debug!("No metrics available - benchmarking may not be enabled");
            }
        } else {
            warn!("Failed to lock benchmark metrics mutex");
        }
        None
    }
} 