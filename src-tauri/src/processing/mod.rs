//! Image processing via the Sharp sidecar.
//!
//! This module handles communication with the Node.js Sharp sidecar process
//! that performs the actual image optimization using libvips.

pub mod sharp;

pub use sharp::types::SharpResult; 