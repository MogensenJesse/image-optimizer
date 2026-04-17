// src-tauri/src/utils/validation.rs

use std::path::Path;
use crate::core::ImageTask;
use crate::utils::{ImageFormat, OptimizerResult, format_from_extension};
use crate::utils::error::ValidationError;
use tokio::fs;

/// Extract filename from a path
pub fn extract_filename(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
}

/// Validates an image processing task.
///
/// SVG tasks only validate path existence (quality is used for precision
/// mapping but doesn't need range validation). Raster tasks additionally
/// validate quality, resize, and output format settings.
pub async fn validate_task(task: &ImageTask) -> OptimizerResult<()> {
    let format = validate_input_path(&task.input_path).await?;
    validate_output_path(&task.output_path).await?;
    if format != ImageFormat::SVG {
        validate_settings(&task.settings)?;
    }
    Ok(())
}

/// Validates that an input path exists and is a supported format.
///
/// Returns the detected `ImageFormat` so callers can branch on SVG vs raster.
pub async fn validate_input_path(path: impl AsRef<Path>) -> OptimizerResult<ImageFormat> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(ValidationError::path_not_found(path).into());
    }
    if !path.is_file() {
        return Err(ValidationError::not_a_file(path).into());
    }
    let path_str = path.to_str()
        .ok_or_else(|| ValidationError::settings("Path contains invalid characters"))?;
    let format = format_from_extension(path_str)?;
    Ok(format)
}

/// Validates that an output path has a valid parent directory.
///
/// The output extension is not validated here because format conversions
/// (e.g., .jpg → .webp) correct the extension downstream in the executor.
pub async fn validate_output_path(path: impl AsRef<Path>) -> OptimizerResult<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent).await.map_err(|e| {
            ValidationError::Path(e.into())
        })?;
    }
    Ok(())
}

/// Validates raster image settings for quality and resize parameters.
///
/// Not called for SVG tasks since quality, resize, and format conversion
/// do not apply to vector graphics.
pub fn validate_settings(settings: &crate::core::ImageSettings) -> OptimizerResult<()> {
    if settings.quality.global == 0 || settings.quality.global > 100 {
        return Err(ValidationError::settings(
            format!("Invalid quality: {}. Must be between 1 and 100", settings.quality.global)
        ).into());
    }

    let format = settings.output_format.to_lowercase();
    if !["jpeg", "jpg", "png", "webp", "avif", "original"].contains(&format.as_str()) {
        return Err(ValidationError::settings(
            format!("Unsupported output format: {}", format)
        ).into());
    }

    if let Some(width) = settings.resize.width
        && width == 0
    {
        return Err(ValidationError::settings("Width cannot be 0").into());
    }
    if let Some(height) = settings.resize.height
        && height == 0
    {
        return Err(ValidationError::settings("Height cannot be 0").into());
    }
    Ok(())
}
