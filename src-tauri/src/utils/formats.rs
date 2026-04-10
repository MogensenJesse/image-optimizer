// src-tauri/src/utils/formats.rs

//! Image format detection and parsing.

use std::str::FromStr;
use crate::utils::OptimizerError;

/// Supported image formats for optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum ImageFormat {
    /// JPEG format (lossy compression)
    JPEG,
    /// PNG format (lossless compression)
    PNG,
    /// WebP format (modern, efficient)
    WebP,
    /// AVIF format (next-gen, best compression)
    AVIF,
    /// SVG format (vector, lossless optimization only)
    SVG,
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
            "svg" => Ok(Self::SVG),
            _ => Err(OptimizerError::format(format!(
                "Unsupported image format: {}", ext
            ))),
        }
    }
}

/// Parses the file extension from a path and returns the corresponding `ImageFormat`.
pub fn format_from_extension(path: &str) -> Result<ImageFormat, OptimizerError> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| OptimizerError::format(
            format!("File has no extension: {}", path)
        ))?;

    ImageFormat::from_str(ext)
}

/// Normalizes a format string: lowercases and maps "jpg" to "jpeg".
pub fn normalize_format(fmt: &str) -> String {
    match fmt.to_lowercase().as_str() {
        "jpg" => "jpeg".to_string(),
        other => other.to_string(),
    }
}
