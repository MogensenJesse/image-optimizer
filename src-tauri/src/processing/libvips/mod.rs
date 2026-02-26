// src-tauri/src/processing/libvips/mod.rs

//! Native image processing via vendored libvips-rs bindings.
//!
//! # Architecture
//!
//! - [`NativeExecutor`]: Drives batch processing and emits Tauri progress events.
//! - [`resize`]: Maps `ResizeSettings` resize modes to `ops::thumbnail_image_with_opts`.
//! - [`formats`]: Maps `QualitySettings` to format-specific `ops::*save_with_opts` calls.

mod executor;
mod formats;
mod resize;

pub use executor::NativeExecutor;

use std::ffi::CStr;

/// Reads the global libvips error buffer for diagnostic messages.
pub(crate) fn vips_error_buffer_string() -> String {
    unsafe {
        let ptr = libvips::bindings::vips_error_buffer();
        if ptr.is_null() {
            return "unknown error".to_string();
        }
        CStr::from_ptr(ptr)
            .to_string_lossy()
            .trim()
            .to_string()
    }
}
