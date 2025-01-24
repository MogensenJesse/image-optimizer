use std::path::Path;
use crate::worker::ImageTask;
use crate::utils::{OptimizerResult, format_from_extension};
use crate::utils::error::ValidationError;
use tokio::fs;

/// Validates an image processing task
pub async fn validate_task(task: &ImageTask) -> OptimizerResult<()> {
    validate_input_path(&task.input_path).await?;
    validate_output_path(&task.output_path).await?;
    validate_settings(&task.settings)?;
    Ok(())
}

/// Validates that an input path exists and is a valid image file
pub async fn validate_input_path(path: impl AsRef<Path>) -> OptimizerResult<()> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(ValidationError::path_not_found(path).into());
    }
    if !path.is_file() {
        return Err(ValidationError::not_a_file(path).into());
    }
    format_from_extension(path.to_str().unwrap_or_default())?;
    Ok(())
}

/// Validates that an output path has a valid parent directory and format
pub async fn validate_output_path(path: impl AsRef<Path>) -> OptimizerResult<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).await.map_err(|e| {
                ValidationError::Path(e.into())
            })?;
        }
    }
    format_from_extension(path.to_str().unwrap_or_default())?;
    Ok(())
}

/// Validates image settings for quality and resize parameters
pub fn validate_settings(settings: &crate::core::ImageSettings) -> OptimizerResult<()> {
    // No extra validations beyond current requirements
    if settings.quality.global == 0 || settings.quality.global > 100 {
        return Err(ValidationError::settings(
            format!("Invalid quality: {}. Must be between 1 and 100", settings.quality.global)
        ).into());
    }

    // Validate output format
    let format = settings.output_format.to_lowercase();
    if !["jpeg", "jpg", "png", "webp", "avif", "original"].contains(&format.as_str()) {
        return Err(ValidationError::settings(
            format!("Unsupported output format: {}", format)
        ).into());
    }

    // Resize validation
    if let Some(width) = settings.resize.width {
        if width == 0 {
            return Err(ValidationError::settings("Width cannot be 0").into());
        }
    }
    if let Some(height) = settings.resize.height {
        if height == 0 {
            return Err(ValidationError::settings("Height cannot be 0").into());
        }
    }
    Ok(())
} 