//! Application state management for Tauri.

use std::sync::Arc;
use crate::processing::libvips::NativeExecutor;
use tracing::debug;
use crate::utils::OptimizerError;

/// Thread-safe guard for the libvips `VipsApp` lifecycle.
///
/// `VipsApp` initializes the libvips thread pool and global state on creation
/// and shuts it down on drop. Wrapping in Arc ensures exactly one shutdown
/// call when the last reference is released.
///
/// # Safety
/// libvips is designed for concurrent multi-threaded use. Operations on
/// separate `VipsImage` instances from different threads are safe.
struct VipsAppGuard(libvips::VipsApp);

// libvips is designed for concurrent use; individual VipsImage instances must
// not be shared between threads, but concurrent creation on separate threads is safe.
unsafe impl Send for VipsAppGuard {}
unsafe impl Sync for VipsAppGuard {}

/// Application state managed by Tauri.
///
/// Holds the app handle and keeps the libvips runtime alive for the
/// entire application lifetime.
#[derive(Clone)]
pub struct AppState {
    app_handle: Arc<tauri::AppHandle>,
    /// Keeps libvips initialized until the last AppState clone is dropped.
    _vips: Arc<VipsAppGuard>,
}

impl AppState {
    /// Creates a new application state.
    ///
    /// Initializes libvips and stores the guard so it stays alive as long as
    /// any clone of this state exists.
    pub fn new(app: tauri::AppHandle) -> Self {
        let vips = libvips::VipsApp::default("image-optimizer")
            .expect("Failed to initialize libvips");
        // 0 = let libvips decide based on available CPU cores
        vips.concurrency_set(0);
        debug!("libvips initialized (concurrency: {})", vips.concurency_get());

        Self {
            app_handle: Arc::new(app),
            _vips: Arc::new(VipsAppGuard(vips)),
        }
    }

    /// Creates a new native libvips executor for batch processing.
    pub fn create_executor(&self) -> NativeExecutor {
        NativeExecutor::new((*self.app_handle).clone())
    }

    /// No-op warmup placeholder kept for API compatibility.
    ///
    /// The native executor has no cold-start overhead unlike the sidecar,
    /// so warmup is not needed.
    pub async fn warmup_executor(&self) -> Result<(), OptimizerError> {
        debug!("Native executor requires no warmup");
        Ok(())
    }
} 