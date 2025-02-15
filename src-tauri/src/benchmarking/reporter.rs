use super::metrics::{BenchmarkMetrics, Percentage, Duration};
use std::fmt;

pub struct BenchmarkReporter {
    metrics: BenchmarkMetrics,
}

impl BenchmarkReporter {
    #[allow(dead_code)]
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

    fn format_tasks_per_worker(tasks_per_worker: &[usize]) -> String {
        if tasks_per_worker.is_empty() {
            return "N/A".to_string();
        }

        let min = tasks_per_worker.iter().min().unwrap_or(&0);
        let max = tasks_per_worker.iter().max().unwrap_or(&0);
        let avg = Self::safe_div(
            tasks_per_worker.iter().sum::<usize>() as f64,
            tasks_per_worker.len() as f64
        );

        format!("min: {}, max: {}, avg: {:.1}", min, max, avg)
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
        writeln!(f)?;
        
        // Worker pool metrics
        if let Some(worker_metrics) = &self.metrics.worker_pool {
            writeln!(f, "Worker Pool Metrics:")?;
            writeln!(f, "- Total Workers: {}", worker_metrics.worker_count)?;
            writeln!(f, "- Active Workers: {}", worker_metrics.active_workers)?;
            writeln!(f, "- Tasks Distribution: {}", Self::format_tasks_per_worker(&worker_metrics.tasks_per_worker))?;
            writeln!(f, "- Queue Length: {}", worker_metrics.queue_length)?;
            writeln!(f, "- Completed Tasks: {}/{}", worker_metrics.completed_tasks, worker_metrics.total_tasks)?;
            writeln!(f, "- Processing Duration: {:.2}s", worker_metrics.duration_seconds)?;
            writeln!(f)?;
        }
        
        // Batch metrics
        writeln!(f, "Batch Metrics:")?;
        writeln!(f, "- Total Batches: {}", self.metrics.total_batches)?;
        if self.metrics.mode_batch_size > 0 {
            writeln!(f, "- Mode Batch Size: {} images", self.metrics.mode_batch_size)?;
        }
        writeln!(f)?;
        
        // Optimization metrics
        writeln!(f, "Optimization Metrics:")?;
        writeln!(f, "- Compression Ratios:")?;
        writeln!(f, "  └── Average: {} (original → optimized)", self.calculate_average_compression())?;
        
        writeln!(f, "- Size Reductions:")?;
        writeln!(f, "  └── Total: {} → {}", 
            Self::format_bytes(self.metrics.total_original_size),
            Self::format_bytes(self.metrics.total_optimized_size)
        )?;
        
        Ok(())
    }
} 