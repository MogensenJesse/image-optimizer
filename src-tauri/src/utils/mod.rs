//! Utility modules for error handling, validation, and format detection.
//!
//! This module provides:
//! - [`OptimizerError`]: Unified error type for the application
//! - [`validate_task`]: Task validation before processing
//! - [`format_from_extension`]: Image format detection from file extensions

pub mod error;
pub mod validation;
pub mod formats;

pub use error::{OptimizerError, OptimizerResult};
pub use validation::{validate_task, extract_filename};
pub use formats::format_from_extension; 