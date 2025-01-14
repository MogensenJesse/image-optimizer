use std::io;
use thiserror::Error;
use serde::Serialize;

#[derive(Error, Debug, Serialize)]
pub enum OptimizerError {
    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Processing error: {0}")]
    ProcessingError(String),

    #[error("Worker error: {0}")]
    WorkerError(String),

    #[error("IO error: {0}")]
    IOError(String),

    #[error("Format error: {0}")]
    FormatError(String),

    #[error("Sidecar error: {0}")]
    SidecarError(String),
}

// Common result type for the optimizer
pub type OptimizerResult<T> = Result<T, OptimizerError>;

// Helper methods for error creation
impl OptimizerError {
    pub fn validation<T: Into<String>>(msg: T) -> Self {
        Self::ValidationError(msg.into())
    }

    pub fn processing<T: Into<String>>(msg: T) -> Self {
        Self::ProcessingError(msg.into())
    }

    pub fn worker<T: Into<String>>(msg: T) -> Self {
        Self::WorkerError(msg.into())
    }

    pub fn format<T: Into<String>>(msg: T) -> Self {
        Self::FormatError(msg.into())
    }

    pub fn io<T: Into<String>>(msg: T) -> Self {
        Self::IOError(msg.into())
    }

    pub fn sidecar<T: Into<String>>(msg: T) -> Self {
        Self::SidecarError(msg.into())
    }
}

// Convert std::io::Error to OptimizerError
impl From<io::Error> for OptimizerError {
    fn from(err: io::Error) -> Self {
        Self::IOError(err.to_string())
    }
} 