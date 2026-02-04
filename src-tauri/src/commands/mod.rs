//! Tauri command handlers for the frontend.
//!
//! This module exposes commands that can be invoked from the React frontend:
//! - [`optimize_image`]: Optimize a single image
//! - [`optimize_images`]: Batch optimize multiple images

mod image;

pub use image::*;
