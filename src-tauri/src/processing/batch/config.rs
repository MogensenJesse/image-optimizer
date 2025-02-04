use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSizeConfig {
    pub min_size: usize,
    pub max_size: usize,
    pub tasks_per_process: usize,
}

impl Default for BatchSizeConfig {
    fn default() -> Self {
        Self {
            min_size: 5,
            max_size: 75,
            tasks_per_process: 25,
        }
    }
} 