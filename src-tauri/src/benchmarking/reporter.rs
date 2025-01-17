use super::metrics::{BenchmarkMetrics, Percentage, Duration};
use std::fmt;

pub struct BenchmarkReporter {
    metrics: BenchmarkMetrics,
}

impl BenchmarkReporter {
    pub fn from_metrics(metrics: BenchmarkMetrics) -> Self {
        Self { metrics }
    }

    fn safe_div(numerator: f64, denominator: f64) -> f64 {
        if denominator == 0.0 {
            0.0
        } else {
            numerator / denominator
        }
    }

    fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", Self::safe_div(bytes as f64, GB as f64))
        } else if bytes >= MB {
            format!("{:.2} MB", Self::safe_div(bytes as f64, MB as f64))
        } else if bytes >= KB {
            format!("{:.2} KB", Self::safe_div(bytes as f64, KB as f64))
        } else {
            format!("{} B", bytes)
        }
    }

    fn calculate_average_compression(&self) -> Percentage {
        if self.metrics.compression_ratios.is_empty() {
            Percentage::zero()
        } else {
            Percentage::new_unchecked(
                Self::safe_div(
                    self.metrics.compression_ratios.iter().map(|p| p.as_f64()).sum::<f64>(),
                    self.metrics.compression_ratios.len() as f64
                )
            )
        }
    }

    fn calculate_average_processing_time(&self) -> Duration {
        if self.metrics.compression_ratios.is_empty() {
            self.metrics.avg_processing_time
        } else {
            let avg_time = self.metrics.total_duration.as_secs_f64() / self.metrics.compression_ratios.len() as f64;
            Duration::new_unchecked(avg_time)
        }
    }
}

impl fmt::Display for BenchmarkReporter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== Image Optimizer Benchmark Report ===")?;
        writeln!(f)?;
        
        // Time-based metrics
        writeln!(f, "Time-based Metrics:")?;
        writeln!(f, "- Total Duration: {}", self.metrics.total_duration)?;
        writeln!(f, "- Average Processing Time: {}/image", self.calculate_average_processing_time())?;
        writeln!(f, "- Processing Throughput: {:.2} MB/s", self.metrics.throughput_mbs)?;
        writeln!(f)?;
        
        // Worker Pool Metrics
        writeln!(f, "Worker Pool Metrics:")?;
        writeln!(f, "- Worker Distribution:")?;
        for (worker_id, tasks) in self.metrics.worker_pool.tasks_per_worker.iter().enumerate() {
            if *tasks > 0 {
                writeln!(f, "  └── Worker {}: {} tasks", worker_id, tasks)?;
            }
        }
        writeln!(f, "  └── Total Active Workers: {}", 
            self.metrics.worker_pool.tasks_per_worker.iter().filter(|&&tasks| tasks > 0).count()
        )?;
        writeln!(f)?;
        
        writeln!(f, "- Contention Metrics:")?;
        writeln!(f, "  └── Contention Events: {}", self.metrics.worker_pool.contention_count)?;
        writeln!(f, "  └── Peak Concurrent Tasks: {}", self.metrics.worker_pool.peak_concurrent_tasks)?;
        writeln!(f)?;
        
        writeln!(f, "- Task Statistics:")?;
        writeln!(f, "  └── Failed Tasks: {}", self.metrics.worker_pool.failed_tasks)?;
        writeln!(f)?;
        
        // Optimization metrics
        writeln!(f, "Optimization Metrics:")?;
        let avg_ratio = self.calculate_average_compression();
        writeln!(f, "- Compression Ratios:")?;
        writeln!(f, "  └── Average: {} (original → optimized)", avg_ratio)?;
        
        writeln!(f, "- Size Reductions:")?;
        writeln!(f, "  └── Total: {} → {}", 
            Self::format_bytes(self.metrics.total_original_size),
            Self::format_bytes(self.metrics.total_optimized_size)
        )?;
        
        Ok(())
    }
} 