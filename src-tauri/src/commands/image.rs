use serde::{Deserialize, Serialize};
use tauri_plugin_shell::ShellExt;

#[derive(Debug, Serialize, Deserialize)]
pub struct OptimizationResult {
    path: String,
    #[serde(rename = "originalSize")]
    original_size: u64,
    #[serde(rename = "optimizedSize")]
    optimized_size: u64,
    #[serde(rename = "savedBytes")]
    saved_bytes: i64,
    #[serde(rename = "compressionRatio")]
    compression_ratio: String,
    format: String,
}

#[tauri::command]
pub async fn optimize_image(app: tauri::AppHandle, input_path: String, output_path: String) -> Result<OptimizationResult, String> {
    let command = app
        .shell()
        .sidecar("sharp-sidecar")
        .expect("failed to create sharp sidecar command")
        .args(&["optimize", &input_path, &output_path]);

    let output = command
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        let result: OptimizationResult = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
            .map_err(|e| e.to_string())?;
        Ok(result)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}
