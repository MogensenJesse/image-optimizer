// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// This is the primary entry point for the Image Optimizer application.
// The lib.rs file serves only as a public API for external consumers.

mod utils;
mod core;
mod processing;
#[cfg(feature = "benchmarking")]
mod benchmarking;
mod commands;

use tracing::{info, debug};
use tauri::Manager;
use crate::core::AppState;
use crate::commands::{optimize_image, optimize_images, get_active_tasks};

// Import the window-vibrancy crate only on macOS
#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

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
        .setup(|_app| {
            #[cfg(target_os = "macos")]
            {
                let window = _app.get_webview_window("main").unwrap();
                info!("Applying vibrancy effect for macOS");
                // Note: This requires macOSPrivateApi=true in tauri.conf.json
                apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None)
                    .expect("Failed to apply vibrancy effect on macOS");
            }
                
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // Initialize app handle in AppState
    let app_handle = app.app_handle().clone();
    let state = app.state::<AppState>();
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        state.set_app_handle(app_handle).await;
        debug!("✓ AppState initialized");
    });

    // Start warmup in a separate task so it doesn't block app startup
    let app_handle = app.app_handle().clone();
    tauri::async_runtime::spawn(async move {
        // Add a small delay to ensure the app is fully initialized
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        let state = app_handle.state::<AppState>();
        if let Err(e) = state.warmup_executor().await {
            debug!("Executor warmup failed: {}", e);
        } else {
            debug!("Executor warmup completed in the background");
        }
    });

    info!("Starting application event loop...");
    app.run(|_app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            info!("Application exiting");
        }
    });
}
