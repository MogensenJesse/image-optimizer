use std::path::Path;
use crate::core::ImageTask;
use crate::utils::{OptimizerError, OptimizerResult};

#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub error: Option<OptimizerError>,
}

pub struct ImageValidator;

impl ImageValidator {
    pub async fn validate_task(task: &ImageTask) -> ValidationResult {
        // Check input path
        if let Err(e) = Self::validate_input_path(&task.input_path) {
            return ValidationResult {
                is_valid: false,
                error: Some(e),
            };
        }

        // Check output path
        if let Err(e) = Self::validate_output_path(&task.output_path) {
            return ValidationResult {
                is_valid: false,
                error: Some(e),
            };
        }

        // Check settings
        if let Err(e) = Self::validate_settings(&task.settings) {
            return ValidationResult {
                is_valid: false,
                error: Some(e),
            };
        }

        ValidationResult {
            is_valid: true,
            error: None,
        }
    }

    fn validate_input_path(path: &str) -> OptimizerResult<()> {
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

        let extension = path.extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| OptimizerError::format(
                format!("Input file has no extension: {}", path.display())
            ))?;

        match extension.to_lowercase().as_str() {
            "jpg" | "jpeg" | "png" | "webp" | "avif" => Ok(()),
            _ => Err(OptimizerError::format(
                format!("Unsupported file format: {}", extension)
            )),
        }
    }

    fn validate_output_path(path: &str) -> OptimizerResult<()> {
        let path = Path::new(path);
        
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(OptimizerError::validation(
                    format!("Output directory does not exist: {}", parent.display())
                ));
            }
        }

        let extension = path.extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| OptimizerError::format(
                format!("Output file has no extension: {}", path.display())
            ))?;

        match extension.to_lowercase().as_str() {
            "jpg" | "jpeg" | "png" | "webp" | "avif" => Ok(()),
            _ => Err(OptimizerError::format(
                format!("Unsupported output format: {}", extension)
            )),
        }
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
} 