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