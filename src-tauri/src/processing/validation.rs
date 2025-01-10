use std::path::Path;
use crate::worker::ImageTask;

pub struct ValidationResult {
    pub is_valid: bool,
    pub error: Option<String>,
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

    fn validate_input_path(path: &str) -> Result<(), String> {
        let path = Path::new(path);
        
        if !path.exists() {
            return Err(format!("Input file does not exist: {}", path.display()));
        }

        if !path.is_file() {
            return Err(format!("Input path is not a file: {}", path.display()));
        }

        let extension = path.extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| format!("Input file has no extension: {}", path.display()))?;

        match extension.to_lowercase().as_str() {
            "jpg" | "jpeg" | "png" | "webp" | "avif" => Ok(()),
            _ => Err(format!("Unsupported file format: {}", extension)),
        }
    }

    fn validate_output_path(path: &str) -> Result<(), String> {
        let path = Path::new(path);
        
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(format!("Output directory does not exist: {}", parent.display()));
            }
        }

        let extension = path.extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| format!("Output file has no extension: {}", path.display()))?;

        match extension.to_lowercase().as_str() {
            "jpg" | "jpeg" | "png" | "webp" | "avif" => Ok(()),
            _ => Err(format!("Unsupported output format: {}", extension)),
        }
    }

    fn validate_settings(settings: &crate::core::ImageSettings) -> Result<(), String> {
        if settings.quality.global == 0 || settings.quality.global > 100 {
            return Err(format!("Invalid quality value: {}. Must be between 1 and 100", settings.quality.global));
        }

        if let Some(width) = settings.resize.width {
            if width == 0 {
                return Err("Width cannot be 0".to_string());
            }
        }

        if let Some(height) = settings.resize.height {
            if height == 0 {
                return Err("Height cannot be 0".to_string());
            }
        }

        Ok(())
    }
} 