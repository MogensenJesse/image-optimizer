//! Error type for the image optimizer.

use serde::Serialize;
use thiserror::Error;

/// All errors in the application are converted to this type before being
/// returned to the frontend.
#[derive(Error, Debug)]
pub enum OptimizerError {
    /// Settings or input failed validation
    #[error("Validation error: {0}")]
    Validation(String),

    /// Image processing failed
    #[error("Processing error: {0}")]
    Processing(String),

    /// Unsupported or invalid image format
    #[error("Format error: {0}")]
    Format(String),
}

/// Convenience result type for optimizer operations.
pub type OptimizerResult<T> = Result<T, OptimizerError>;

impl OptimizerError {
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    pub fn processing(msg: impl Into<String>) -> Self {
        Self::Processing(msg.into())
    }

    pub fn format(msg: impl Into<String>) -> Self {
        Self::Format(msg.into())
    }
}

/// Tauri sends a failed command's error to the frontend as JSON. The UI shows
/// it as text, so serialize the readable message instead of the enum structure.
impl Serialize for OptimizerError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
