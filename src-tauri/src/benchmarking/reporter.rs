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
        
        // Batch Size Metrics
        writeln!(f, "Batch Size Metrics:")?;
        writeln!(f, "- Average Batch Size: {:.1}", self.metrics.batch_metrics.average())?;
        writeln!(f, "- Minimum Batch Size: {}", self.metrics.batch_metrics.min())?;
        writeln!(f, "- Maximum Batch Size: {}", self.metrics.batch_metrics.max())?;
        
        let range = (self.metrics.batch_metrics.max_size() - 
                    self.metrics.batch_metrics.min_size()) / 3;
        
        writeln!(f, "- Batch Size Distribution:")?;
        for i in 0..3 {
            let start = self.metrics.batch_metrics.min_size() + (i * range);
            let end = start + range;
            writeln!(f, "  └── {}-{}: {}", 
                start, end, 
                self.metrics.batch_metrics.size_distribution[i]
            )?;
        }
        writeln!(f)?;
        
        // Add memory metrics section
        writeln!(f, "Memory Usage Metrics:")?;
        writeln!(f, "- Initial Available Memory: {}MB", 
            self.metrics.batch_metrics.memory_metrics.initial_memory / (1024 * 1024))?;
        writeln!(f, "- Average Batch Memory Usage: {}MB", 
            self.metrics.batch_metrics.memory_metrics.avg_batch_memory / (1024 * 1024))?;
        writeln!(f, "- Maximum Batch Memory Usage: {}MB", 
            self.metrics.batch_metrics.memory_metrics.max_batch_memory / (1024 * 1024))?;
        writeln!(f, "- Peak Memory Pressure: {}MB", 
            self.metrics.batch_metrics.memory_metrics.peak_pressure / (1024 * 1024))?;
        
        writeln!(f, "- Memory Usage Distribution:")?;
        writeln!(f, "  └── 0-33%: {}", self.metrics.batch_metrics.memory_metrics.memory_distribution[0])?;
        writeln!(f, "  └── 34-66%: {}", self.metrics.batch_metrics.memory_metrics.memory_distribution[1])?;
        writeln!(f, "  └── 67-100%: {}", self.metrics.batch_metrics.memory_metrics.memory_distribution[2])?;
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
        
        // Add Process Pool Metrics section
        if let Some(process_pool) = &self.metrics.process_pool {
            writeln!(f)?;
            writeln!(f, "Process Pool Metrics:")?;
            writeln!(f, "- Total Processes Spawned: {}", process_pool.total_spawns)?;
            
            // Average spawn time
            if !process_pool.spawn_times.is_empty() {
                let avg_spawn_time = process_pool.spawn_times.iter()
                    .map(|d| d.as_secs_f64())
                    .sum::<f64>() / process_pool.spawn_times.len() as f64;
                writeln!(f, "- Average Spawn Time: {:.2}ms", avg_spawn_time * 1000.0)?;
            }
            
            // Process utilization
            if !process_pool.active_processes.is_empty() {
                let avg_active = process_pool.active_processes.iter().sum::<usize>() as f64 
                    / process_pool.active_processes.len() as f64;
                let max_active = process_pool.active_processes.iter().max().unwrap_or(&0);
                writeln!(f, "- Process Utilization:")?;
                writeln!(f, "  └── Average Active: {:.1}", avg_active)?;
                writeln!(f, "  └── Peak Active: {}", max_active)?;
            }
        }
        
        Ok(())
    }
} 