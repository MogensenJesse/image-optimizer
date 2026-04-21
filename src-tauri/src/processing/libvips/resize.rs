// src-tauri/src/processing/libvips/resize.rs

//! Resize logic mapping ImageSettings resize modes to libvips operations.

use libvips::{ops, VipsImage};
use crate::core::ResizeSettings;
use crate::utils::OptimizerError;
use super::vips_error_buffer_string;

type Result<T> = std::result::Result<T, OptimizerError>;

/// Returns `true` when `settings` will actually resize (mode is not "none"
/// and a valid target size is provided).
pub fn needs_resize(settings: &ResizeSettings) -> bool {
    settings.mode != "none" && matches!(settings.size, Some(s) if s > 0)
}

/// Loads and resizes an image from `path` in one step.
///
/// Uses the file-based `vips_thumbnail` which enables **shrink-on-load**:
/// for JPEG, libjpeg can skip decoding most DCT coefficients when
/// downsizing by integer factors (2x, 4x, 8x), making large-image
/// resizes significantly faster than loading first and resizing second.
///
/// For "width" and "height" modes, the target dimension is set directly
/// without probing the image first. The "longest" and "shortest" modes
/// require a probe to determine aspect ratio orientation.
pub fn load_and_resize(path: &str, settings: &ResizeSettings) -> Result<VipsImage> {
    let size = settings.size.unwrap_or(0) as i32;
    if size <= 0 {
        return Err(OptimizerError::processing("No target size for resize".to_string()));
    }

    // libvips thumbnail treats height=1 as "unconstrained by height" (the
    // default).  For height-only mode we pass the documented max width
    // (10 000 000) so the width constraint never fires.
    const UNCONSTRAINED: i32 = 10_000_000;

    match settings.mode.as_str() {
        "width" => thumbnail_file(path, size, 1, "width"),
        "height" => thumbnail_file(path, UNCONSTRAINED, size, "height"),
        "longest" | "shortest" => {
            let probe = VipsImage::new_from_file(path)
                .map_err(|_| OptimizerError::processing(format!(
                    "Failed to probe '{}': {}", path, vips_error_buffer_string()
                )))?;
            let orig_w = probe.get_width();
            let orig_h = probe.get_height();

            match settings.mode.as_str() {
                "longest" => {
                    if orig_w >= orig_h {
                        thumbnail_file(path, size, orig_h, "longest")
                    } else {
                        thumbnail_file(path, orig_w, size, "longest")
                    }
                }
                "shortest" => {
                    if orig_w <= orig_h {
                        thumbnail_file(path, size, orig_h, "shortest")
                    } else {
                        thumbnail_file(path, orig_w, size, "shortest")
                    }
                }
                _ => unreachable!(),
            }
        }
        unknown => Err(OptimizerError::processing(format!(
            "Unknown resize mode: {unknown}"
        ))),
    }
}

/// File-based thumbnail: loads and resizes in one step, enabling
/// shrink-on-load for formats that support it (JPEG, WebP, TIFF).
fn thumbnail_file(path: &str, target_w: i32, target_h: i32, mode: &str) -> Result<VipsImage> {
    use ops::{Size, ThumbnailOptions};

    let opts = ThumbnailOptions {
        height: target_h,
        size: Size::Down,
        import_profile: "sRGB".to_string(),
        ..ThumbnailOptions::default()
    };

    ops::thumbnail_with_opts(path, target_w, &opts)
        .map_err(|_| OptimizerError::processing(format!(
            "Resize ({mode}) failed: {}",
            vips_error_buffer_string()
        )))
}

