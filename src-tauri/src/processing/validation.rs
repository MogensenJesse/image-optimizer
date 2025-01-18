use crate::core::ImageTask;
use crate::utils::{OptimizerResult, validation::{validate_input_path, validate_output_path, validate_settings}};

pub async fn validate_task(task: &ImageTask) -> OptimizerResult<()> {
    // Validate input path
    validate_input_path(&task.input_path).await?;

    // Validate output path
    validate_output_path(&task.output_path).await?;

    // Validate settings
    validate_settings(&task.settings)?;

    Ok(())
} 