// Module declarations
mod commands;
pub mod core;
pub mod worker;
pub mod processing;

// Public exports
pub use commands::*;
pub use core::{AppState, ImageSettings, OptimizationResult};
pub use worker::{WorkerPool, ImageTask};
pub use processing::{ImageOptimizer, ImageValidator, ValidationResult};

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
