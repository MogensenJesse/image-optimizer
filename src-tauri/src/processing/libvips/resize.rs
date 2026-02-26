// src-tauri/src/processing/libvips/resize.rs

//! Resize logic mapping ImageSettings resize modes to libvips operations.

use libvips::{ops, VipsImage};
use crate::core::ResizeSettings;
use crate::utils::OptimizerError;
use super::vips_error_buffer_string;

type Result<T> = std::result::Result<T, OptimizerError>;

/// Applies the resize specified in `settings` to `image`.
///
/// Returns the original image unchanged when the mode is "none" or no
/// target size is provided. Uses `thumbnail_image` for all modes so that
/// libvips can apply its high-quality shrink+reduce pipeline.
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
        "width" => thumbnail(&image, size, orig_h, "width"),
        "height" => thumbnail(&image, orig_w, size, "height"),
        "longest" => {
            if orig_w >= orig_h {
                thumbnail(&image, size, orig_h, "longest")
            } else {
                thumbnail(&image, orig_w, size, "longest")
            }
        }
        "shortest" => {
            if orig_w <= orig_h {
                thumbnail(&image, size, orig_h, "shortest")
            } else {
                thumbnail(&image, orig_w, size, "shortest")
            }
        }
        unknown => Err(OptimizerError::processing(format!(
            "Unknown resize mode: {unknown}"
        ))),
    }
}

/// Runs `thumbnail_image` with explicit width and height constraints.
///
/// `Size::Down` prevents upscaling when the image is already smaller than
/// the target. Passing both width and height avoids the need for sentinel
/// values that libvips may reject.
///
/// `import_profile` is set to `"sRGB"` because libvips-rs unconditionally
/// passes the option to the C API. An empty string (the default) causes
/// libvips to try opening a file named `""`, which fails for images that
/// need profile conversion.
fn thumbnail(image: &VipsImage, target_w: i32, target_h: i32, mode: &str) -> Result<VipsImage> {
    use ops::{Size, ThumbnailImageOptions};

    let opts = ThumbnailImageOptions {
        height: target_h,
        size: Size::Down,
        import_profile: "sRGB".to_string(),
        export_profile: "sRGB".to_string(),
        ..ThumbnailImageOptions::default()
    };

    ops::thumbnail_image_with_opts(image, target_w, &opts)
        .map_err(|_| OptimizerError::processing(format!(
            "Resize ({mode}) failed: {}",
            vips_error_buffer_string()
        )))
}

