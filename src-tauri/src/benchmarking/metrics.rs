use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::warn;
use std::fmt;

/// Trait for types that can be benchmarked with detailed metrics
pub trait Benchmarkable {
    /// Record the processing time for a task
    fn add_processing_time(&mut self, duration: Duration);
    
    /// Record metrics for a specific worker
    fn record_worker_metrics(&mut self, worker_id: usize, idle_time: Duration, busy_time: Duration);
    
    /// Record queue metrics
    fn record_queue_metrics(&mut self, contention: bool);
    
    /// Finalize benchmarking and return the metrics
    fn finalize_benchmarking(&mut self) -> BenchmarkMetrics;
}

// Constants for validation
const MIN_WORKER_COUNT: usize = 1;  // Minimum number of workers
const MAX_WORKER_COUNT: usize = 32;  // Reasonable upper limit for worker count
const MAX_DURATION_SECS: f64 = 3600.0 * 24.0;  // 24 hours - reasonable max duration for a single operation

/// A strongly-typed duration value that ensures non-negative time values
/// and provides safe arithmetic operations.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Duration(f64);

impl Duration {
    /// Creates a new Duration without validation, but with safety guards.
    /// Invalid values will be clamped to valid range with warning logs.
    pub fn new_unchecked(seconds: f64) -> Self {
        if seconds < 0.0 {
            warn!("Negative duration provided: {:.2}s, using 0.0s instead", seconds);
            Self(0.0)
        } else if seconds > MAX_DURATION_SECS {
            warn!("Duration exceeds maximum allowed value: {:.2}s > {:.2}s, capping at maximum", 
                seconds, MAX_DURATION_SECS);
            Self(MAX_DURATION_SECS)
        } else {
            Self(seconds)
        }
    }

    /// Returns the duration in seconds as an f64.
    pub fn as_secs_f64(&self) -> f64 {
        self.0
    }

    /// Returns a Duration representing zero seconds.
    pub fn zero() -> Self {
        Self(0.0)
    }
}

impl Default for Duration {
    fn default() -> Self {
        Self::zero()
    }
}

impl std::ops::Add for Duration {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl std::ops::AddAssign for Duration {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 >= 60.0 {
            let minutes = (self.0 / 60.0).floor();
            let seconds = self.0 % 60.0;
            write!(f, "{:.0}m {:.2}s", minutes, seconds)
        } else if self.0 >= 1.0 {
            write!(f, "{:.2}s", self.0)
        } else {
            write!(f, "{:.0}ms", self.0 * 1000.0)
        }
    }
}

/// A strongly-typed percentage value that ensures values are between 0% and 100%.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Percentage(f64);

impl Percentage {
    /// Creates a new Percentage without validation, but with safety guards.
    /// Invalid values will be clamped to valid range with warning logs.
    pub fn new_unchecked(value: f64) -> Self {
        if value < 0.0 {
            warn!("Negative percentage provided: {:.1}%, using 0.0% instead", value);
            Self(0.0)
        } else if value > 100.0 {
            warn!("Percentage exceeds 100%: {:.1}%, capping at 100%", value);
            Self(100.0)
        } else {
            Self(value)
        }
    }

    /// Returns the percentage value as an f64.
    pub fn as_f64(&self) -> f64 {
        self.0
    }

    /// Returns a Percentage representing 0%.
    pub fn zero() -> Self {
        Self(0.0)
    }
}

impl Default for Percentage {
    fn default() -> Self {
        Self::zero()
    }
}

impl fmt::Display for Percentage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1}%", self.0)
    }
}

/// Metrics for tracking worker pool performance and utilization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerPoolMetrics {
    // Task distribution
    pub tasks_per_worker: Vec<usize>,     // Number of tasks processed by each worker
    pub failed_tasks: usize,              // Number of failed tasks
    pub contention_count: usize,          // Number of times tasks had to wait
}

impl WorkerPoolMetrics {
    /// Validates that a worker ID is within allowed range.
    fn validate_worker_id(&self, worker_id: usize) -> bool {
        if worker_id >= MAX_WORKER_COUNT {
            warn!("Worker ID exceeds maximum allowed value: {} > {}", worker_id, MAX_WORKER_COUNT);
            false
        } else {
            true
        }
    }

    /// Ensures that vectors have sufficient capacity for the given worker ID.
    fn ensure_worker_capacity(&mut self, worker_id: usize) {
        if !self.validate_worker_id(worker_id) {
            return;
        }

        let required_capacity = worker_id + 1;
        
        // Pre-allocate with the maximum of required capacity and minimum worker count
        let target_capacity = required_capacity.max(MIN_WORKER_COUNT);
        
        if self.tasks_per_worker.capacity() < target_capacity {
            self.tasks_per_worker.reserve(target_capacity - self.tasks_per_worker.len());
        }

        while self.tasks_per_worker.len() <= worker_id {
            self.tasks_per_worker.push(0);
        }
    }
}

impl Default for WorkerPoolMetrics {
    fn default() -> Self {
        Self {
            tasks_per_worker: Vec::new(),
            failed_tasks: 0,
            contention_count: 0,
        }
    }
}

/// Metrics for tracking batch size performance and distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSizeMetrics {
    pub batch_sizes: Vec<usize>,
    pub size_distribution: [usize; 3],
    #[serde(skip)]
    config: BatchSizeConfig,
}

impl BatchSizeMetrics {
    pub fn new(config: BatchSizeConfig) -> Self {
        Self {
            batch_sizes: Vec::with_capacity(100),
            size_distribution: [0; 3],
            config,
        }
    }

    pub fn record_batch_size(&mut self, size: usize) {
        self.batch_sizes.push(size);
        
        // Calculate distribution index based on equal ranges between min and max
        let range_size = (self.config.max_size - self.config.min_size) / 3;
        let ranges = [
            self.config.min_size..=(self.config.min_size + range_size),
            (self.config.min_size + range_size + 1)..=(self.config.min_size + 2 * range_size),
            (self.config.min_size + 2 * range_size + 1)..=self.config.max_size
        ];
        
        for (i, range) in ranges.iter().enumerate() {
            if range.contains(&size) {
                self.size_distribution[i] += 1;
                break;
            }
        }
    }

    pub fn average(&self) -> f64 {
        if self.batch_sizes.is_empty() {
            0.0
        } else {
            self.batch_sizes.iter().sum::<usize>() as f64 / self.batch_sizes.len() as f64
        }
    }

    pub fn min(&self) -> usize {
        *self.batch_sizes.iter().min().unwrap_or(&0)
    }

    pub fn max(&self) -> usize {
        *self.batch_sizes.iter().max().unwrap_or(&0)
    }

    pub fn min_size(&self) -> usize {
        self.config.min_size
    }

    pub fn max_size(&self) -> usize {
        self.config.max_size
    }

    pub fn finalize(&mut self) {
        if self.batch_sizes.is_empty() {
            return;
        }

        // Update distribution using the current config ranges
        self.size_distribution = [0; 3];
        for &size in &self.batch_sizes {
            // Calculate distribution index based on config ranges
            let range = (self.config.max_size - self.config.min_size) / 3;
            if range == 0 {
                // If range is 0, put everything in the first bucket
                self.size_distribution[0] += 1;
                continue;
            }
            
            // Handle case where size is less than min_size
            if size < self.config.min_size {
                self.size_distribution[0] += 1;
                continue;
            }
            
            // Now we can safely calculate the index
            let index = ((size - self.config.min_size) / range).min(2);
            self.size_distribution[index] += 1;
        }
    }
}

/// Metrics for tracking batch size configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSizeConfig {
    pub min_size: usize,
    pub max_size: usize,
    pub target_memory_usage: usize,
}

impl Default for BatchSizeConfig {
    fn default() -> Self {
        Self {
            min_size: 5,
            max_size: 50,
            target_memory_usage: 1024 * 1024 * 512, // 512MB
        }
    }
}

/// Metrics for tracking benchmarking results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    // Time-based metrics
    pub total_duration: Duration,
    pub processing_times: Vec<Duration>,  // Track individual processing times
    pub avg_processing_time: Duration,    // Calculated in finalize()
    
    // Optimization metrics
    pub compression_ratios: Vec<Percentage>,
    pub total_original_size: u64,
    pub total_optimized_size: u64,
    pub throughput_mbs: f64,
    
    // Internal tracking
    #[serde(skip)]
    start_time: Option<Instant>,
    
    // Worker pool metrics
    pub worker_pool: WorkerPoolMetrics,
    
    // Batch size metrics
    pub batch_metrics: BatchSizeMetrics,

    // Process pool metrics
    pub process_pool: Option<ProcessPoolMetrics>,
}

impl Default for BenchmarkMetrics {
    fn default() -> Self {
        Self {
            total_duration: Duration::zero(),
            processing_times: Vec::new(),
            avg_processing_time: Duration::zero(),
            compression_ratios: Vec::new(),
            total_original_size: 0,
            total_optimized_size: 0,
            throughput_mbs: 0.0,
            start_time: None,
            worker_pool: WorkerPoolMetrics::default(),
            batch_metrics: BatchSizeMetrics::new(BatchSizeConfig::default()),
            process_pool: None,
        }
    }
}

impl BenchmarkMetrics {
    pub fn new(task_capacity: usize) -> Self {
        Self {
            total_duration: Duration::zero(),
            processing_times: Vec::with_capacity(task_capacity),
            avg_processing_time: Duration::zero(),
            compression_ratios: Vec::with_capacity(task_capacity),
            total_original_size: 0,
            total_optimized_size: 0,
            throughput_mbs: 0.0,
            start_time: None,
            worker_pool: WorkerPoolMetrics::default(),
            batch_metrics: BatchSizeMetrics::new(BatchSizeConfig::default()),
            process_pool: Some(ProcessPoolMetrics::default()),
        }
    }

    /// Reset all metrics to their initial state
    pub fn reset(&mut self) {
        self.total_duration = Duration::zero();
        self.processing_times.clear();
        self.avg_processing_time = Duration::zero();
        self.compression_ratios.clear();
        self.total_original_size = 0;
        self.total_optimized_size = 0;
        self.throughput_mbs = 0.0;
        self.start_time = None;
        self.worker_pool.tasks_per_worker.clear();
        self.worker_pool.failed_tasks = 0;
        self.worker_pool.contention_count = 0;
        self.batch_metrics = BatchSizeMetrics::new(self.batch_metrics.config.clone());
        self.process_pool = Some(ProcessPoolMetrics::default());
    }

    pub fn start_benchmarking(&mut self) {
        self.reset(); // Reset metrics when starting a new benchmark
        self.start_time = Some(Instant::now());
    }

    pub fn record_processing_time(&mut self, time: Duration) {
        self.processing_times.push(time);
    }

    #[allow(dead_code)]
    pub fn record_compression(&mut self, original_size: u64, optimized_size: u64) {
        self.total_original_size += original_size;
        self.total_optimized_size += optimized_size;
        
        let bytes_saved = original_size as i64 - optimized_size as i64;
        let ratio = if original_size > 0 {
            (bytes_saved as f64 / original_size as f64) * 100.0
        } else {
            0.0
        };
        self.compression_ratios.push(Percentage::new_unchecked(ratio));
    }

    pub fn finalize(&mut self) -> Self {
        if let Some(start) = self.start_time.take() {
            self.total_duration = Duration::new_unchecked(start.elapsed().as_secs_f64());
            
            // Calculate average processing time from individual times
            if !self.processing_times.is_empty() {
                let total_proc_time: f64 = self.processing_times.iter()
                    .map(|d| d.as_secs_f64())
                    .sum();
                self.avg_processing_time = Duration::new_unchecked(
                    total_proc_time / self.processing_times.len() as f64
                );
            }
            
            // Calculate throughput in MB/s using total_duration for wall clock time
            let total_bytes = self.total_original_size;
            let duration_secs = self.total_duration.as_secs_f64();
            if duration_secs > 0.0 {
                self.throughput_mbs = (total_bytes as f64 / 1_000_000.0) / duration_secs;
            }

            // Ensure batch metrics are properly finalized
            self.batch_metrics.finalize();
        }

        // Create a clone with preserved batch metrics
        let mut clone = self.clone();
        clone.batch_metrics = self.batch_metrics.clone();
        clone
    }

    pub fn record_worker_busy(&mut self, worker_id: usize, _time: Duration) {
        if !self.worker_pool.validate_worker_id(worker_id) {
            return;
        }
        self.worker_pool.ensure_worker_capacity(worker_id);
        self.worker_pool.tasks_per_worker[worker_id] += 1;
    }

    pub fn record_worker_idle(&mut self, worker_id: usize, _time: Duration) {
        if !self.worker_pool.validate_worker_id(worker_id) {
            return;
        }
        self.worker_pool.ensure_worker_capacity(worker_id);
    }

    pub fn record_contention(&mut self) {
        self.worker_pool.contention_count += 1;
    }

    pub fn record_task_failure(&mut self) {
        self.worker_pool.failed_tasks += 1;
    }
}

impl Benchmarkable for BenchmarkMetrics {
    fn add_processing_time(&mut self, duration: Duration) {
        self.record_processing_time(duration);
    }
    
    fn record_worker_metrics(&mut self, worker_id: usize, idle_time: Duration, busy_time: Duration) {
        self.record_worker_idle(worker_id, idle_time);
        self.record_worker_busy(worker_id, busy_time);
    }
    
    fn record_queue_metrics(&mut self, contention: bool) {
        if contention {
            self.record_contention();
        }
    }
    
    fn finalize_benchmarking(&mut self) -> BenchmarkMetrics {
        self.finalize()
    }
}

/// Core metrics for the Sharp sidecar process pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessPoolMetrics {
    pub active_processes: Vec<usize>,      // Track number of active processes at each point
    pub spawn_times: Vec<Duration>,        // Time taken to spawn each process
    pub total_spawns: usize,              // Total number of processes spawned
}

impl Default for ProcessPoolMetrics {
    fn default() -> Self {
        Self {
            active_processes: Vec::with_capacity(100),
            spawn_times: Vec::with_capacity(100),
            total_spawns: 0,
        }
    }
}

impl ProcessPoolMetrics {
    pub fn record_spawn(&mut self, spawn_time: Duration) {
        self.total_spawns += 1;
        self.spawn_times.push(spawn_time);
    }
    
    pub fn update_active_count(&mut self, count: usize) {
        self.active_processes.push(count);
    }
} 