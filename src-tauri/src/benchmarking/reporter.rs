use super::metrics::{BenchmarkMetrics, Duration, Percentage};
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

    fn calculate_stage_percentage(&self, stage_time: Duration) -> Percentage {
        Percentage::new_unchecked(
            Self::safe_div(stage_time.as_secs_f64(), self.metrics.total_duration.as_secs_f64()) * 100.0
        )
    }

    fn calculate_worker_efficiency(&self, busy: Duration, total: Duration) -> Percentage {
        Percentage::new_unchecked(
            Self::safe_div(busy.as_secs_f64(), total.as_secs_f64()) * 100.0
        )
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
}

impl fmt::Display for BenchmarkReporter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== Image Optimizer Benchmark Report ===")?;
        writeln!(f)?;
        
        // Time-based metrics
        writeln!(f, "Time-based Metrics:")?;
        writeln!(f, "- Total Duration: {}", self.metrics.total_duration)?;
        writeln!(f, "- Average Processing Time: {}/image", self.metrics.avg_processing_time)?;
        writeln!(f, "- Worker Init Time: {}", self.metrics.worker_init_time)?;
        writeln!(f, "- Processing Throughput: {:.2} MB/s", self.metrics.throughput_mbs)?;
        writeln!(f)?;

        // Processing stage metrics
        writeln!(f, "Processing Stage Metrics:")?;
        writeln!(f, "- Loading Time: {} ({})", 
            self.metrics.loading_time,
            self.calculate_stage_percentage(self.metrics.loading_time)
        )?;
        writeln!(f, "- Optimization Time: {} ({})", 
            self.metrics.optimization_time,
            self.calculate_stage_percentage(self.metrics.optimization_time)
        )?;
        writeln!(f, "- Saving Time: {} ({})", 
            self.metrics.saving_time,
            self.calculate_stage_percentage(self.metrics.saving_time)
        )?;
        writeln!(f, "- Overhead Time: {} ({})", 
            self.metrics.overhead_time,
            self.calculate_stage_percentage(self.metrics.overhead_time)
        )?;
        writeln!(f)?;
        
        // Worker Pool Metrics
        writeln!(f, "Worker Pool Metrics:")?;
        writeln!(f, "- Queue Statistics:")?;
        writeln!(f, "  └── Max Length: {}", self.metrics.worker_pool.max_queue_length)?;
        writeln!(f, "  └── Average Length: {:.1}", self.metrics.worker_pool.avg_queue_length)?;
        writeln!(f)?;
        
        writeln!(f, "- Worker Utilization:")?;
        for (worker_id, (busy, idle)) in self.metrics.worker_pool.worker_busy_time.iter()
            .zip(self.metrics.worker_pool.worker_idle_time.iter())
            .enumerate() 
        {
            let total = *busy + *idle;
            let efficiency = self.calculate_worker_efficiency(*busy, total);
            writeln!(f, "  └── Worker {}: {} ({} tasks)", 
                worker_id,
                efficiency,
                self.metrics.worker_pool.tasks_per_worker
                    .get(worker_id)
                    .copied()
                    .unwrap_or(0)
            )?;
        }
        writeln!(f, "  └── Average Efficiency: {}", 
            self.metrics.worker_pool.avg_worker_efficiency
        )?;
        writeln!(f)?;
        
        writeln!(f, "- Contention Metrics:")?;
        writeln!(f, "  └── Contention Events: {}", self.metrics.worker_pool.contention_count)?;
        writeln!(f, "  └── Peak Concurrent Tasks: {}", self.metrics.worker_pool.peak_concurrent_tasks)?;
        writeln!(f)?;
        
        writeln!(f, "- Task Statistics:")?;
        writeln!(f, "  └── Failed Tasks: {}", self.metrics.worker_pool.failed_tasks)?;
        writeln!(f, "  └── Retried Tasks: {}", self.metrics.worker_pool.retried_tasks)?;
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