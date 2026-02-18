// src-tauri/src/processing/libvips/mod.rs

//! Native image processing via libvips-rs.
//!
//! Replaces the Node.js Sharp sidecar with direct Rust-to-libvips calls,
//! eliminating subprocess spawn overhead, JSON serialisation, and the bundled
//! Node.js runtime from the application bundle.
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
