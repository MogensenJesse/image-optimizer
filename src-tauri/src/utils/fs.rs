use std::path::Path;
use tokio::fs;
use crate::utils::{OptimizerError, OptimizerResult};

/// Get file size in bytes
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

/// Create directory and all parent directories if they don't exist
pub async fn create_dir_all(path: impl AsRef<Path>) -> OptimizerResult<()> {
    fs::create_dir_all(path.as_ref())
        .await
        .map_err(|e| OptimizerError::io(format!("Failed to create directory: {}", e)))
}

/// Ensure parent directory exists, creating it if necessary
pub async fn ensure_parent_dir(path: impl AsRef<Path>) -> OptimizerResult<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            create_dir_all(parent).await?;
        }
    }
    Ok(())
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

/// Validate input file path
pub async fn validate_input_path(path: impl AsRef<Path>) -> OptimizerResult<()> {
    let path = path.as_ref();
    
    if !path.exists() {
        return Err(OptimizerError::validation(
            format!("Input file does not exist: {}", path.display())
        ));
    }

    if !path.is_file() {
        return Err(OptimizerError::validation(
            format!("Input path is not a file: {}", path.display())
        ));
    }

    Ok(())
}

/// Validate output file path
pub async fn validate_output_path(path: impl AsRef<Path>) -> OptimizerResult<()> {
    let path = path.as_ref();
    
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            return Err(OptimizerError::validation(
                format!("Output directory does not exist: {}", parent.display())
            ));
        }
    }

    Ok(())
} 