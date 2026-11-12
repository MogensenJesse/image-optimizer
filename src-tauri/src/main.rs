// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Entry point of the Image Optimizer application.

mod commands;
mod error;
mod optimize;
mod types;

use tracing::{debug, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use tauri::Manager;
use crate::commands::optimize_images;
use crate::optimize::VipsRuntime;

// Import the window-vibrancy crate only on macOS
#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

fn main() {
    // An AppImage ships its own libheif plugins (the AV1 encoder and decoder
    // that AVIF needs); libheif only looks for them where this variable says.
    #[cfg(target_os = "linux")]
    {
        if let Some(appdir) = std::env::var_os("APPDIR") {
            let plugins = std::path::Path::new(&appdir).join("usr/lib/libheif/plugins");
            if plugins.is_dir() {
                // SAFETY: nothing else is running yet, this is the first thing main does
                unsafe { std::env::set_var("LIBHEIF_PLUGIN_PATH", plugins) };
            }
        }
    }

    // Respect RUST_LOG when set; otherwise info for the app and warn for HTTP stacks.
    // Global DEBUG was causing hyper/reqwest + tauri-plugin-updater to print full
    // latest.json (changelog text, signatures) on every update check.
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(
            "info,\
             image_optimizer=debug,\
             hyper=warn,\
             h2=warn,\
             reqwest=warn,\
             rustls=warn",
        )
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_file(false)
                .with_line_number(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_target(false)
                .with_ansi(true)
                .with_writer(std::io::stdout)
                .compact(),
        )
        .init();
    
    info!("=== Application Starting ===");

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .invoke_handler(tauri::generate_handler![optimize_images])
        .setup(|app| {
            // Initializes libvips and keeps it alive for the whole app lifetime
            app.manage(VipsRuntime::init());
            debug!("✓ libvips runtime initialized");

            // Register updater plugin (desktop only)
            #[cfg(desktop)]
            {
                app.handle()
                    .plugin(tauri_plugin_updater::Builder::new().build())
                    .expect("Failed to initialize updater plugin");
                debug!("✓ Updater plugin initialized");
            }
            
            #[cfg(target_os = "macos")]
            {
                let window = app.get_webview_window("main").unwrap();
                info!("Applying vibrancy effect for macOS");
                // Note: This requires macOSPrivateApi=true in tauri.conf.json
                apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None)
                    .expect("Failed to apply vibrancy effect on macOS");
            }
                
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    info!("Starting application event loop...");
    app.run(|_app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            info!("Application exiting");
        }
    });
}
