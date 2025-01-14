// Module declarations in dependency order
pub mod utils;      // Base utilities, no internal dependencies
pub mod core;       // Core types and state
pub mod processing; // Processing logic, depends on core and utils
pub mod worker;     // Worker implementation, depends on processing and core
pub mod benchmarking; // Benchmarking functionality
mod commands;       // Command handlers, depends on all other modules

// Public exports
pub use core::{AppState, ImageSettings, OptimizationResult};
pub use worker::{WorkerPool, ImageTask};
pub use processing::ImageOptimizer;
pub use utils::{OptimizerError, OptimizerResult};
pub use benchmarking::{BenchmarkMetrics, BenchmarkReporter};
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
