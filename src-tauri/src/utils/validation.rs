// src-tauri/src/utils/validation.rs

use std::path::Path;
use crate::core::ImageTask;
use crate::utils::{ImageFormat, OptimizerResult, format_from_extension};
use crate::utils::error::ValidationError;
use tokio::fs;

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
    let metadata = fs::metadata(path).await.map_err(|_| {
        ValidationError::path_not_found(path)
    })?;
    if !metadata.is_file() {
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
/// (e.g., .jpg to .webp) correct the extension downstream in the executor.
pub async fn validate_output_path(path: impl AsRef<Path>) -> OptimizerResult<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        let exists = fs::try_exists(parent).await.unwrap_or(false);
        if !exists {
            fs::create_dir_all(parent).await.map_err(|e| {
                ValidationError::Path(e.into())
            })?;
        }
    }
    Ok(())
}

/// Valid resize modes accepted by the processing pipeline.
const VALID_RESIZE_MODES: &[&str] = &["none", "width", "height", "longest", "shortest"];

/// Validates raster image settings for quality, resize, and output format.
///
/// Not called for SVG tasks since quality, resize, and format conversion
/// do not apply to vector graphics.
pub fn validate_settings(settings: &crate::core::ImageSettings) -> OptimizerResult<()> {
    validate_quality_range("global", settings.quality.global)?;

    for (name, value) in [
        ("jpeg", settings.quality.jpeg),
        ("png", settings.quality.png),
        ("webp", settings.quality.webp),
        ("avif", settings.quality.avif),
    ] {
        if let Some(v) = value {
            validate_quality_range(name, v)?;
        }
    }

    let format = settings.output_format.to_lowercase();
    if !["jpeg", "jpg", "png", "webp", "avif", "original"].contains(&format.as_str()) {
        return Err(ValidationError::settings(
            format!("Unsupported output format: {}", format)
        ).into());
    }

    if !VALID_RESIZE_MODES.contains(&settings.resize.mode.as_str()) {
        return Err(ValidationError::settings(
            format!("Unknown resize mode: '{}'. Must be one of: {}", settings.resize.mode, VALID_RESIZE_MODES.join(", "))
        ).into());
    }

    Ok(())
}

fn validate_quality_range(name: &str, value: u32) -> OptimizerResult<()> {
    if value == 0 || value > 100 {
        return Err(ValidationError::settings(
            format!("Invalid {name} quality: {value}. Must be between 1 and 100")
        ).into());
    }
    Ok(())
}
