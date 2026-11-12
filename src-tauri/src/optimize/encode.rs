//! Maps the quality setting to libvips format-specific save options.

use libvips::ops::{
    self, ForeignHeifCompression, ForeignKeep, ForeignPngFilter, ForeignSubsample,
};
use libvips::VipsImage;

use super::vips_error_buffer_string;
use crate::error::{OptimizerError, OptimizerResult};
use crate::types::ImageFormat;

// ── Default quality constants ─────────────────────────────────────────────────────────

const PNG_COMPRESSION: i32 = 7; // 0-9
const PNG_EFFORT: i32 = 4;
const WEBP_EFFORT: i32 = 4;
const AVIF_EFFORT: i32 = 2;

/// Quality 100 asks for the best result the format can give.
fn is_lossless(quality: u8) -> bool {
    quality == 100
}

// ── Format save functions ──────────────────────────────────────────────────────────────

/// Saves `image` as JPEG with mozjpeg settings.
///
/// Always progressive with optimised scans (smaller files). Trellis
/// quantisation and overshoot deringing are only enabled at quality 100: below
/// that they made a 200-image batch ~1.6x slower for about 8% more savings
/// (200 synthetic 3000x2000 images, 8 cores: 6.0 s vs 9.5 s).
/// `optimize_scans` only takes effect together with `interlace`; without it the
/// output was baseline.
fn save_jpeg(image: &VipsImage, output_path: &str, quality: u8) -> OptimizerResult<()> {
    let lossless = is_lossless(quality);

    let opts = ops::JpegsaveOptions {
        q: quality as i32,
        optimize_coding: true,
        interlace: true,
        optimize_scans: true,
        trellis_quant: lossless,
        overshoot_deringing: lossless,
        quant_table: 3,
        // Auto lets libvips pick 4:4:4 at high quality and 4:2:0 at lower quality
        subsample_mode: ForeignSubsample::Auto,
        keep: ForeignKeep::None,
        ..ops::JpegsaveOptions::default()
    };

    ops::jpegsave_with_opts(image, output_path, &opts)
        .map_err(|_| OptimizerError::processing(format!("JPEG save failed: {}", vips_error_buffer_string())))
}

/// Saves `image` as PNG.
///
/// At quality 100: lossless, with max deflate compression and adaptive row
/// filtering. Otherwise: palette quantisation (`effort` and `q` only apply in
/// this mode) with fast defaults.
fn save_png(image: &VipsImage, output_path: &str, quality: u8) -> OptimizerResult<()> {
    let lossless = is_lossless(quality);

    let opts = ops::PngsaveOptions {
        compression: if lossless { 9 } else { PNG_COMPRESSION },
        palette: !lossless,
        q: quality as i32,
        effort: if lossless { 10 } else { PNG_EFFORT },
        filter: if lossless { ForeignPngFilter::All } else { ForeignPngFilter::None },
        keep: ForeignKeep::None,
        ..ops::PngsaveOptions::default()
    };

    ops::pngsave_with_opts(image, output_path, &opts)
        .map_err(|_| OptimizerError::processing(format!("PNG save failed: {}", vips_error_buffer_string())))
}

/// Saves `image` as WebP.
///
/// At quality 100: lossless with max effort. Otherwise: lossy with fast defaults.
fn save_webp(image: &VipsImage, output_path: &str, quality: u8) -> OptimizerResult<()> {
    let lossless = is_lossless(quality);
    let q = quality as i32;

    let opts = ops::WebpsaveOptions {
        q,
        lossless,
        alpha_q: q,
        effort: if lossless { 6 } else { WEBP_EFFORT },
        min_size: lossless,
        smart_subsample: false,
        keep: ForeignKeep::None,
        ..ops::WebpsaveOptions::default()
    };

    ops::webpsave_with_opts(image, output_path, &opts)
        .map_err(|_| OptimizerError::processing(format!("WebP save failed: {}", vips_error_buffer_string())))
}

/// Saves `image` as AVIF (AV1 via HEIF container).
///
/// At quality 100 we use q=100 with 4:4:4 chroma, max effort (9), and no
/// `lossless` flag. The `lossless` flag is avoided because AV1 lossless mode
/// applies an internal RGB->YCbCr conversion that produced visible color
/// shifts on some encoder builds (seen with Windows aom/svt-av1 in libvips
/// 8.18.0; not re-tested on 8.18.7).
fn save_avif(image: &VipsImage, output_path: &str, quality: u8) -> OptimizerResult<()> {
    let lossless = is_lossless(quality);
    let near_lossless = quality >= 90;

    let opts = ops::HeifsaveOptions {
        q: quality as i32,
        lossless: false,
        compression: ForeignHeifCompression::Av1,
        effort: if lossless { 9 } else { AVIF_EFFORT },
        bitdepth: 8,
        subsample_mode: if near_lossless { ForeignSubsample::Off } else { ForeignSubsample::On },
        keep: ForeignKeep::None,
        ..ops::HeifsaveOptions::default()
    };

    ops::heifsave_with_opts(image, output_path, &opts)
        .map_err(|_| OptimizerError::processing(format!("AVIF save failed: {}", vips_error_buffer_string())))
}

/// Saves `image` to `output_path` in `format`.
pub fn save_image_as(
    image: &VipsImage,
    output_path: &str,
    format: ImageFormat,
    quality: u8,
) -> OptimizerResult<()> {
    match format {
        ImageFormat::Jpeg => save_jpeg(image, output_path, quality),
        ImageFormat::Png => save_png(image, output_path, quality),
        ImageFormat::WebP => save_webp(image, output_path, quality),
        ImageFormat::Avif => save_avif(image, output_path, quality),
        ImageFormat::Svg => Err(OptimizerError::format("SVG cannot be saved as a raster image")),
    }
}
