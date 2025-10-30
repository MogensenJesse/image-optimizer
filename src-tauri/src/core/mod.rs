mod state;
mod types;
mod task;
mod progress;

pub use state::AppState;
pub use types::{ImageSettings, OptimizationResult};
pub use task::ImageTask;
pub use progress::{Progress, ProgressType}; 