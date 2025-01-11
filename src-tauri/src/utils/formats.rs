use serde::{Deserialize, Serialize};
use std::str::FromStr;
use crate::utils::OptimizerError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    JPEG,
    PNG,
    WebP,
    AVIF,
}

impl ImageFormat {
    /// Get the default quality value for this format
    pub fn default_quality(&self) -> u32 {
        match self {
            Self::JPEG => 85,
            Self::PNG => 100,  // PNG uses compression level instead
            Self::WebP => 80,
            Self::AVIF => 70,
        }
    }

    /// Validate quality value for this format
    pub fn validate_quality(&self, quality: u32) -> bool {
        match self {
            Self::PNG => quality <= 100,  // PNG uses compression level 0-100
            _ => quality > 0 && quality <= 100,
        }
    }

    /// Get file extensions associated with this format
    pub fn extensions(&self) -> &[&str] {
        match self {
            Self::JPEG => &["jpg", "jpeg"],
            Self::PNG => &["png"],
            Self::WebP => &["webp"],
            Self::AVIF => &["avif"],
        }
    }

    /// Check if the extension matches this format
    pub fn matches_extension(&self, ext: &str) -> bool {
        let ext = ext.to_lowercase();
        self.extensions().contains(&ext.as_str())
    }

    /// Get the primary extension for this format
    pub fn primary_extension(&self) -> &str {
        self.extensions()[0]
    }
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

/// Get format from file extension
pub fn format_from_extension(path: &str) -> Result<ImageFormat, OptimizerError> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| OptimizerError::format(
            format!("File has no extension: {}", path)
        ))?;
    
    ImageFormat::from_str(ext)
}

/// Check if two formats are compatible for conversion
#[allow(unused_variables)]  // Parameters kept for API clarity
pub fn is_compatible_conversion(from: ImageFormat, to: ImageFormat) -> bool {
    // All formats can be converted between each other
    true
}

/// Get optimal quality for conversion between formats
#[allow(unused_variables)]  // 'from' parameter kept for future format-specific optimizations
pub fn get_conversion_quality(from: ImageFormat, to: ImageFormat, requested_quality: Option<u32>) -> u32 {
    match to {
        // When converting to lossy formats, use format-specific defaults
        ImageFormat::JPEG => requested_quality.unwrap_or(92),
        ImageFormat::WebP => requested_quality.unwrap_or(90),
        ImageFormat::AVIF => requested_quality.unwrap_or(85),
        ImageFormat::PNG => requested_quality.unwrap_or(100),
    }
} 