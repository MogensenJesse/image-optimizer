//! Error types for the image optimizer.
//!
//! Provides a hierarchy of error types using `thiserror` for ergonomic error handling.

use std::io;
use std::path::PathBuf;
use thiserror::Error;
use serde::Serialize;

/// Validation errors for input tasks and settings.
#[derive(Error, Debug, Serialize)]
pub enum ValidationError {
    /// Path-related validation error
    #[error("Path error: {0}")]
    Path(#[from] PathError),
    /// Invalid settings error
    #[error("Settings error: {0}")]
    Settings(String),
}

/// File path errors.
#[derive(Error, Debug, Serialize)]
pub enum PathError {
    /// File does not exist
    #[error("File not found: {0}")]
    NotFound(PathBuf),
    /// Path exists but is not a file
    #[error("Not a file: {0}")]
    NotFile(PathBuf),
    /// IO error accessing the path
    #[error("IO error: {0}")]
    IO(String),
}

/// Main error type for the optimizer application.
///
/// All errors in the application are converted to this type before being
/// returned to the frontend.
#[derive(Error, Debug, Serialize)]
pub enum OptimizerError {
    /// Task or input validation failed
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    /// Image processing failed
    #[error("Processing error: {0}")]
    Processing(String),

    /// File IO error
    #[error("IO error: {0}")]
    IO(String),

    /// Unsupported or invalid image format
    #[error("Format error: {0}")]
    Format(String),

    /// Sharp sidecar process error
    #[error("Sidecar error: {0}")]
    Sidecar(String),
}

/// Convenience result type for optimizer operations.
pub type OptimizerResult<T> = Result<T, OptimizerError>;

// Helper methods for error creation
impl OptimizerError {
    // Note: validation and io methods were removed as they were unused

    pub fn processing<T: Into<String>>(msg: T) -> Self {
        Self::Processing(msg.into())
    }

    pub fn format<T: Into<String>>(msg: T) -> Self {
        Self::Format(msg.into())
    }

    pub fn sidecar<T: Into<String>>(msg: T) -> Self {
        Self::Sidecar(msg.into())
    }
}

// Helper methods for validation error creation
impl ValidationError {
    pub fn path_not_found(path: impl Into<PathBuf>) -> Self {
        Self::Path(PathError::NotFound(path.into()))
    }

    pub fn not_a_file(path: impl Into<PathBuf>) -> Self {
        Self::Path(PathError::NotFile(path.into()))
    }

    pub fn settings(msg: impl Into<String>) -> Self {
        Self::Settings(msg.into())
    }
}

// Convert std::io::Error to OptimizerError
impl From<io::Error> for OptimizerError {
    fn from(err: io::Error) -> Self {
        Self::IO(err.to_string())
    }
}

// Convert io::Error to PathError
impl From<io::Error> for PathError {
    fn from(err: io::Error) -> Self {
        Self::IO(err.to_string())
    }
}

// Convert PathError to OptimizerError
impl From<PathError> for OptimizerError {
    fn from(err: PathError) -> Self {
        Self::Validation(ValidationError::Path(err))
    }
} 