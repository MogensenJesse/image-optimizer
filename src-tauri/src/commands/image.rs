use tauri_plugin_shell::ShellExt;

#[tauri::command]
pub async fn optimize_image(app: tauri::AppHandle, input_path: String, output_path: String) -> Result<String, String> {
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
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}
