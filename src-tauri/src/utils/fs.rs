use std::path::Path;
use tokio::fs;
use crate::utils::{OptimizerError, OptimizerResult};

/// Get file size in bytes
#[allow(dead_code)]
pub async fn get_file_size(path: impl AsRef<Path>) -> OptimizerResult<u64> {
    fs::metadata(path.as_ref())
        .await
        .map(|m| m.len())
        .map_err(|e| OptimizerError::io(format!("Failed to get file size: {}", e)))
}

#[allow(dead_code)]
/// Check if file exists
pub async fn file_exists(path: impl AsRef<Path>) -> bool {
    Path::new(path.as_ref()).exists()
}

#[allow(dead_code)]
/// Check if directory exists
pub async fn dir_exists(path: impl AsRef<Path>) -> bool {
    let path = Path::new(path.as_ref());
    path.exists() && path.is_dir()
}

#[allow(dead_code)]
/// Get file extension as lowercase string
pub fn get_extension(path: impl AsRef<Path>) -> OptimizerResult<String> {
    path.as_ref()
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .ok_or_else(|| OptimizerError::validation(
            format!("File has no extension: {}", path.as_ref().display())
        ))
} 