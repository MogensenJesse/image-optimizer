// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod commands;
mod worker_pool;

pub use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            optimize_image,
            optimize_images,
            get_active_tasks,
            get_worker_metrics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
