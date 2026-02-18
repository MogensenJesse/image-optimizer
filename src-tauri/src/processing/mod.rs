//! Image processing modules.
//!
//! - [`libvips`]: Native Rust-to-libvips executor (primary).
//! - [`sharp`]: Legacy Node.js sidecar integration (kept during transition only).

pub mod libvips;
