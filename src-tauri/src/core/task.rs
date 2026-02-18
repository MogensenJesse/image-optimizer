//! Image task definition and creation.

use serde::{Deserialize, Serialize};
use crate::core::ImageSettings;

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