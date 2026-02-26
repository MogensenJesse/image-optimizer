//! Core application types and state management.
//!
//! This module contains the fundamental types used throughout the application:
//! - [`AppState`]: Application state managed by Tauri
//! - [`ImageTask`]: Represents an image optimization task
//! - [`ImageSettings`]: Configuration for image processing
//! - [`OptimizationResult`]: Result of an optimization operation
mod state;
mod types;
mod task;

pub use state::AppState;
pub use types::{ImageSettings, QualitySettings, ResizeSettings, OptimizationResult};
pub use task::ImageTask;