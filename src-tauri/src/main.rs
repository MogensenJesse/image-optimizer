// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod utils;
mod core;
mod processing;
mod worker;
mod benchmarking;
mod commands;

use std::env;
use tracing::{info, debug};
use tauri::Manager;
use crate::core::AppState;
use crate::commands::{optimize_image, optimize_images, get_active_tasks};

fn main() {
    // Initialize logging with more verbose output in benchmark mode
    let benchmark_mode = env::var("BENCHMARK").is_ok();
    
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

    // Initialize worker pool with benchmarking if enabled
    if benchmark_mode {
        info!("Initializing worker pool with benchmarking...");
        let app_handle = app.app_handle().clone();
        let state = app.state::<AppState>();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let pool = state.get_or_init_worker_pool(app_handle).await
                .expect("Failed to initialize worker pool");
            
            // Enable benchmarking on the pool
            pool.enable_benchmarking().await;
            info!("✓ Worker pool initialized with benchmarking enabled");
            debug!("Worker pool configuration: {} workers", pool.get_worker_count());
        });
    }

    info!("Starting application event loop...");
    app.run(|_app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            info!("Application exiting");
        }
    });
}
