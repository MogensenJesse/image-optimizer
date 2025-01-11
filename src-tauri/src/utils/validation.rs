use std::path::Path;
use crate::core::{ImageSettings, ImageTask};
use crate::utils::{OptimizerError, OptimizerResult, format_from_extension};

/// Validates an image processing task
pub async fn validate_task(task: &ImageTask) -> OptimizerResult<()> {
    validate_input_path(&task.input_path)?;
    validate_output_path(&task.output_path)?;
    validate_settings(&task.settings)?;
    Ok(())
}

/// Validates the input file path and format
pub fn validate_input_path(path: &str) -> OptimizerResult<()> {
    let path = Path::new(path);
    
    if !path.exists() {
        return Err(OptimizerError::validation(
            format!("Input file does not exist: {}", path.display())
        ));
    }

    if !path.is_file() {
        return Err(OptimizerError::validation(
            format!("Input path is not a file: {}", path.display())
        ));
    }

    // This will validate the extension and format
    format_from_extension(path.to_str().unwrap_or_default())?;
    Ok(())
}

/// Validates the output file path and format
pub fn validate_output_path(path: &str) -> OptimizerResult<()> {
    let path = Path::new(path);
    
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            return Err(OptimizerError::validation(
                format!("Output directory does not exist: {}", parent.display())
            ));
        }
    }

    // This will validate the extension and format
    format_from_extension(path.to_str().unwrap_or_default())?;
    Ok(())
}

/// Validates image processing settings
pub fn validate_settings(settings: &ImageSettings) -> OptimizerResult<()> {
    // Validate quality settings
    if settings.quality.global == 0 || settings.quality.global > 100 {
        return Err(OptimizerError::validation(
            format!("Invalid quality value: {}. Must be between 1 and 100", settings.quality.global)
        ));
    }

    // Validate resize settings
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

    if let Some(size) = settings.resize.size {
        if size == 0 {
            return Err(OptimizerError::validation("Size cannot be 0"));
        }
    }

    Ok(())
} 