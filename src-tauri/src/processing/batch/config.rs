#[derive(Debug, Clone)]
pub struct BatchSizeConfig {
    pub min_size: usize,
    pub max_size: usize,
    pub target_memory_usage: usize,
    pub target_memory_percentage: f32,
    pub tasks_per_process: usize,
}

impl Default for BatchSizeConfig {
    fn default() -> Self {
        Self {
            min_size: 10,
            max_size: 75,
            target_memory_usage: 1024 * 1024 * 4096,  // 4GB limit
            target_memory_percentage: 0.7,     // 70% of available memory
            tasks_per_process: 20,            // Target 20 tasks per process
        }
    }
} 