//! Core types for image optimization settings and results.

use serde::{Deserialize, Serialize};

/// Configuration settings for image optimization.
///
/// Contains quality, resize, and output format settings that control
/// how images are processed by the Sharp sidecar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSettings {
    /// Quality settings for compression
    pub quality: QualitySettings,
    /// Resize settings for image dimensions
    pub resize: ResizeSettings,
    /// Output format (jpeg, png, webp, avif, or "original")
    #[serde(rename = "outputFormat")]
    pub output_format: String,
}

/// Quality settings for image compression.
///
/// Allows setting a global quality level and format-specific overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitySettings {
    /// Global quality level (1-100) applied when format-specific is not set
    pub global: u32,
    /// JPEG-specific quality override
    pub jpeg: Option<u32>,
    /// PNG-specific quality override
    pub png: Option<u32>,
    /// WebP-specific quality override
    pub webp: Option<u32>,
    /// AVIF-specific quality override
    pub avif: Option<u32>,
}

/// Resize settings for image dimensions.
///
/// Supports multiple resize modes: width, height, longest side, shortest side.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResizeSettings {
    /// Target width in pixels
    pub width: Option<u32>,
    /// Target height in pixels
    pub height: Option<u32>,
    /// Whether to maintain aspect ratio when resizing
    #[serde(rename = "maintainAspect")]
    pub maintain_aspect: bool,
    /// Resize mode: "none", "width", "height", "longest", "shortest"
    pub mode: String,
    /// Target size for longest/shortest side modes
    pub size: Option<u32>,
}

/// Result of an image optimization operation.
///
/// Contains the original and optimized file information along with
/// compression statistics.
#[derive(Debug, Clone, Serialize)]
pub struct OptimizationResult {
    /// Path to the original input file
    pub original_path: String,
    /// Path to the optimized output file
    pub optimized_path: String,
    /// Original file size in bytes
    pub original_size: u64,
    /// Optimized file size in bytes
    pub optimized_size: u64,
    /// Whether the optimization succeeded
    pub success: bool,
    /// Error message if optimization failed
    pub error: Option<String>,
    /// Bytes saved (can be negative if file grew)
    #[serde(rename = "savedBytes")]
    pub saved_bytes: i64,
    /// Compression ratio as a percentage
    #[serde(rename = "compressionRatio")]
    pub compression_ratio: f64,
}

/// Result of a benchmark run measuring optimization throughput.
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkResult {
    /// Total wall-clock time for the entire batch in milliseconds
    #[serde(rename = "totalTimeMs")]
    pub total_time_ms: u64,
    /// Average time per image in milliseconds
    #[serde(rename = "avgPerImageMs")]
    pub avg_per_image_ms: f64,
    /// Throughput in images per second
    #[serde(rename = "throughputImagesPerSec")]
    pub throughput_images_per_sec: f64,
    /// Number of images processed
    #[serde(rename = "imageCount")]
    pub image_count: usize,
    /// Total input bytes across all images
    #[serde(rename = "totalInputBytes")]
    pub total_input_bytes: u64,
    /// Total output bytes across all images
    #[serde(rename = "totalOutputBytes")]
    pub total_output_bytes: u64,
} 