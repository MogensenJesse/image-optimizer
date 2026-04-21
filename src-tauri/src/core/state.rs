//! Application state management for Tauri.

use std::sync::Arc;
use crate::processing::libvips::NativeExecutor;
use tracing::debug;

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

/// Always process at least 2 images concurrently for I/O overlap.
const BATCH_CONCURRENCY: usize = 2;

/// Threads reserved for the OS, Tauri webview, and UI responsiveness.
const RESERVED_THREADS: usize = 2;

/// Application state managed by Tauri.
///
/// Holds the app handle and keeps the libvips runtime alive for the
/// entire application lifetime.
#[derive(Clone)]
pub struct AppState {
    app_handle: Arc<tauri::AppHandle>,
    /// Keeps libvips initialized until the last AppState clone is dropped.
    _vips: Arc<VipsAppGuard>,
    /// How many images to process in parallel (computed once at startup).
    batch_concurrency: usize,
}

impl AppState {
    /// Creates a new application state.
    ///
    /// Initializes libvips with a tuned thread pool and a fixed batch
    /// concurrency of 2 images in parallel.
    ///
    /// The per-image thread count is set so that
    /// `vips_threads * BATCH_CONCURRENCY + RESERVED_THREADS ≈ cpu_threads`,
    /// keeping the machine responsive during heavy encodes (e.g. AVIF).
    pub fn new(app: tauri::AppHandle) -> Self {
        let vips = libvips::VipsApp::default("image-optimizer")
            .expect("Failed to initialize libvips");

        let cpu_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        // Split available threads across concurrent images, reserving some
        // for the OS and webview so the desktop stays responsive.
        let vips_threads = ((cpu_threads.saturating_sub(RESERVED_THREADS)) / BATCH_CONCURRENCY).max(2);
        vips.concurrency_set(vips_threads as i32);

        debug!(
            "libvips initialized (per-image threads: {}, batch concurrency: {}, logical CPUs: {})",
            vips_threads,
            BATCH_CONCURRENCY,
            cpu_threads,
        );

        Self {
            app_handle: Arc::new(app),
            _vips: Arc::new(VipsAppGuard(vips)),
            batch_concurrency: BATCH_CONCURRENCY,
        }
    }

    /// Creates a new native libvips executor for batch processing.
    pub fn create_executor(&self) -> NativeExecutor {
        NativeExecutor::new((*self.app_handle).clone(), self.batch_concurrency)
    }
} 