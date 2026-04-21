// src-tauri/src/utils/formats.rs

//! Image format detection, parsing, and path helpers.

use std::borrow::Cow;
use std::path::Path;
use std::str::FromStr;
use crate::utils::OptimizerError;
use crate::utils::OptimizerResult;

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
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| OptimizerError::format(
            format!("File has no extension: {}", path)
        ))?;

    ImageFormat::from_str(ext)
}

/// Normalizes a format string: lowercases and maps "jpg" to "jpeg".
///
/// Returns a borrowed `&str` when the input already matches a known format,
/// avoiding heap allocation in the common case.
pub fn normalize_format(fmt: &str) -> Cow<'_, str> {
    match fmt.to_lowercase().as_str() {
        "jpg" => Cow::Borrowed("jpeg"),
        _ if fmt.chars().all(|c| c.is_ascii_lowercase()) => Cow::Borrowed(fmt),
        _ => Cow::Owned(fmt.to_lowercase()),
    }
}

/// Extracts the filename component from a path string.
pub fn extract_filename(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
}

/// Resolves "original" to the actual input format and normalizes "jpg" to "jpeg".
pub fn resolve_output_format(input_path: &str, requested: &str) -> OptimizerResult<String> {
    if requested == "original" {
        let ext = Path::new(input_path)
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| OptimizerError::format("Input file has no extension"))?;

        return Ok(normalize_format(ext).into_owned());
    }

    Ok(normalize_format(requested).into_owned())
}

/// Returns `output_path` with the extension corrected to match `format`.
///
/// When the output format differs from the extension already on `output_path`
/// (e.g. converting foo.jpg to webp), the extension is replaced.
pub fn ensure_correct_extension(output_path: &str, input_path: &str, format: &str) -> String {
    let new_ext = match format {
        "jpeg" => "jpg",
        other => other,
    };

    let path = Path::new(output_path);
    let current_ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if normalize_format(&current_ext) == format {
        return output_path.to_string();
    }

    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let parent = path.parent().unwrap_or(Path::new(""));

    let stem = if stem.is_empty() {
        Path::new(input_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output")
    } else {
        stem
    };

    parent
        .join(format!("{stem}.{new_ext}"))
        .to_string_lossy()
        .to_string()
}
