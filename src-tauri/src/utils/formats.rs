//! Image format detection and parsing.

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use crate::utils::OptimizerError;

/// Supported image formats for optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[allow(clippy::upper_case_acronyms)] // Standard naming for image formats
pub enum ImageFormat {
    /// JPEG format (lossy compression)
    JPEG,
    /// PNG format (lossless compression)
    PNG,
    /// WebP format (modern, efficient)
    WebP,
    /// AVIF format (next-gen, best compression)
    AVIF,
}

impl FromStr for ImageFormat {
    type Err = OptimizerError;

    fn from_str(ext: &str) -> Result<Self, Self::Err> {
        let ext = ext.to_lowercase();
        match ext.as_str() {
            "jpg" | "jpeg" => Ok(Self::JPEG),
            "png" => Ok(Self::PNG),
            "webp" => Ok(Self::WebP),
            "avif" => Ok(Self::AVIF),
            _ => Err(OptimizerError::format(format!(
                "Unsupported image format: {}", ext
            ))),
        }
    }
}

/// Determines the image format from a file path's extension.
///
/// # Arguments
/// * `path` - File path to check
///
/// # Returns
/// The detected [`ImageFormat`] or an error if unsupported.
pub fn format_from_extension(path: &str) -> Result<ImageFormat, OptimizerError> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| OptimizerError::format(
            format!("File has no extension: {}", path)
        ))?;
    
    ImageFormat::from_str(ext)
} 