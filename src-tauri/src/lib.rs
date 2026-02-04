//! Image Optimizer Library
//!
//! A high-performance image optimization library built with Tauri.
//!
//! This crate provides the core functionality for the Image Optimizer desktop application,
//! including:
//!
//! - Image optimization via Sharp/libvips sidecar process
//! - Batch processing with progress tracking
//! - Support for JPEG, PNG, WebP, and AVIF formats
//! - Memory-mapped file transfer for efficient large batch processing
//!
//! # Architecture
//!
//! The library is organized into the following modules:
//!
//! - [`commands`]: Tauri command handlers for frontend invocation
//! - [`core`]: Application state, types, and task definitions
//! - [`processing`]: Sharp sidecar communication and execution
//! - [`utils`]: Error handling, validation, and format utilities
//!
//! # Example
//!
//! ```ignore
//! // From the frontend (JavaScript/TypeScript):
//! import { invoke } from '@tauri-apps/api/core';
//!
//! const result = await invoke('optimize_image', {
//!     inputPath: '/path/to/input.jpg',
//!     outputPath: '/path/to/output.jpg',
//!     settings: {
//!         quality: { global: 80 },
//!         resize: { mode: 'none', maintainAspect: true },
//!         outputFormat: 'original'
//!     }
//! });
//! ```

// Module declarations in dependency order
pub mod commands;
pub mod core;
pub mod processing;
pub mod utils;

// Public exports for external consumers
pub use core::{AppState, ImageTask, ImageSettings, OptimizationResult};
pub use utils::{OptimizerError, OptimizerResult};
pub use commands::*;
