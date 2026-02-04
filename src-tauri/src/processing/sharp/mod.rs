//! Sharp sidecar integration.
//!
//! This module handles communication with the Node.js Sharp sidecar process
//! that performs image optimization using libvips.
//!
//! The sidecar is spawned as a separate process and communicates via:
//! - Memory-mapped files for batch task data (avoids command-line length limits)
//! - stdout/stderr for progress messages and results

pub mod types;
mod progress_handler;
mod memory_map_executor;

pub use memory_map_executor::MemoryMapExecutor;
