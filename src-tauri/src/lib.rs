// Module declarations in dependency order
#[cfg(feature = "benchmarking")]
pub mod benchmarking;
pub mod commands;
pub mod core;
pub mod processing;
pub mod utils;

// Public exports
pub use core::{AppState, ImageTask};
pub use utils::{OptimizerError, OptimizerResult};
#[cfg(feature = "benchmarking")]
pub use benchmarking::metrics::BenchmarkMetrics;
#[cfg(feature = "benchmarking")]
pub use benchmarking::reporter::BenchmarkReporter;
pub use commands::*;

use tauri::Manager;
use window_vibrancy::{apply_vibrancy, apply_acrylic, NSVisualEffectMaterial};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            optimize_image,
            optimize_images,
            get_active_tasks,
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            
            #[cfg(target_os = "macos")]
            apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None)
                .expect("Failed to apply vibrancy effect on macOS");
            
            #[cfg(target_os = "windows")]
            apply_acrylic(&window, Some((18, 18, 18, 125)))
                .expect("Failed to apply acrylic effect on Windows");
                
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
