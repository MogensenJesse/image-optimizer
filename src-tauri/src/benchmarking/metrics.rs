use serde::{Deserialize, Serialize};
use std::time::Instant;
use std::collections::HashMap;
use crate::benchmarking::reporter::BenchmarkReporter;

/// Module containing validation and formatting functions for metrics
pub mod validations {
    use tracing::warn;
    
    // Constants for validation
    pub const MAX_DURATION_SECS: f64 = 3600.0 * 24.0;  // 24 hours - reasonable max duration

    /// Validates a duration value, ensuring it's within acceptable bounds
    pub fn validate_duration(seconds: f64) -> f64 {
        if seconds < 0.0 {
            warn!("Negative duration provided: {:.2}s, using 0.0s instead", seconds);
            0.0
        } else if seconds > MAX_DURATION_SECS {
            warn!("Duration exceeds maximum allowed value: {:.2}s > {:.2}s, capping at maximum", 
                seconds, MAX_DURATION_SECS);
            MAX_DURATION_SECS
        } else {
            seconds
        }
    }

    /// Validates a percentage value, ensuring it's between 0 and 100
    pub fn validate_percentage(value: f64) -> f64 {
        if value < 0.0 {
            warn!("Negative percentage provided: {:.1}%, using 0.0% instead", value);
            0.0
        } else if value > 100.0 {
            warn!("Percentage exceeds 100%: {:.1}%, capping at 100%", value);
            100.0
        } else {
            value
        }
    }

    /// Formats a duration value as a readable string
    pub fn format_duration(seconds: f64) -> String {
        if seconds >= 60.0 {
            let minutes = (seconds / 60.0).floor();
            let secs = seconds % 60.0;
            format!("{:.0}m {:.2}s", minutes, secs)
        } else if seconds >= 1.0 {
            format!("{:.2}s", seconds)
        } else {
            format!("{:.0}ms", seconds * 1000.0)
        }
    }

    /// Formats a percentage value as a readable string
    pub fn format_percentage(value: f64) -> String {
        format!("{:.1}%", value)
    }
}

/// Metrics collected from the worker pool during processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerPoolMetrics {
    pub worker_count: usize,
    pub tasks_per_worker: Vec<usize>,
}

impl Default for WorkerPoolMetrics {
    fn default() -> Self {
        Self {
            worker_count: 0,
            tasks_per_worker: Vec::new(),
        }
    }
}

/// Core metrics collector trait that defines the basic metrics interface 
/// without direct dependencies on the benchmarking system.
/// 
/// This trait can be implemented by any component that wants to track metrics
/// without directly depending on the benchmarking system.
pub trait MetricsCollector: Send + Sync {
    /// Record the processing time for a task
    fn record_time(&mut self, duration_secs: f64);
    
    /// Record compression metrics for an optimized image
    fn record_size_change(&mut self, original_size: u64, optimized_size: u64);
    
    /// Record batch processing information
    fn record_batch_info(&mut self, batch_size: usize);
    
    /// Record worker pool statistics
    fn record_worker_stats(&mut self, worker_count: usize, tasks_per_worker: Vec<usize>);
    
    /// Finalize collection and return the metrics
    fn finalize(&mut self) -> Option<BenchmarkMetrics> {
        None
    }
}

/// Null implementation of MetricsCollector that doesn't record anything
pub struct NullMetricsCollector;

impl NullMetricsCollector {
    pub fn new() -> Self {
        Self {}
    }
}

impl MetricsCollector for NullMetricsCollector {
    fn record_time(&mut self, _duration_secs: f64) {
        // Do nothing
    }
    
    fn record_size_change(&mut self, _original_size: u64, _optimized_size: u64) {
        // Do nothing
    }
    
    fn record_batch_info(&mut self, _batch_size: usize) {
        // Do nothing
    }
    
    fn record_worker_stats(&mut self, _worker_count: usize, _tasks_per_worker: Vec<usize>) {
        // Do nothing
    }
    
    fn finalize(&mut self) -> Option<BenchmarkMetrics> {
        None
    }
}

/// Metrics for tracking overall benchmark performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    // Time-based metrics
    pub total_duration: f64,
    pub avg_processing_time: f64,    // Calculated in finalize()
    
    // Optimization metrics - Essential for image optimization benchmarking
    pub avg_compression_ratio: f64,  // Replaced individual ratios with average
    pub total_original_size: u64,
    pub total_optimized_size: u64,
    
    // Batch metrics - Simplified to only track essential counts
    pub total_batches: usize,
    pub mode_batch_size: usize,      // Kept for insight into batch processing efficiency
    
    // Worker pool metrics
    pub worker_pool: Option<WorkerPoolMetrics>,
    
    // Internal tracking fields - not visible in serialization
    #[serde(skip)]
    start_time: Option<Instant>,
    
    // These fields are used for calculations but not reported directly
    #[serde(skip)]
    processing_times_sum: f64,       // Sum instead of vector for reduced memory usage
    #[serde(skip)]
    processing_times_count: usize,   // Count of times instead of vector length
    #[serde(skip)]
    compression_ratios_sum: f64,     // Sum instead of vector
    #[serde(skip)]
    compression_ratios_count: usize, // Count of ratios
    #[serde(skip)]
    batch_size_counts: HashMap<usize, usize>,  // For mode calculation
}

impl Default for BenchmarkMetrics {
    fn default() -> Self {
        Self {
            total_duration: 0.0,
            avg_processing_time: 0.0,
            avg_compression_ratio: 0.0,
            total_original_size: 0,
            total_optimized_size: 0,
            total_batches: 0,
            mode_batch_size: 0,
            worker_pool: None,
            start_time: None,
            processing_times_sum: 0.0,
            processing_times_count: 0,
            compression_ratios_sum: 0.0,
            compression_ratios_count: 0,
            batch_size_counts: HashMap::new(),
        }
    }
}

impl BenchmarkMetrics {
    pub fn start_benchmarking(&mut self) {
        self.start_time = Some(Instant::now());
    }

    pub fn record_processing_time(&mut self, time: f64) {
        let validated_time = validations::validate_duration(time);
        self.processing_times_sum += validated_time;
        self.processing_times_count += 1;
    }

    pub fn record_compression(&mut self, original_size: u64, optimized_size: u64) {
        self.total_original_size += original_size;
        self.total_optimized_size += optimized_size;

        let ratio = if original_size > 0 && original_size >= optimized_size {
            ((original_size - optimized_size) as f64 / original_size as f64) * 100.0
        } else {
            0.0
        };
        let validated_ratio = validations::validate_percentage(ratio);
        self.compression_ratios_sum += validated_ratio;
        self.compression_ratios_count += 1;
    }

    pub fn record_batch(&mut self, batch_size: usize) {
        self.total_batches += 1;
        *self.batch_size_counts.entry(batch_size).or_insert(0) += 1;
    }

    fn calculate_mode_batch_size(&self) -> usize {
        if self.batch_size_counts.is_empty() {
            return 0;
        }

        self.batch_size_counts
            .iter()
            .max_by_key(|&(_, count)| count)
            .map(|(size, _)| *size)
            .unwrap_or(0)
    }

    fn finalize_metrics(&mut self) -> Self {
        if let Some(start_time) = self.start_time {
            let total_duration = start_time.elapsed().as_secs_f64();
            self.total_duration = validations::validate_duration(total_duration);

            // Calculate average processing time
            if self.processing_times_count > 0 {
                let avg_time = self.processing_times_sum / self.processing_times_count as f64;
                self.avg_processing_time = validations::validate_duration(avg_time);
            }

            // Calculate average compression ratio
            if self.compression_ratios_count > 0 {
                let avg_ratio = self.compression_ratios_sum / self.compression_ratios_count as f64;
                self.avg_compression_ratio = validations::validate_percentage(avg_ratio);
            }

            // Calculate mode batch size
            self.mode_batch_size = self.calculate_mode_batch_size();
        }

        self.clone()
    }

    /// Sets the worker pool metrics
    /// 
    /// This method merges the provided metrics with any existing worker pool metrics.
    /// 
    /// This is used to integrate metrics from the NodeJS sidecar with metrics
    /// collected during processing.
    pub fn set_worker_pool_metrics(&mut self, metrics: Option<WorkerPoolMetrics>) {
        if let Some(new_metrics) = metrics {
            // Get or create the existing metrics
            let worker_pool = self.worker_pool.get_or_insert_with(WorkerPoolMetrics::default);
            
            // Update with the new metrics - no need for redundant logging
            *worker_pool = new_metrics;
        } else {
            // If we're setting None but already have metrics, prefer to keep them
            if self.worker_pool.is_none() {
                self.worker_pool = None;
            }
        }
    }
}

// Implement core MetricsCollector trait for BenchmarkMetrics
impl MetricsCollector for BenchmarkMetrics {
    fn record_time(&mut self, duration_secs: f64) {
        self.record_processing_time(duration_secs);
    }
    
    fn record_size_change(&mut self, original_size: u64, optimized_size: u64) {
        self.record_compression(original_size, optimized_size);
    }
    
    fn record_batch_info(&mut self, batch_size: usize) {
        self.record_batch(batch_size);
    }
    
    fn record_worker_stats(&mut self, worker_count: usize, tasks_per_worker: Vec<usize>) {
        let metrics = WorkerPoolMetrics {
            worker_count,
            tasks_per_worker,
        };
        self.set_worker_pool_metrics(Some(metrics));
    }
    
    fn finalize(&mut self) -> Option<BenchmarkMetrics> {
        Some(self.finalize_metrics())
    }
}

/// Factory for creating the appropriate metrics collector based on configuration
pub struct MetricsFactory;

impl MetricsFactory {
    /// Create a metrics collector based on whether benchmarking is enabled
    pub fn create_collector(enable_benchmarking: bool) -> Box<dyn MetricsCollector> {
        if enable_benchmarking {
            let mut metrics = BenchmarkMetrics::default();
            metrics.start_benchmarking();
            Box::new(metrics)
        } else {
            Box::new(NullMetricsCollector::new())
        }
    }
    
    /// Extract benchmark metrics from a collector and create a reporter, if benchmarking is enabled
    pub fn extract_benchmark_metrics(
        enable_benchmarking: bool, 
        mut collector: Box<dyn MetricsCollector>
    ) -> Option<BenchmarkReporter> {
        if !enable_benchmarking {
            return None;
        }
        
        collector.finalize().map(BenchmarkReporter::from_metrics)
    }
} 