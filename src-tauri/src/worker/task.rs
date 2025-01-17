use std::path::Path;
use serde::Serialize;
use crate::core::ImageSettings;
use crate::utils::{OptimizerResult, OptimizerError, format_from_extension};

#[derive(Debug, Clone, Serialize)]
pub struct ImageTask {
    pub input_path: String,
    pub output_path: String,
    pub settings: ImageSettings,
}

impl ImageTask {
    pub async fn validate(&self) -> OptimizerResult<()> {
        self.validate_input_path().await?;
        self.validate_output_path().await?;
        self.validate_settings()?;
        Ok(())
    }

    async fn validate_input_path(&self) -> OptimizerResult<()> {
        let path = Path::new(&self.input_path);
        
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

    async fn validate_output_path(&self) -> OptimizerResult<()> {
        let path = Path::new(&self.output_path);
        
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

    fn validate_settings(&self) -> OptimizerResult<()> {
        // Validate quality settings
        if self.settings.quality.global == 0 || self.settings.quality.global > 100 {
            return Err(OptimizerError::validation(
                format!("Invalid quality value: {}. Must be between 1 and 100", self.settings.quality.global)
            ));
        }

        // Validate resize settings
        if let Some(width) = self.settings.resize.width {
            if width == 0 {
                return Err(OptimizerError::validation("Width cannot be 0"));
            }
        }

        if let Some(height) = self.settings.resize.height {
            if height == 0 {
                return Err(OptimizerError::validation("Height cannot be 0"));
            }
        }

        if let Some(size) = self.settings.resize.size {
            if size == 0 {
                return Err(OptimizerError::validation("Size cannot be 0"));
            }
        }

        Ok(())
    }
}
