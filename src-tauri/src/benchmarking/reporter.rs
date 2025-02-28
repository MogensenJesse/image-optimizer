use super::metrics::{BenchmarkMetrics, validations};
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
        writeln!(f, "- Total Duration: {}", validations::format_duration(self.metrics.total_duration))?;
        writeln!(f)?;
        
        // Worker pool metrics
        if let Some(worker_metrics) = &self.metrics.worker_pool {
            writeln!(f, "Worker Pool Metrics:")?;
            writeln!(f, "- Total Workers: {}", worker_metrics.worker_count)?;
            writeln!(f, "- Tasks Distribution: {}", Self::format_tasks_per_worker(&worker_metrics.tasks_per_worker))?;
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
        writeln!(f, "- Compression Ratio: {} (original → optimized)", validations::format_percentage(self.metrics.avg_compression_ratio))?;
        
        writeln!(f, "- Size Reductions:")?;
        writeln!(f, "  └── Total: {} → {}", 
            Self::format_bytes(self.metrics.total_original_size),
            Self::format_bytes(self.metrics.total_optimized_size)
        )?;
        
        // Calculate and display size savings as a percentage
        let bytes_saved = self.metrics.total_original_size.saturating_sub(self.metrics.total_optimized_size);
        let savings_percentage = if self.metrics.total_original_size > 0 {
            (bytes_saved as f64 / self.metrics.total_original_size as f64) * 100.0
        } else {
            0.0
        };
        
        writeln!(f, "  └── Saved: {} ({})", 
            Self::format_bytes(bytes_saved),
            validations::format_percentage(savings_percentage)
        )?;
        
        Ok(())
    }
} 