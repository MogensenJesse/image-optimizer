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

#[derive(Debug, Serialize, Deserialize)]
pub struct ResizeSettings {
    width: Option<u32>,
    height: Option<u32>,
    #[serde(rename = "maintainAspect")]
    maintain_aspect: bool,
    mode: String,
    size: Option<u32>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QualitySettings {
    global: u32,
    jpeg: Option<u32>,
    png: Option<u32>,
    webp: Option<u32>,
    avif: Option<u32>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageSettings {
    quality: QualitySettings,
    resize: ResizeSettings,
    #[serde(rename = "outputFormat")]
    output_format: String
}

#[tauri::command]
pub async fn optimize_image(
    app: tauri::AppHandle, 
    input_path: String, 
    output_path: String,
    settings: ImageSettings
) -> Result<OptimizationResult, String> {
    println!("Settings received in Rust: {:?}", settings);

    // Serialize settings to JSON string
    let settings_json = serde_json::to_string(&settings)
        .map_err(|e| e.to_string())?;

    let command = app
        .shell()
        .sidecar("sharp-sidecar")
        .expect("failed to create sharp sidecar command")
        .args(&[
            "optimize",
            &input_path,
            &output_path,
            &settings_json
        ]);

    println!("Sending settings to sidecar: {}", settings_json);  // Debug log

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
