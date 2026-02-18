// src-tauri/src/processing/libvips/resize.rs

//! Resize logic mapping ImageSettings resize modes to libvips operations.

use libvips::{ops, VipsImage};
use crate::core::ResizeSettings;
use crate::utils::OptimizerError;

type Result<T> = std::result::Result<T, OptimizerError>;

/// Applies the resize specified in `settings` to `image`.
///
/// Returns the original image unchanged when the mode is "none" or no
/// target size is provided. Uses `thumbnail_image` for all modes so that
/// libvips can apply its high-quality shrink-on-load optimisation.
pub fn apply_resize(image: VipsImage, settings: &ResizeSettings) -> Result<VipsImage> {
    if settings.mode == "none" {
        return Ok(image);
    }

    let size = match settings.size {
        Some(s) if s > 0 => s as i32,
        _ => return Ok(image),
    };

    let orig_w = image.get_width();
    let orig_h = image.get_height();

    match settings.mode.as_str() {
        "width" => resize_by_width(image, size),
        "height" => resize_by_height(image, size),
        "longest" => {
            if orig_w >= orig_h {
                resize_by_width(image, size)
            } else {
                resize_by_height(image, size)
            }
        }
        "shortest" => {
            if orig_w <= orig_h {
                resize_by_width(image, size)
            } else {
                resize_by_height(image, size)
            }
        }
        unknown => Err(OptimizerError::processing(format!(
            "Unknown resize mode: {unknown}"
        ))),
    }
}

/// Resizes so the width becomes `target_w` (height scales proportionally).
/// Will not enlarge the image if it is already smaller than the target.
fn resize_by_width(image: VipsImage, target_w: i32) -> Result<VipsImage> {
    use ops::{Size, ThumbnailImageOptions};

    let opts = ThumbnailImageOptions {
        size: Size::Down, // never upscale
        ..ThumbnailImageOptions::default()
    };

    ops::thumbnail_image_with_opts(&image, target_w, &opts)
        .map_err(|e| OptimizerError::processing(format!("Resize (width) failed: {e}")))
}

/// Resizes so the height becomes `target_h` (width scales proportionally).
/// Will not enlarge the image if it is already smaller than the target.
fn resize_by_height(image: VipsImage, target_h: i32) -> Result<VipsImage> {
    use ops::{Size, ThumbnailImageOptions};

    // Pass a very large width so the height constraint drives the scale.
    let opts = ThumbnailImageOptions {
        height: target_h,
        size: Size::Down,
        ..ThumbnailImageOptions::default()
    };

    ops::thumbnail_image_with_opts(&image, i32::MAX, &opts)
        .map_err(|e| OptimizerError::processing(format!("Resize (height) failed: {e}")))
}
