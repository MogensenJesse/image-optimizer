// src-tauri/src/processing/libvips/formats.rs

//! Maps ImageSettings quality values to libvips format-specific save options.

use libvips::ops::{
    self,
    ForeignHeifCompression, ForeignSubsample,
    ForeignKeep,
};
use crate::core::QualitySettings;
use crate::utils::OptimizerError;
use libvips::VipsImage;
use super::vips_error_buffer_string;

type Result<T> = std::result::Result<T, OptimizerError>;

// ── Default quality constants ─────────────────────────────────────────────────────────

const PNG_COMPRESSION: i32 = 7; // 0-9
const PNG_EFFORT: i32 = 4;
const WEBP_EFFORT: i32 = 4;
const AVIF_EFFORT: i32 = 2;

// ── Effective quality helpers ──────────────────────────────────────────────────────────

/// Returns the effective quality for a given format, respecting per-format overrides.
fn effective_quality(quality: &QualitySettings, format: &str) -> u32 {
    let per_format = match format {
        "jpeg" => quality.jpeg,
        "png" => quality.png,
        "webp" => quality.webp,
        "avif" => quality.avif,
        _ => None,
    };
    per_format.unwrap_or(quality.global)
}

/// Returns `true` when the effective quality for a format is 100 (lossless).
fn is_lossless(quality: &QualitySettings, format: &str) -> bool {
    effective_quality(quality, format) == 100
}

// ── Format save functions ──────────────────────────────────────────────────────────────

/// Saves `image` as JPEG with mozjpeg-equivalent settings.
///
/// When quality == 100: uses trellis quantisation + optimal scans (near-lossless).
/// Otherwise: standard optimised JPEG.
pub fn save_jpeg(image: &VipsImage, output_path: &str, quality: &QualitySettings) -> Result<()> {
    let q = effective_quality(quality, "jpeg") as i32;
    let lossless = is_lossless(quality, "jpeg");

    let opts = ops::JpegsaveOptions {
        q,
        optimize_coding: true,
        optimize_scans: true,
        // Trellis quantisation and overshoot deringing give mozjpeg-level quality
        trellis_quant: lossless,
        overshoot_deringing: lossless,
        // quant_table 3 = mozjpeg quantisation table (higher quality at same byte count)
        quant_table: 3,
        subsample_mode: ForeignSubsample::On, // 4:2:0 chroma subsampling
        keep: ForeignKeep::None,              // strip metadata
        ..ops::JpegsaveOptions::default()
    };

    ops::jpegsave_with_opts(image, output_path, &opts)
        .map_err(|_| OptimizerError::processing(format!("JPEG save failed: {}", vips_error_buffer_string())))
}

/// Saves `image` as PNG.
///
/// When quality == 100: lossless (no palette quantisation).
/// Otherwise: palette quantisation + adaptive compression.
pub fn save_png(image: &VipsImage, output_path: &str, quality: &QualitySettings) -> Result<()> {
    let q = effective_quality(quality, "png") as i32;
    let lossless = is_lossless(quality, "png");

    let opts = ops::PngsaveOptions {
        compression: PNG_COMPRESSION,
        palette: !lossless,
        q,
        effort: PNG_EFFORT,
        keep: ForeignKeep::None,
        ..ops::PngsaveOptions::default()
    };

    ops::pngsave_with_opts(image, output_path, &opts)
        .map_err(|_| OptimizerError::processing(format!("PNG save failed: {}", vips_error_buffer_string())))
}

/// Saves `image` as WebP.
///
/// When quality == 100: lossless mode.
/// Otherwise: lossy with smart subsampling.
pub fn save_webp(image: &VipsImage, output_path: &str, quality: &QualitySettings) -> Result<()> {
    let q = effective_quality(quality, "webp") as i32;
    let lossless = is_lossless(quality, "webp");

    let opts = ops::WebpsaveOptions {
        q,
        lossless,
        alpha_q: q,            // alpha quality matches overall quality
        effort: WEBP_EFFORT,
        smart_subsample: false,
        keep: ForeignKeep::None,
        ..ops::WebpsaveOptions::default()
    };

    ops::webpsave_with_opts(image, output_path, &opts)
        .map_err(|_| OptimizerError::processing(format!("WebP save failed: {}", vips_error_buffer_string())))
}

/// Saves `image` as AVIF (AV1 via HEIF container).
///
/// At quality 100 we use q=100 with 4:4:4 chroma (no subsampling) instead of
/// the encoder's `lossless` flag, because AV1 lossless mode applies an
/// internal RGB→YCbCr conversion that produces visible color shifts on some
/// encoder builds (notably the Windows aom/svt-av1 bundled with libvips 8.18).
pub fn save_avif(image: &VipsImage, output_path: &str, quality: &QualitySettings) -> Result<()> {
    let q = effective_quality(quality, "avif") as i32;
    let near_lossless = q >= 90;

    let opts = ops::HeifsaveOptions {
        q,
        lossless: false,
        compression: ForeignHeifCompression::Av1,
        effort: AVIF_EFFORT,
        bitdepth: 8,
        subsample_mode: if near_lossless { ForeignSubsample::Off } else { ForeignSubsample::On },
        keep: ForeignKeep::None,
        ..ops::HeifsaveOptions::default()
    };

    ops::heifsave_with_opts(image, output_path, &opts)
        .map_err(|_| OptimizerError::processing(format!("AVIF save failed: {}", vips_error_buffer_string())))
}

/// Dispatches to the correct format save function based on `format`.
///
/// `format` must be one of: `"jpeg"`, `"png"`, `"webp"`, `"avif"`.
pub fn save_image_as(
    image: &VipsImage,
    output_path: &str,
    format: &str,
    quality: &QualitySettings,
) -> Result<()> {
    match format {
        "jpeg" => save_jpeg(image, output_path, quality),
        "png" => save_png(image, output_path, quality),
        "webp" => save_webp(image, output_path, quality),
        "avif" => save_avif(image, output_path, quality),
        other => Err(OptimizerError::format(format!("Unsupported output format: {other}"))),
    }
}
