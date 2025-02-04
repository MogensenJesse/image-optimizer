use tracing::debug;

#[derive(Debug, Clone)]
pub struct BatchMemoryMetrics {
    pub initial_memory: usize,
    pub avg_batch_memory: usize,
    pub peak_pressure: usize,
    pub memory_distribution: [usize; 3],
}

impl BatchMemoryMetrics {
    pub fn new(initial_memory: usize) -> Self {
        Self {
            initial_memory,
            avg_batch_memory: 0,
            peak_pressure: 0,
            memory_distribution: [0; 3],
        }
    }

    pub fn record_usage(&mut self, used_memory: usize, available_memory: usize) {
        // Update average (exponential moving average with alpha=0.2)
        if self.avg_batch_memory == 0 {
            self.avg_batch_memory = used_memory;
        } else {
            self.avg_batch_memory = (used_memory / 5) + (self.avg_batch_memory * 4 / 5);
        }
        
        // Update peak pressure (track highest memory usage)
        self.peak_pressure = self.peak_pressure.max(used_memory);
        
        // Update distribution based on percentage of initial memory used
        let usage_pct = (used_memory as f64 / self.initial_memory as f64) * 100.0;
        let index = (usage_pct / 33.33).min(2.0) as usize;
        self.memory_distribution[index] += 1;

        debug!(
            "Memory usage recorded - Used: {}MB, Available: {}MB, Usage: {:.1}%, Index: {}", 
            used_memory / (1024 * 1024),
            available_memory / (1024 * 1024),
            usage_pct,
            index
        );
    }
} 