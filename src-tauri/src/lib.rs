// Module declarations in dependency order
pub mod benchmarking;
pub mod commands;
pub mod core;
pub mod processing;
pub mod utils;

// Public exports
pub use core::{AppState, ImageTask};
pub use utils::{OptimizerError, OptimizerResult};
pub use benchmarking::metrics::BenchmarkMetrics;
pub use benchmarking::reporter::BenchmarkReporter;
pub use commands::*;

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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
