use crate::core::ImageTask;
use crate::utils::{OptimizerError, OptimizerResult, validate_input_path, validate_output_path, ensure_parent_dir};

pub async fn validate_task(task: &ImageTask) -> OptimizerResult<()> {
    // Validate input path
    validate_input_path(&task.input_path).await?;

    // Validate output path and ensure directory exists
    validate_output_path(&task.output_path).await?;
    ensure_parent_dir(&task.output_path).await?;

    // Validate settings
    validate_settings(&task.settings)?;

    Ok(())
}

fn validate_settings(settings: &crate::core::ImageSettings) -> OptimizerResult<()> {
    if settings.quality.global == 0 || settings.quality.global > 100 {
        return Err(OptimizerError::validation(
            format!("Invalid quality value: {}. Must be between 1 and 100", settings.quality.global)
        ));
    }

    if let Some(width) = settings.resize.width {
        if width == 0 {
            return Err(OptimizerError::validation("Width cannot be 0"));
        }
    }

    if let Some(height) = settings.resize.height {
        if height == 0 {
            return Err(OptimizerError::validation("Height cannot be 0"));
        }
    }

    Ok(())
} 