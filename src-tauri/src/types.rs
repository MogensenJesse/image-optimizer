//! Types shared by the command and the optimizer: settings, progress and results.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::{OptimizerError, OptimizerResult};

/// Image formats the app can read. SVG is optimized without re-encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Jpeg,
    Png,
    WebP,
    Avif,
    Svg,
}

impl ImageFormat {
    /// Format for a file extension, case-insensitive (`jpg` and `jpeg` are both JPEG).
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "png" => Some(Self::Png),
            "webp" => Some(Self::WebP),
            "avif" => Some(Self::Avif),
            "svg" => Some(Self::Svg),
            _ => None,
        }
    }

    /// Extension of files written in this format.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::WebP => "webp",
            Self::Avif => "avif",
            Self::Svg => "svg",
        }
    }

    /// Format of the file at `path`, judged by its extension.
    pub fn from_path(path: &str) -> OptimizerResult<Self> {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| OptimizerError::format(format!("File has no extension: {path}")))?;

        Self::from_extension(ext).ok_or_else(|| {
            OptimizerError::format(format!("Unsupported image format: {}", ext.to_lowercase()))
        })
    }
}

/// Format to write: keep each file's own format, or convert everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Original,
    Jpeg,
    Png,
    Webp,
    Avif,
}

impl OutputFormat {
    /// The format to convert to, or `None` to keep each file's own format.
    pub fn target(self) -> Option<ImageFormat> {
        match self {
            Self::Original => None,
            Self::Jpeg => Some(ImageFormat::Jpeg),
            Self::Png => Some(ImageFormat::Png),
            Self::Webp => Some(ImageFormat::WebP),
            Self::Avif => Some(ImageFormat::Avif),
        }
    }
}

/// Which side the resize `size` applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResizeMode {
    None,
    Width,
    Height,
    Longest,
    Shortest,
}

/// Resize settings. Images are only ever shrunk, never enlarged.
#[derive(Debug, Clone, Deserialize)]
pub struct Resize {
    pub mode: ResizeMode,
    /// Target size in pixels for the side picked by `mode`
    pub size: Option<u32>,
}

impl Resize {
    /// True when a mode and a valid target size are both set.
    pub fn is_active(&self) -> bool {
        self.mode != ResizeMode::None && matches!(self.size, Some(s) if s > 0)
    }
}

/// Settings shared by every file of a batch.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageSettings {
    /// Quality 1-100; 100 means lossless where the format allows it
    pub quality: u8,
    pub resize: Resize,
    pub output_format: OutputFormat,
}

/// Running totals of a batch, emitted as the `image_optimization_progress`
/// event after every finished file.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchProgress {
    /// Files finished so far, failed ones included
    pub completed: usize,
    /// Files in the batch
    pub total: usize,
    /// Files that could not be optimized
    pub failed: usize,
    /// Bytes saved by the successful files (negative if they grew)
    pub saved_bytes: i64,
    /// Combined original size of the successful files
    pub original_bytes: u64,
}

/// One file that could not be optimized.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileError {
    pub file_name: String,
    pub error: String,
}

/// Final outcome of a batch.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchSummary {
    #[serde(flatten)]
    pub progress: BatchProgress,
    /// Wall-clock time of the whole batch
    pub duration_ms: u64,
    /// The first failures (capped); `progress.failed` has the full count
    pub errors: Vec<FileError>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_map_to_formats_case_insensitively() {
        assert_eq!(ImageFormat::from_extension("JPG"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::from_extension("jpeg"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::from_extension("WebP"), Some(ImageFormat::WebP));
        assert_eq!(ImageFormat::from_extension("txt"), None);
    }

    #[test]
    fn unsupported_and_missing_extensions_are_format_errors() {
        assert!(ImageFormat::from_path("/p/notes.txt").is_err());
        assert!(ImageFormat::from_path("/p/noext").is_err());
        assert_eq!(ImageFormat::from_path("/p/a.PNG").unwrap(), ImageFormat::Png);
    }

    #[test]
    fn frontend_settings_json_deserializes() {
        let json = r#"{"quality":90,"resize":{"mode":"longest","size":1600},"outputFormat":"webp"}"#;
        let settings: ImageSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.quality, 90);
        assert_eq!(settings.resize.mode, ResizeMode::Longest);
        assert_eq!(settings.output_format, OutputFormat::Webp);
        assert!(settings.resize.is_active());
    }

    #[test]
    fn resize_needs_both_a_mode_and_a_size() {
        let none = Resize { mode: ResizeMode::None, size: Some(800) };
        let no_size = Resize { mode: ResizeMode::Width, size: None };
        let zero = Resize { mode: ResizeMode::Width, size: Some(0) };
        assert!(!none.is_active() && !no_size.is_active() && !zero.is_active());
    }
}
