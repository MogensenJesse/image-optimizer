use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::warn;
use std::fmt;
use std::collections::HashMap;

/// Metrics collected from the worker pool during processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerPoolMetrics {
    pub worker_count: usize,
    pub tasks_per_worker: Vec<usize>,
    pub active_workers: usize,
    pub queue_length: usize,
    pub completed_tasks: usize,
    pub total_tasks: usize,
    pub duration_seconds: f64,
}

impl Default for WorkerPoolMetrics {
    fn default() -> Self {
        Self {
            worker_count: 0,
            tasks_per_worker: Vec::new(),
            active_workers: 0,
            queue_length: 0,
            completed_tasks: 0,
            total_tasks: 0,
            duration_seconds: 0.0,
        }
    }
}

/// Trait for types that can be benchmarked with detailed metrics
pub trait Benchmarkable {
    /// Record the processing time for a task
    fn add_processing_time(&mut self, duration: Duration);
    
    /// Record process pool metrics
    fn record_pool_metrics(&mut self, active_processes: usize, queue_length: usize);
    
    /// Finalize benchmarking and return the metrics
    fn finalize_benchmarking(&mut self) -> BenchmarkMetrics;
}

// Constants for validation
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

/// Metrics for tracking overall benchmark performance.
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
    
    // Batch metrics
    pub total_batches: usize,
    pub batch_sizes: Vec<usize>,
    pub mode_batch_size: usize,
    
    // Worker pool metrics
    pub worker_pool: Option<WorkerPoolMetrics>,
    
    // Internal tracking
    #[serde(skip)]
    start_time: Option<Instant>,
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
            total_batches: 0,
            batch_sizes: Vec::new(),
            mode_batch_size: 0,
            worker_pool: None,
            start_time: None,
        }
    }
}

impl BenchmarkMetrics {
    pub fn new(task_capacity: usize) -> Self {
        Self {
            processing_times: Vec::with_capacity(task_capacity),
            compression_ratios: Vec::with_capacity(task_capacity),
            ..Default::default()
        }
    }

    pub fn reset(&mut self) {
        self.total_duration = Duration::zero();
        self.processing_times.clear();
        self.avg_processing_time = Duration::zero();
        self.compression_ratios.clear();
        self.total_original_size = 0;
        self.total_optimized_size = 0;
        self.total_batches = 0;
        self.batch_sizes.clear();
        self.mode_batch_size = 0;
        self.start_time = None;
    }

    pub fn start_benchmarking(&mut self) {
        self.start_time = Some(Instant::now());
    }

    pub fn record_processing_time(&mut self, time: Duration) {
        self.processing_times.push(time);
    }

    pub fn record_compression(&mut self, original_size: u64, optimized_size: u64) {
        self.total_original_size += original_size;
        self.total_optimized_size += optimized_size;

        let ratio = if original_size > 0 {
            ((original_size - optimized_size) as f64 / original_size as f64) * 100.0
        } else {
            0.0
        };
        self.compression_ratios.push(Percentage::new_unchecked(ratio));
    }

    pub fn record_batch(&mut self, batch_size: usize) {
        self.total_batches += 1;
        self.batch_sizes.push(batch_size);
    }

    fn calculate_mode_batch_size(&self) -> usize {
        if self.batch_sizes.is_empty() {
            return 0;
        }

        let mut size_counts: HashMap<usize, usize> = HashMap::new();
        for &size in &self.batch_sizes {
            *size_counts.entry(size).or_insert(0) += 1;
        }

        size_counts
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(size, _)| size)
            .unwrap_or(0)
    }

    fn finalize_metrics(&mut self) -> Self {
        if let Some(start_time) = self.start_time {
            let total_duration = start_time.elapsed().as_secs_f64();
            self.total_duration = Duration::new_unchecked(total_duration);

            // Calculate average processing time
            if !self.processing_times.is_empty() {
                let avg_time = self.processing_times.iter()
                    .map(|d| d.as_secs_f64())
                    .sum::<f64>() / self.processing_times.len() as f64;
                self.avg_processing_time = Duration::new_unchecked(avg_time);
            }

            // Calculate mode batch size
            self.mode_batch_size = self.calculate_mode_batch_size();
        }

        self.clone()
    }
}

impl Benchmarkable for BenchmarkMetrics {
    fn add_processing_time(&mut self, duration: Duration) {
        self.record_processing_time(duration);
    }

    fn record_pool_metrics(&mut self, _active_processes: usize, _queue_length: usize) {
        // Process pool metrics removed
    }

    fn finalize_benchmarking(&mut self) -> BenchmarkMetrics {
        self.finalize_metrics()
    }
} 