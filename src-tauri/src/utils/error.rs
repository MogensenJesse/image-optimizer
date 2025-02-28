use std::io;
use std::path::PathBuf;
use thiserror::Error;
use serde::Serialize;

#[derive(Error, Debug, Serialize)]
pub enum ValidationError {
    #[error("Path error: {0}")]
    Path(#[from] PathError),
    #[error("Settings error: {0}")]
    Settings(String),
}

#[derive(Error, Debug, Serialize)]
pub enum PathError {
    #[error("File not found: {0}")]
    NotFound(PathBuf),
    #[error("Not a file: {0}")]
    NotFile(PathBuf),
    #[error("IO error: {0}")]
    IO(String),
}

#[derive(Error, Debug, Serialize)]
pub enum OptimizerError {
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Processing error: {0}")]
    Processing(String),

    #[error("IO error: {0}")]
    IO(String),

    #[error("Format error: {0}")]
    Format(String),

    #[error("Sidecar error: {0}")]
    Sidecar(String),
}

// Common result type for the optimizer
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