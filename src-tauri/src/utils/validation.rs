use crate::worker::ImageTask;
use crate::utils::OptimizerResult;

/// Validates an image processing task
pub async fn validate_task(task: &ImageTask) -> OptimizerResult<()> {
    task.validate().await
} 