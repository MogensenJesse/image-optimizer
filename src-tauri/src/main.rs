// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod utils;
mod core;
mod processing;
#[cfg(feature = "benchmarking")]
mod benchmarking;
mod commands;

use tracing::info;
#[cfg(feature = "benchmarking")]
use tracing::debug;
#[cfg(feature = "benchmarking")]
use tauri::Manager;
use crate::core::AppState;
use crate::commands::{optimize_image, optimize_images, get_active_tasks};

fn main() {
    // Initialize logging with more verbose output in benchmark mode
    #[cfg(feature = "benchmarking")]
    let benchmark_mode = true;
    
    #[cfg(not(feature = "benchmarking"))]
    let benchmark_mode = false;
    
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(if benchmark_mode {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .with_file(false)         // Remove file path
        .with_line_number(false)  // Remove line numbers
        .with_thread_ids(false)   // Remove thread IDs
        .with_thread_names(false) // Remove thread names
        .with_target(false)       // Remove module path
        .with_ansi(true)         // Keep colored output
        .with_writer(std::io::stdout)
        .compact();              // Use compact formatter instead of pretty

    subscriber.init();
    
    info!("=== Application Starting ===");
    if benchmark_mode {
        info!("Benchmark mode: ENABLED");
        info!("Debug logging: ENABLED");
    } else {
        info!("Benchmark mode: DISABLED");
        info!("Debug logging: DISABLED");
    }

    let app = tauri::Builder::default()
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
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // Initialize process pool with benchmarking if enabled
    #[cfg(feature = "benchmarking")]
    {
        info!("Initializing process pool with benchmarking...");
        let app_handle = app.app_handle().clone();
        let state = app.state::<AppState>();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let pool = state.get_or_init_process_pool(app_handle).await
                .expect("Failed to initialize process pool");
            
            // Enable benchmarking on the pool
            pool.set_benchmark_mode(true).await;
            info!("✓ Process pool initialized with benchmarking enabled");
            debug!("Process pool configuration: {} processes", pool.get_max_size());
        });
    }

    info!("Starting application event loop...");
    app.run(|_app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            info!("Application exiting");
        }
    });
}
