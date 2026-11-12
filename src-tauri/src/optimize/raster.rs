//! Raster optimization: load (with shrink-on-load resize), orient, convert
//! colours, then hand over to the encoder.

use tracing::debug;

use libvips::ops::{self, Access};
use libvips::VipsImage;

use super::encode::save_image_as;
use super::{Outcome, extract_filename, vips_error_buffer_string};
use crate::error::{OptimizerError, OptimizerResult};
use crate::types::{ImageFormat, ImageSettings, Resize, ResizeMode};

/// libvips' maximum coordinate; used as "no limit" for the unconstrained side.
const NO_LIMIT: i32 = 10_000_000;

/// Optimises one raster image synchronously via libvips.
///
/// The output format is taken from the extension of `output_path`, which
/// `plan_output` already set to match the requested format.
pub fn optimize(input_path: &str, output_path: &str, settings: &ImageSettings) -> OptimizerResult<Outcome> {
    let original_size = std::fs::metadata(input_path)
        .map(|m| m.len())
        .map_err(|e| OptimizerError::processing(format!("Cannot read input file: {e}")))?;

    let output_format = ImageFormat::from_path(output_path)?;

    let image = if settings.resize.is_active() {
        let img = load_and_resize(input_path, &settings.resize)?;
        debug!(
            "Loaded+resized '{}': {}×{}",
            extract_filename(input_path),
            img.get_width(),
            img.get_height()
        );
        img
    } else {
        let img = VipsImage::new_from_file_access(input_path, Access::Sequential, false)
            .map_err(|_| OptimizerError::processing(format!(
                "Failed to load '{input_path}': {}",
                vips_error_buffer_string()
            )))?;
        debug!(
            "Loaded '{}': {}×{}",
            extract_filename(input_path),
            img.get_width(),
            img.get_height()
        );
        auto_rotate(img)?
    };

    let image = to_srgb_if_profiled(image)?;
    save_image_as(&image, output_path, output_format, settings.quality)?;

    let optimized_size = std::fs::metadata(output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(Outcome::new(extract_filename(input_path), original_size, optimized_size))
}

// ── Resize ────────────────────────────────────────────────────────────────────────────

/// Loads and resizes an image from `path` in one step.
///
/// Uses the file-based `vips_thumbnail` which enables **shrink-on-load**:
/// for JPEG, libjpeg can skip decoding most DCT coefficients when
/// downsizing by integer factors (2x, 4x, 8x), making large-image
/// resizes significantly faster than loading first and resizing second.
/// `thumbnail` also applies the EXIF orientation, so the bounding box is
/// fitted against the *displayed* (rotated) image.
fn load_and_resize(path: &str, resize: &Resize) -> OptimizerResult<VipsImage> {
    let size = resize.size.unwrap_or(0) as i32;
    if size <= 0 {
        return Err(OptimizerError::processing("No target size for resize"));
    }

    // Only "shortest" depends on the image's own dimensions; the other modes
    // need no probe, so those files are opened once instead of twice.
    let (disp_w, disp_h) = if resize.mode == ResizeMode::Shortest {
        let probe = VipsImage::new_from_file(path)
            .map_err(|_| OptimizerError::processing(format!(
                "Failed to probe '{}': {}", path, vips_error_buffer_string()
            )))?;
        displayed_dims(probe.get_width(), probe.get_height(), probe.orientation_swaps_dims())
    } else {
        (0, 0)
    };

    let (target_w, target_h) = resize_box(resize.mode, size, disp_w, disp_h);
    thumbnail_file(path, target_w, target_h, resize.mode)
}

/// Width and height as displayed, i.e. after applying the EXIF orientation.
fn displayed_dims(width: i32, height: i32, swaps_dims: bool) -> (i32, i32) {
    if swaps_dims { (height, width) } else { (width, height) }
}

/// Bounding box `(width, height)` to hand to `thumbnail` for a resize mode.
///
/// With `Size::Down` the image is scaled to fit the box and never enlarged, so
/// the unconstrained side gets [`NO_LIMIT`]. `disp_w`/`disp_h` are only read by
/// "shortest". `ResizeMode::None` never gets here (`Resize::is_active` gates
/// it); it maps to a box that changes nothing.
fn resize_box(mode: ResizeMode, size: i32, disp_w: i32, disp_h: i32) -> (i32, i32) {
    match mode {
        ResizeMode::None => (NO_LIMIT, NO_LIMIT),
        ResizeMode::Width => (size, NO_LIMIT),
        ResizeMode::Height => (NO_LIMIT, size),
        ResizeMode::Longest => (size, size),
        ResizeMode::Shortest if disp_w <= disp_h => (size, NO_LIMIT),
        ResizeMode::Shortest => (NO_LIMIT, size),
    }
}

/// File-based thumbnail: loads and resizes in one step, enabling
/// shrink-on-load for formats that support it (JPEG, WebP, TIFF).
fn thumbnail_file(path: &str, target_w: i32, target_h: i32, mode: ResizeMode) -> OptimizerResult<VipsImage> {
    use ops::{Size, ThumbnailOptions};

    let opts = ThumbnailOptions {
        height: target_h,
        size: Size::Down,
        import_profile: "sRGB".to_string(),
        ..ThumbnailOptions::default()
    };

    ops::thumbnail_with_opts(path, target_w, &opts)
        .map_err(|_| OptimizerError::processing(format!(
            "Resize ({mode:?}) failed: {}",
            vips_error_buffer_string()
        )))
}

// ── Orientation and colour ────────────────────────────────────────────────────────────

fn vips_err(context: &str) -> OptimizerError {
    OptimizerError::processing(format!("{context}: {}", vips_error_buffer_string()))
}

/// Applies the EXIF orientation so the saved file is upright.
///
/// Savers strip metadata (including the orientation tag), so without this a
/// portrait phone photo would come out sideways. The resize path needs no
/// call: `thumbnail` rotates by itself.
fn auto_rotate(img: VipsImage) -> OptimizerResult<VipsImage> {
    if img.get_orientation() <= 1 {
        return Ok(img);
    }
    // autorot needs random access; the image was opened for sequential reads.
    let in_memory = VipsImage::image_copy_memory(img)
        .map_err(|_| vips_err("Failed to load image for rotation"))?;
    ops::autorot(&in_memory).map_err(|_| vips_err("Auto-rotate failed"))
}

/// Converts an image with an embedded ICC profile to sRGB.
///
/// Savers strip the profile, so wide-gamut photos (Display-P3, AdobeRGB) would
/// otherwise look washed out. Images without a profile, and thumbnails (which
/// already come out as profile-free sRGB), pass through untouched.
fn to_srgb_if_profiled(img: VipsImage) -> OptimizerResult<VipsImage> {
    if !img.has_icc_profile() {
        return Ok(img);
    }
    let opts = ops::IccTransformOptions {
        embedded: true,
        // The wrapper always passes input-profile, and an empty string would be
        // loaded as a file name. Built-in "sRGB" is the fallback for a bad profile.
        input_profile: "sRGB".to_string(),
        ..ops::IccTransformOptions::default()
    };
    ops::icc_transform_with_opts(&img, "sRGB", &opts)
        .map_err(|_| vips_err("Colour profile conversion failed"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_and_height_constrain_one_side() {
        assert_eq!(resize_box(ResizeMode::Width, 800, 0, 0), (800, NO_LIMIT));
        assert_eq!(resize_box(ResizeMode::Height, 800, 0, 0), (NO_LIMIT, 800));
    }

    #[test]
    fn longest_fits_a_square_box() {
        assert_eq!(resize_box(ResizeMode::Longest, 800, 0, 0), (800, 800));
    }

    #[test]
    fn shortest_constrains_the_shorter_displayed_side() {
        // landscape: height is shorter
        assert_eq!(resize_box(ResizeMode::Shortest, 800, 4000, 3000), (NO_LIMIT, 800));
        // portrait: width is shorter
        assert_eq!(resize_box(ResizeMode::Shortest, 800, 3000, 4000), (800, NO_LIMIT));
        // square: either side works, width is picked
        assert_eq!(resize_box(ResizeMode::Shortest, 800, 3000, 3000), (800, NO_LIMIT));
    }

    #[test]
    fn no_resize_mode_changes_nothing() {
        assert_eq!(resize_box(ResizeMode::None, 800, 0, 0), (NO_LIMIT, NO_LIMIT));
    }

    #[test]
    fn rotated_photo_is_measured_as_displayed() {
        // 4000x3000 stored with orientation 6 displays as 3000x4000 (portrait)
        let (w, h) = displayed_dims(4000, 3000, true);
        assert_eq!((w, h), (3000, 4000));
        assert_eq!(resize_box(ResizeMode::Shortest, 800, w, h), (800, NO_LIMIT));
        assert_eq!(displayed_dims(4000, 3000, false), (4000, 3000));
    }
}
