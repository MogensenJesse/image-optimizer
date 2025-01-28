use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tauri::AppHandle;
use crate::core::OptimizationResult;
use crate::worker::ImageTask;
use crate::processing::ImageOptimizer;
use crate::benchmarking::{BenchmarkMetrics, Duration, Benchmarkable};
use crate::benchmarking::reporter::BenchmarkReporter;
use crate::worker::error::{WorkerError, WorkerResult};
use tracing::{debug, info, warn};
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
    /// The application handle, kept for potential optimizer restarts
    /// or future process management operations.
    /// Currently only used during initialization.
    #[allow(dead_code)]
    app: AppHandle,
    active_workers: Arc<Mutex<usize>>,
    semaphore: Arc<Semaphore>,
    worker_count: usize,
    benchmark_mode: Arc<Mutex<bool>>,
    benchmark_metrics: Arc<Mutex<Option<BenchmarkMetrics>>>,
}

impl WorkerPool {
    pub fn new(app: AppHandle, worker_count: Option<usize>) -> WorkerResult<Self> {
        let worker_count = worker_count.unwrap_or_else(calculate_optimal_workers);
        debug!("Creating worker pool with {} workers", worker_count);
        let start_time = std::time::Instant::now();
        
        // Validate worker count
        if worker_count == 0 {
            return Err(WorkerError::InitializationError("Worker count cannot be zero".to_string()));
        }
        
        let pool = Self {
            optimizer: ImageOptimizer::new(app.clone()),
            app,
            active_workers: Arc::new(Mutex::new(0)),
            semaphore: Arc::new(Semaphore::new(worker_count)),
            worker_count,
            benchmark_mode: Arc::new(Mutex::new(false)),
            benchmark_metrics: Arc::new(Mutex::new(None::<BenchmarkMetrics>)),
        };

        let init_time = Duration::new_unchecked(start_time.elapsed().as_secs_f64());
        if let Ok(mut time) = INIT_TIME.lock() {
            *time = init_time;
        } else {
            return Err(WorkerError::InitializationError("Failed to initialize worker pool timing".to_string()));
        }
        debug!("Worker pool initialized in {}", init_time);
        Ok(pool)
    }

    // Helper method to record metrics only when benchmarking is enabled
    async fn record_metric(&self, f: impl FnOnce(&mut BenchmarkMetrics, bool)) {
        let is_benchmark = *self.benchmark_mode.lock().await;
        if !is_benchmark {
            return;
        }
        if let Ok(mut metrics) = self.benchmark_metrics.try_lock() {
            if let Some(ref mut m) = *metrics {
                f(m, is_benchmark);
            }
        }
    }

    pub fn get_worker_count(&self) -> usize {
        self.worker_count
    }

    pub async fn enable_benchmarking(&self) {
        let mut mode = self.benchmark_mode.lock().await;
        *mode = true;
        debug!("Enabling benchmarking for worker pool");
        
        let mut metrics = self.benchmark_metrics.try_lock()
            .expect("Failed to lock benchmark metrics mutex - this indicates a poisoned lock");
        let new_metrics = BenchmarkMetrics::new(100); // Default expected tasks
        *metrics = Some(new_metrics);
        debug!("Benchmark metrics initialized");
    }

    // Helper method to reset metrics
    async fn reset_metrics(&self) {
        if let Ok(mut metrics) = self.benchmark_metrics.try_lock() {
            if let Some(ref mut m) = *metrics {
                m.reset();
                debug!("Reset benchmark metrics");
            }
        }
    }

    pub async fn process(&self, task: ImageTask) -> WorkerResult<OptimizationResult> {
        let task_path = task.input_path.clone();
        debug!("Processing single task: {}", task_path);
        
        // Start benchmarking timing here, when we actually begin processing
        self.record_metric(|m, is_benchmark| {
            if is_benchmark {
                m.start_benchmarking();
                debug!("Started benchmark timing");
            }
        }).await;
        
        // Record queue metrics only if benchmarking
        let queue_len = self.get_queue_length().await;
        let contention = queue_len > 0;
        self.record_metric(|m, is_benchmark| {
            <dyn Benchmarkable>::record_queue_metrics(m, contention);
            if is_benchmark {
                debug!("Current queue length: {}", queue_len);
            }
        }).await;
        
        // Try to acquire a permit, record contention if we have to wait
        let _permit = match self.semaphore.try_acquire() {
            Ok(permit) => permit,
            Err(_) => {
                self.record_metric(|m, is_benchmark| {
                    <dyn Benchmarkable>::record_queue_metrics(m, true);
                    if is_benchmark {
                        debug!("Recorded contention event");
                    }
                }).await;
                self.semaphore.acquire().await.map_err(|e| {
                    warn!("Failed to acquire semaphore: {}", e);
                    WorkerError::CapacityError(format!("Failed to acquire worker: {}", e))
                })?
            }
        };
        
        let start_time = std::time::Instant::now();
        
        // Process single task as a batch of one
        let process_result = self.optimizer.process_batch(vec![task]).await;
        
        match process_result {
            Ok((mut results, _memory_metrics)) => {  // Destructure tuple and ignore memory metrics
                let result = results.pop().ok_or_else(|| {
                    WorkerError::ProcessingError("No result returned from batch processing".to_string())
                })?;

                let processing_time = Duration::new_unchecked(start_time.elapsed().as_secs_f64());
                debug!("Task processed in {}", processing_time);
                
                // Record metrics if in benchmark mode
                self.record_metric(|m, is_benchmark| {
                    if is_benchmark {
                        m.add_processing_time(processing_time);
                    }
                }).await;
                
                Ok(result)
            },
            Err(e) => {
                // Record task failure
                self.record_metric(|m, is_benchmark| {
                    if is_benchmark {
                        m.record_task_failure();
                    }
                }).await;
                Err(e.into())
            },
        }
    }

    async fn get_queue_length(&self) -> usize {
        let active_workers = *self.active_workers.lock().await;
        let available_permits = self.semaphore.available_permits();
        let total_capacity = self.worker_count;
        
        // Queue length is: total tasks waiting = (active workers + unavailable permits) - total capacity
        active_workers.saturating_add(total_capacity - available_permits).saturating_sub(total_capacity)
    }

    pub async fn process_batch(&self, tasks: Vec<ImageTask>) -> WorkerResult<(Vec<OptimizationResult>, Duration)> {
        info!("Starting batch processing of {} tasks", tasks.len());
        
        // Reset metrics before starting new batch
        self.reset_metrics().await;
        
        // Start benchmarking timing here, when we actually begin processing
        self.record_metric(|m, is_benchmark| {
            if is_benchmark {
                m.start_benchmarking();
                debug!("Started benchmark timing");
            }
        }).await;

        let total_tasks = tasks.len();
        
        // Record queue metrics only if benchmarking
        let contention = total_tasks > self.worker_count;
        self.record_metric(|m, is_benchmark| {
            <dyn Benchmarkable>::record_queue_metrics(m, contention);
            if is_benchmark {
                debug!("Queue metrics recorded: contention={}", contention);
            }
        }).await;

        // Process the batch
        info!("Processing batch with {} tasks using {} workers", total_tasks, self.worker_count);
        let optimizer_result = self.optimizer.process_batch(tasks).await
            .map_err(|e| WorkerError::ProcessingError(format!("Batch processing failed: {}", e)))?;
        
        let (results, memory_metrics) = optimizer_result;
        let mut total_duration = Duration::zero();
        
        // Record metrics and generate report if benchmarking
        self.record_metric(|m, is_benchmark| {
            if is_benchmark {
                debug!("Recording batch metrics");
                
                // Record memory metrics
                m.batch_metrics.memory_metrics.initial_memory = memory_metrics.initial_memory;
                m.batch_metrics.memory_metrics.avg_batch_memory = memory_metrics.avg_batch_memory;
                m.batch_metrics.memory_metrics.peak_pressure = memory_metrics.peak_pressure;
                m.batch_metrics.memory_metrics.memory_distribution = memory_metrics.memory_distribution;
                
                // Record each chunk's size as a separate batch
                let batch_size = 50; // This is from optimizer's BatchSizeConfig::default().max_size
                let full_chunks = total_tasks / batch_size;
                let remainder = total_tasks % batch_size;
                
                // Record full chunks
                for _ in 0..full_chunks {
                    m.batch_metrics.record_batch_size(batch_size);
                }
                
                // Record the remainder chunk if any
                if remainder > 0 {
                    m.batch_metrics.record_batch_size(remainder);
                }
            }
            
            // Get the current total duration
            total_duration = m.total_duration;
            
            // Calculate actual parallel execution metrics
            let tasks_per_worker = (total_tasks + self.worker_count - 1) / self.worker_count;
            
            if is_benchmark {
                debug!("Calculated parallel metrics: {} tasks per worker", tasks_per_worker);
            }
            
            // Record worker distribution based on actual results
            for i in 0..results.len() {
                let worker_id = i % self.worker_count;
                <dyn Benchmarkable>::record_worker_metrics(m, worker_id, Duration::zero(), Duration::zero());
            }
            
            // Add compression ratios for all results
            if is_benchmark {
                debug!("Recording compression ratios for {} results", results.len());
            }
            for result in &results {
                if result.success {
                    m.record_compression(result.original_size, result.optimized_size);
                }
            }

            // Only generate report in benchmark mode
            if is_benchmark {
                info!("Finalizing benchmark metrics");
                let metrics = <dyn Benchmarkable>::finalize_benchmarking(m);
                let reporter = BenchmarkReporter::from_metrics(metrics);
                info!("\nBatch Processing Report:\n{}", reporter);
            }
        }).await;
        
        Ok((results, total_duration))
    }

    pub async fn get_active_workers(&self) -> usize {
        *self.active_workers.lock().await
    }

    pub(crate) async fn get_active_workers_detailed(&self) -> (usize, Vec<String>) {
        let count = *self.active_workers.lock().await;
        let active_tasks = self.optimizer.get_active_tasks().await;
        (count, active_tasks)
    }

    // Ensure benchmarking metrics are finalized even if an error occurs
    #[allow(dead_code)]
    pub fn finalize_benchmarking(&self) {
        if let Ok(mut metrics) = self.benchmark_metrics.try_lock() {
            if let Some(ref mut m) = *metrics {
                let _ = <dyn Benchmarkable>::finalize_benchmarking(m);
                debug!("Finalized benchmarking metrics");
            }
        }
    }
}

impl Benchmarkable for WorkerPool {
    fn add_processing_time(&mut self, duration: Duration) {
        if let Ok(mut metrics) = self.benchmark_metrics.try_lock() {
            if let Some(ref mut m) = *metrics {
                m.record_processing_time(duration);
            }
        }
    }
    
    fn record_worker_metrics(&mut self, worker_id: usize, idle_time: Duration, busy_time: Duration) {
        if let Ok(mut metrics) = self.benchmark_metrics.try_lock() {
            if let Some(ref mut m) = *metrics {
                m.record_worker_metrics(worker_id, idle_time, busy_time);
            }
        }
    }
    
    fn record_queue_metrics(&mut self, contention: bool) {
        if let Ok(mut metrics) = self.benchmark_metrics.try_lock() {
            if let Some(ref mut m) = *metrics {
                if contention {
                    m.record_contention();
                }
            }
        }
    }
    
    fn finalize_benchmarking(&mut self) -> BenchmarkMetrics {
        if let Ok(mut metrics) = self.benchmark_metrics.try_lock() {
            if let Some(ref mut m) = *metrics {
                return m.finalize();
            }
        }
        BenchmarkMetrics::default()
    }
} 