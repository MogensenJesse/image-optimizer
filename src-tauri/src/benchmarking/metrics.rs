use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{debug, warn};
use std::fmt;

// Constants for vector pre-allocation
const DEFAULT_QUEUE_SAMPLES_CAPACITY: usize = 1000;  // Assuming 1 sample per millisecond for a 1s operation
const DEFAULT_COMPRESSION_RATIO_CAPACITY: usize = 100;  // Default batch size
const MIN_WORKER_COUNT: usize = 4;  // Minimum number of workers

// Constants for validation
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

    /// Checks if the duration value is valid (within allowed range).
    pub fn is_valid(&self) -> bool {
        (0.0..=MAX_DURATION_SECS).contains(&self.0)
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
        } else {
            write!(f, "{:.2}s", self.0)
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
    // Queue metrics
    pub max_queue_length: usize,
    pub avg_queue_length: f64,
    pub queue_samples: Vec<(Duration, usize)>, // (timestamp, length)
    
    // Worker utilization
    pub worker_busy_time: Vec<Duration>,       // Time each worker spent processing
    pub worker_idle_time: Vec<Duration>,       // Time each worker spent idle
    pub contention_count: usize,          // Number of times tasks had to wait
    
    // Task distribution
    pub tasks_per_worker: Vec<usize>,     // Number of tasks processed by each worker
    pub failed_tasks: usize,              // Number of failed tasks
    pub retried_tasks: usize,             // Number of retried tasks
    
    // Scaling metrics
    pub avg_worker_efficiency: Percentage,       // Average worker utilization
    pub peak_concurrent_tasks: usize,     // Maximum number of concurrent tasks
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
        
        if self.worker_busy_time.capacity() < target_capacity {
            self.worker_busy_time.reserve(target_capacity - self.worker_busy_time.len());
            self.worker_idle_time.reserve(target_capacity - self.worker_idle_time.len());
            self.tasks_per_worker.reserve(target_capacity - self.tasks_per_worker.len());
        }

        while self.worker_busy_time.len() <= worker_id {
            self.worker_busy_time.push(Duration::zero());
            self.worker_idle_time.push(Duration::zero());
            self.tasks_per_worker.push(0);
        }
    }

    /// Creates a new WorkerPoolMetrics with pre-allocated capacity for the given worker count.
    pub fn with_capacity(worker_count: usize) -> Self {
        let capacity = worker_count.max(MIN_WORKER_COUNT).min(MAX_WORKER_COUNT);
        if capacity != worker_count {
            warn!("Adjusted worker count from {} to {} (min: {}, max: {})", 
                worker_count, capacity, MIN_WORKER_COUNT, MAX_WORKER_COUNT);
        }
        Self {
            max_queue_length: 0,
            avg_queue_length: 0.0,
            queue_samples: Vec::with_capacity(DEFAULT_QUEUE_SAMPLES_CAPACITY),
            worker_busy_time: Vec::with_capacity(capacity),
            worker_idle_time: Vec::with_capacity(capacity),
            contention_count: 0,
            tasks_per_worker: Vec::with_capacity(capacity),
            failed_tasks: 0,
            retried_tasks: 0,
            avg_worker_efficiency: Percentage::zero(),
            peak_concurrent_tasks: 0,
        }
    }

    /// Ensures that queue_samples vector has sufficient capacity.
    pub fn reserve_queue_samples(&mut self, additional: usize) {
        if self.queue_samples.len() + additional > self.queue_samples.capacity() {
            self.queue_samples.reserve(additional);
        }
    }
}

impl Default for WorkerPoolMetrics {
    fn default() -> Self {
        Self::with_capacity(num_cpus::get())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    // Time-based metrics
    pub total_duration: Duration,
    pub avg_processing_time: Duration,
    pub worker_init_time: Duration,
    
    // Processing stage metrics
    pub loading_time: Duration,
    pub optimization_time: Duration,
    pub saving_time: Duration,
    pub overhead_time: Duration,  // Time spent in task management/concurrency
    
    // Optimization metrics
    pub compression_ratios: Vec<Percentage>,
    pub total_original_size: u64,
    pub total_optimized_size: u64,
    pub throughput_mbs: f64,  // New field for MB/s throughput
    
    // Internal tracking
    #[serde(skip)]
    start_time: Option<Instant>,
    #[serde(skip)]
    pub total_active_time: Duration,
    
    // Worker pool metrics
    pub worker_pool: WorkerPoolMetrics,
}

impl Default for BenchmarkMetrics {
    fn default() -> Self {
        Self {
            total_duration: Duration::zero(),
            avg_processing_time: Duration::zero(),
            worker_init_time: Duration::zero(),
            loading_time: Duration::zero(),
            optimization_time: Duration::zero(),
            saving_time: Duration::zero(),
            overhead_time: Duration::zero(),
            compression_ratios: Vec::new(),
            total_original_size: 0,
            total_optimized_size: 0,
            throughput_mbs: 0.0,
            start_time: None,
            total_active_time: Duration::zero(),
            worker_pool: WorkerPoolMetrics::default(),
        }
    }
}

impl BenchmarkMetrics {
    fn safe_div(numerator: f64, denominator: f64) -> f64 {
        if denominator == 0.0 {
            0.0
        } else {
            numerator / denominator
        }
    }

    fn get_elapsed_time(&self) -> Option<Duration> {
        self.start_time.map(|start| Duration::new_unchecked(start.elapsed().as_secs_f64()))
    }

    fn get_elapsed_time_or(&self, default: Duration) -> Duration {
        self.get_elapsed_time().unwrap_or(default)
    }

    fn validate_timestamp(&self, time: Duration) -> bool {
        if !time.is_valid() {
            warn!("Invalid duration provided: {}", time);
            return false;
        }

        if let Some(elapsed) = self.get_elapsed_time() {
            // Time should not be negative or greater than total elapsed time
            if time > elapsed {
                warn!("Time value exceeds elapsed time: {} > {}", time, elapsed);
                return false;
            }
            true
        } else {
            warn!("Cannot validate timestamp: benchmark not started");
            false
        }
    }

    pub fn new_with_capacity(worker_count: usize, expected_tasks: usize) -> Self {
        let task_capacity = expected_tasks.max(DEFAULT_COMPRESSION_RATIO_CAPACITY);
        Self {
            total_duration: Duration::zero(),
            avg_processing_time: Duration::zero(),
            worker_init_time: Duration::zero(),
            loading_time: Duration::zero(),
            optimization_time: Duration::zero(),
            saving_time: Duration::zero(),
            overhead_time: Duration::zero(),
            compression_ratios: Vec::with_capacity(task_capacity),
            total_original_size: 0,
            total_optimized_size: 0,
            throughput_mbs: 0.0,
            start_time: None,
            total_active_time: Duration::zero(),
            worker_pool: WorkerPoolMetrics::with_capacity(worker_count),
        }
    }

    pub fn reset(&mut self) {
        debug!("Resetting benchmark metrics");
        *self = Self::default();
    }

    pub fn start_benchmark(&mut self) {
        self.reset();
        self.start_time = Some(Instant::now());
        debug!("Started benchmark timing");
    }

    pub fn record_processing_time(&mut self, time: Duration) {
        if time.as_secs_f64() < 0.0 {
            warn!("Negative processing time provided: {}, ignoring", time);
            return;
        }
        self.total_active_time += time;
        debug!(
            total_time = self.total_active_time.as_secs_f64(),
            added_time = time.as_secs_f64(),
            "Recorded processing time"
        );
    }

    pub fn add_compression_ratio(&mut self, original: u64, optimized: u64) {
        self.total_original_size += original;
        self.total_optimized_size += optimized;
        
        let ratio = if original > 0 {
            Self::safe_div(optimized as f64, original as f64) * 100.0
        } else {
            warn!("Original size is 0, using 0% compression ratio");
            0.0
        };
        let ratio = Percentage::new_unchecked(ratio);
        self.compression_ratios.push(ratio);
        debug!("Added compression ratio: {}", ratio);
    }

    pub fn record_queue_length(&mut self, length: usize) {
        if let Some(timestamp) = self.get_elapsed_time() {
            // Ensure we have capacity for the new sample
            self.worker_pool.reserve_queue_samples(1);
            self.worker_pool.queue_samples.push((timestamp, length));
            self.worker_pool.max_queue_length = self.worker_pool.max_queue_length.max(length);
            debug!(
                queue_length = length,
                timestamp = timestamp.as_secs_f64(),
                "Recorded queue length"
            );
        } else {
            warn!("Cannot record queue length: benchmark not started");
        }
    }

    pub fn record_worker_busy(&mut self, worker_id: usize, time: Duration) {
        if !self.worker_pool.validate_worker_id(worker_id) {
            return;
        }
        if !self.validate_timestamp(time) {
            warn!("Invalid worker busy time: {}, ignoring", time);
            return;
        }
        self.worker_pool.ensure_worker_capacity(worker_id);
        self.worker_pool.worker_busy_time[worker_id] += time;
        self.worker_pool.tasks_per_worker[worker_id] += 1;
        debug!(
            worker_id = worker_id,
            time = time.as_secs_f64(),
            total_time = self.worker_pool.worker_busy_time[worker_id].as_secs_f64(),
            "Worker busy time updated"
        );
    }

    pub fn record_worker_idle(&mut self, worker_id: usize, time: Duration) {
        if !self.worker_pool.validate_worker_id(worker_id) {
            return;
        }
        if !self.validate_timestamp(time) {
            warn!("Invalid worker idle time: {}, ignoring", time);
            return;
        }
        self.worker_pool.ensure_worker_capacity(worker_id);
        self.worker_pool.worker_idle_time[worker_id] += time;
        debug!(
            worker_id = worker_id,
            time = time.as_secs_f64(),
            total_time = self.worker_pool.worker_idle_time[worker_id].as_secs_f64(),
            "Worker idle time updated"
        );
    }

    pub fn record_contention(&mut self) {
        self.worker_pool.contention_count += 1;
    }

    pub fn record_task_failure(&mut self) {
        self.worker_pool.failed_tasks += 1;
    }

    pub fn update_concurrent_tasks(&mut self, count: usize) {
        self.worker_pool.peak_concurrent_tasks = self.worker_pool.peak_concurrent_tasks.max(count);
    }

    pub fn finalize(&mut self, start: Instant) {
        self.total_duration = Duration::new_unchecked(start.elapsed().as_secs_f64());
        if !self.validate_timestamp(self.total_duration) {
            warn!("Invalid total duration: {}, using elapsed time", self.total_duration);
            self.total_duration = self.get_elapsed_time_or(Duration::zero());
        }

        // Calculate averages safely using number of processed images
        let total_images = self.compression_ratios.len() as f64;
        self.avg_processing_time = Duration::new_unchecked(
            Self::safe_div(self.total_active_time.as_secs_f64(), total_images)
        );

        // Calculate throughput (MB/s) safely
        let total_mb = (self.total_original_size as f64) / (1024.0 * 1024.0);
        self.throughput_mbs = Self::safe_div(total_mb, self.total_duration.as_secs_f64());

        // Calculate worker pool metrics safely
        let sum: usize = self.worker_pool.queue_samples.iter().map(|(_, len)| len).sum();
        self.worker_pool.avg_queue_length = Self::safe_div(
            sum as f64,
            self.worker_pool.queue_samples.len() as f64
        );

        // Calculate worker efficiency safely
        let mut total_efficiency = 0.0;
        for (busy, _) in self.worker_pool.worker_busy_time.iter().zip(&self.worker_pool.worker_idle_time) {
            total_efficiency += if self.total_duration.as_secs_f64() > 0.0 {
                Self::safe_div(busy.as_secs_f64(), self.total_duration.as_secs_f64()) * 100.0
            } else {
                0.0
            };
        }

        self.worker_pool.avg_worker_efficiency = Percentage::new_unchecked(total_efficiency);

        // Debug logging with structured fields
        debug!(
            total_duration = self.total_duration.as_secs_f64(),
            avg_processing_time = self.avg_processing_time.as_secs_f64(),
            worker_efficiency = self.worker_pool.avg_worker_efficiency.as_f64(),
            throughput_mbs = self.throughput_mbs,
            total_images = total_images,
            "Benchmark metrics finalized"
        );
    }

    pub fn record_stage_time(&mut self, stage: ProcessingStage, time: Duration) {
        if !self.validate_timestamp(time) {
            warn!("Invalid stage time for {}: {}, ignoring", stage, time);
            return;
        }
        match stage {
            ProcessingStage::Loading => {
                self.loading_time += time;
                self.total_active_time += time;
            },
            ProcessingStage::Optimization => {
                self.optimization_time += time;
                self.total_active_time += time;
            },
            ProcessingStage::Saving => {
                self.saving_time += time;
                self.total_active_time += time;
            },
            ProcessingStage::Overhead => self.overhead_time += time,
        }
        debug!(
            stage = ?stage,
            time = time.as_secs_f64(),
            "Stage time recorded"
        );
    }

    pub fn record_task_for_worker(&mut self, worker_id: usize) {
        self.worker_pool.ensure_worker_capacity(worker_id);
        self.worker_pool.tasks_per_worker[worker_id] += 1;
        debug!(
            worker_id = worker_id,
            total_tasks = self.worker_pool.tasks_per_worker[worker_id],
            "Task recorded for worker"
        );
    }
}

/// The stage of image processing being measured.
#[derive(Debug, Clone, Copy)]
pub enum ProcessingStage {
    /// Loading and validating input files
    Loading,
    /// Optimizing images using Sharp
    Optimization,
    /// Saving optimized files
    Saving,
    /// Task management and coordination overhead
    Overhead,
}

impl std::fmt::Display for ProcessingStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Loading => write!(f, "loading"),
            Self::Optimization => write!(f, "optimization"),
            Self::Saving => write!(f, "saving"),
            Self::Overhead => write!(f, "overhead"),
        }
    }
} 