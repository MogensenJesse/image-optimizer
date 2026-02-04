//! Image task definition and creation.

use serde::{Deserialize, Serialize};
use crate::core::ImageSettings;
use crate::core::types::{QualitySettings, ResizeSettings};
use crate::utils::{OptimizerError, OptimizerResult};

/// Represents a single image optimization task.
///
/// Contains the input/output paths and settings for processing one image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageTask {
    /// Path to the source image file
    pub input_path: String,
    /// Path where the optimized image will be written
    pub output_path: String,
    /// Optimization settings (quality, resize, format)
    pub settings: ImageSettings,
}

impl ImageTask {
    /// Creates a minimal task suitable for warming up the executor.
    ///
    /// Uses a bundled tiny JPEG image to initialize the Sharp sidecar
    /// without requiring user input.
    pub fn create_warmup_task() -> OptimizerResult<Self> {
        // Get the path to a temporary directory
        let temp_dir = std::env::temp_dir();
        
        // Create paths for input and output
        let input_path = temp_dir.join("warmup_input.jpg");
        let output_path = temp_dir.join("warmup_output.jpg");
        
        // Create a tiny 1x1 pixel JPEG file if it doesn't exist
        if !input_path.exists() {
            // Create a minimal 1x1 pixel JPEG file
            // Using the fs plugin to write a base64-encoded 1x1 JPEG
            let minimal_jpeg = include_bytes!("../../resources/warmup.jpg");
            std::fs::write(&input_path, minimal_jpeg)
                .map_err(|e| OptimizerError::processing(
                    format!("Failed to create warmup file: {}", e)
                ))?;
        }
        
        // Create task with minimal settings
        let task = Self {
            input_path: input_path.to_string_lossy().to_string(),
            output_path: output_path.to_string_lossy().to_string(),
            settings: ImageSettings {
                quality: QualitySettings {
                    global: 80,
                    jpeg: None,
                    png: None,
                    webp: None,
                    avif: None,
                },
                resize: ResizeSettings {
                    width: None,
                    height: None,
                    maintain_aspect: true,
                    mode: "none".to_string(),
                    size: None,
                },
                output_format: "original".to_string(),
            },
        };
        
        Ok(task)
    }
} 