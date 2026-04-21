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
pub use validation::validate_task;
pub use formats::{
    ImageFormat, format_from_extension,
    extract_filename, resolve_output_format, ensure_correct_extension,
};