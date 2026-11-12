//! Image optimization: batch runner, output planning and the libvips runtime.
//!
//! Raster images are processed via libvips; SVG files are optimized with oxvg.
//! A batch is spread over a few worker threads, because the encoders (mozjpeg,
//! libpng, libwebp) are single-threaded and libvips' own thread pool only
//! speeds up pixel work.
//!
//! - [`raster`]: load, orient, colour-convert and resize
//! - [`encode`]: quality setting → format-specific save options
//! - [`svg`]: SVG minification

mod encode;
mod raster;
mod svg;

use std::collections::HashSet;
use std::ffi::CStr;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use tracing::{debug, warn};

use crate::error::{OptimizerError, OptimizerResult};
use crate::types::{BatchProgress, BatchSummary, FileError, ImageFormat, ImageSettings};

/// Upper bound on concurrent images: large AVIF/PNG jobs hold a full decoded
/// image in memory per worker.
const MAX_WORKERS: usize = 8;

/// Failed files reported back to the UI; any further failures are only counted.
const MAX_REPORTED_ERRORS: usize = 20;

// ── libvips runtime ───────────────────────────────────────────────────────────────────

/// Keeps libvips initialized for the whole app lifetime.
///
/// Register it with `App::manage`: libvips shuts down when it is dropped.
pub struct VipsRuntime(libvips::VipsApp);

// libvips is designed for concurrent use. Individual `VipsImage` values must
// not be shared between threads, but separate images can be created and
// processed on separate threads.
unsafe impl Send for VipsRuntime {}
unsafe impl Sync for VipsRuntime {}

impl VipsRuntime {
    pub fn init() -> Self {
        let vips = libvips::VipsApp::default("image-optimizer")
            .expect("Failed to initialize libvips");
        // 0 = let libvips decide based on available CPU cores
        // (each batch sets its own share, see `run_batch`)
        vips.concurrency_set(0);
        // Every file is opened once, so cached operations are never reused
        vips.cache_set_max(0);
        debug!("libvips initialized (concurrency: {})", vips.concurency_get());
        Self(vips)
    }
}

/// Reads and clears the global libvips error buffer for diagnostic messages.
///
/// Clearing keeps one failure's text from showing up in the next failure's message.
fn vips_error_buffer_string() -> String {
    unsafe {
        let ptr = libvips::bindings::vips_error_buffer();
        let message = if ptr.is_null() {
            "unknown error".to_string()
        } else {
            CStr::from_ptr(ptr).to_string_lossy().trim().to_string()
        };
        libvips::bindings::vips_error_clear();
        message
    }
}

fn extract_filename(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
}

// ── Batch runner ──────────────────────────────────────────────────────────────────────

/// Size result of one optimized file.
struct Outcome {
    original_bytes: u64,
    /// Negative when the output is larger than the input
    saved_bytes: i64,
}

impl Outcome {
    /// Computes the savings and logs them.
    fn new(file_name: &str, original_bytes: u64, optimized_bytes: u64) -> Self {
        let saved_bytes = original_bytes as i64 - optimized_bytes as i64;
        let percent = if original_bytes > 0 {
            saved_bytes as f64 / original_bytes as f64 * 100.0
        } else {
            0.0
        };
        debug!("'{file_name}' → {saved_bytes} bytes saved ({percent:.1}%)");
        Self { original_bytes, saved_bytes }
    }
}

/// Optimises every file in `paths` and returns the totals.
///
/// Blocking: call it from `spawn_blocking`. Each result lands in
/// `<source folder>/optimized/`. `on_progress` runs after every finished file,
/// under a lock, so the `completed` count it sees only ever goes up.
pub fn run_batch(
    paths: &[String],
    settings: &ImageSettings,
    on_progress: impl Fn(&BatchProgress) + Sync,
) -> BatchSummary {
    let start = Instant::now();
    let planned = plan_tasks(paths, settings);

    let output_dirs: HashSet<PathBuf> = planned
        .iter()
        .filter_map(|(_, output)| output.as_ref().ok())
        .filter_map(|output| Path::new(output).parent().map(Path::to_path_buf))
        .collect();
    for dir in &output_dirs {
        // A failure here resurfaces as a per-file save error with the real reason.
        if let Err(e) = std::fs::create_dir_all(dir) {
            warn!("Cannot create '{}': {e}", dir.display());
        }
    }

    let total = planned.len();
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    let workers = cores.min(total).clamp(1, MAX_WORKERS);
    // Give each worker an even share of the cores for libvips' own threads.
    // SAFETY: plain setter on libvips' global thread-pool size.
    unsafe { libvips::bindings::vips_concurrency_set((cores / workers).max(1) as i32) };
    debug!("Optimizing {total} files with {workers} workers");

    let next = AtomicUsize::new(0);
    let state = Mutex::new((
        BatchProgress { total, ..BatchProgress::default() },
        Vec::<FileError>::new(),
    ));

    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some((path, output)) = planned.get(index) else { break };

                    let outcome: Result<Outcome, String> = match output {
                        Ok(output_path) => catch_unwind(AssertUnwindSafe(|| {
                            optimize_single(path, output_path, settings)
                        }))
                        .unwrap_or_else(|_| {
                            Err(OptimizerError::processing("Crashed while processing"))
                        })
                        .map_err(|e| e.to_string()),
                        Err(e) => Err(e.to_string()),
                    };

                    let mut guard = state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    let (progress, errors) = &mut *guard;
                    progress.completed += 1;
                    match outcome {
                        Ok(outcome) => {
                            progress.saved_bytes += outcome.saved_bytes;
                            progress.original_bytes += outcome.original_bytes;
                        }
                        Err(error) => {
                            // The libvips error buffer is global, so with several workers a
                            // message can occasionally carry another file's error text.
                            warn!("Optimization failed for {path}: {error}");
                            progress.failed += 1;
                            if errors.len() < MAX_REPORTED_ERRORS {
                                errors.push(FileError {
                                    file_name: extract_filename(path).to_string(),
                                    error,
                                });
                            }
                        }
                    }
                    on_progress(progress);
                }
            });
        }
    });

    let (progress, errors) = state.into_inner().unwrap_or_else(|poisoned| poisoned.into_inner());
    BatchSummary {
        progress,
        duration_ms: start.elapsed().as_millis() as u64,
        errors,
    }
}

/// Optimises one file synchronously — dispatches to the SVG or raster pipeline.
fn optimize_single(input_path: &str, output_path: &str, settings: &ImageSettings) -> OptimizerResult<Outcome> {
    match ImageFormat::from_path(input_path)? {
        ImageFormat::Svg => svg::optimize(input_path, output_path, settings.quality),
        _ => raster::optimize(input_path, output_path, settings),
    }
}

// ── Output planning ───────────────────────────────────────────────────────────────────

/// One entry per input path: where its output goes, or why it cannot be processed.
type Planned = (String, OptimizerResult<String>);

/// Decides every output path of the batch up front, so workers never race on a name.
fn plan_tasks(paths: &[String], settings: &ImageSettings) -> Vec<Planned> {
    let mut used = HashSet::new();
    paths
        .iter()
        .map(|path| (path.clone(), plan_output(path, settings, &mut used)))
        .collect()
}

/// `<parent>/optimized/<name>` for one input.
///
/// SVGs, "original" and a conversion to the file's own format keep the file
/// name; any other conversion swaps the extension (`jpeg` → `jpg`). A name that
/// is already taken in the same folder (e.g. `a.jpg` and `a.png` both
/// converted to WebP) gets `-2`, `-3`, … appended.
fn plan_output(path: &str, settings: &ImageSettings, used: &mut HashSet<String>) -> OptimizerResult<String> {
    let format = ImageFormat::from_path(path)?;
    let input = Path::new(path);
    let dir = input.parent().unwrap_or(Path::new("")).join("optimized");
    let stem = input.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let input_ext = input.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();

    let ext = match settings.output_format.target() {
        Some(target) if format != ImageFormat::Svg && target != format => target.extension().to_string(),
        _ => input_ext,
    };

    for n in 1.. {
        let name = if n == 1 { format!("{stem}.{ext}") } else { format!("{stem}-{n}.{ext}") };
        let candidate = dir.join(name).to_string_lossy().to_string();
        // Windows and default macOS volumes are case-insensitive.
        if used.insert(candidate.to_lowercase()) {
            return Ok(candidate);
        }
    }
    unreachable!("the loop above only ends by returning")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OutputFormat, Resize, ResizeMode};
    use libvips::VipsImage;
    use libvips::ops;
    use std::sync::Mutex;

    fn settings_for(output_format: OutputFormat) -> ImageSettings {
        ImageSettings {
            quality: 80,
            resize: Resize { mode: ResizeMode::None, size: None },
            output_format,
        }
    }

    fn settings() -> ImageSettings {
        settings_for(OutputFormat::Original)
    }

    /// libvips may only be initialised once per process and tests run in parallel
    /// threads, so initialise it once and never shut it down.
    fn init_vips() {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| std::mem::forget(VipsRuntime::init()));
    }

    fn unique_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("image-optimizer-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Output file names (not full paths, so the tests hold on any OS separator).
    fn planned_names(paths: &[&str], output_format: OutputFormat) -> Vec<String> {
        let paths: Vec<String> = paths.iter().map(|p| p.to_string()).collect();
        plan_tasks(&paths, &settings_for(output_format))
            .into_iter()
            .map(|(_, output)| match output {
                Ok(output) => Path::new(&output).file_name().unwrap().to_string_lossy().to_string(),
                Err(e) => format!("ERR: {e}"),
            })
            .collect()
    }

    #[test]
    fn outputs_go_to_an_optimized_folder_next_to_the_source() {
        let paths = vec!["/photos/trip/a.jpg".to_string()];
        let (_, output) = plan_tasks(&paths, &settings()).remove(0);
        let out = PathBuf::from(output.unwrap());
        assert_eq!(out.parent().unwrap().file_name().unwrap(), "optimized");
        assert_eq!(out.parent().unwrap().parent().unwrap().file_name().unwrap(), "trip");
    }

    #[test]
    fn original_keeps_the_input_name_untouched() {
        assert_eq!(planned_names(&["/p/Photo.JPG", "/p/b.jpeg"], OutputFormat::Original), ["Photo.JPG", "b.jpeg"]);
    }

    #[test]
    fn conversion_swaps_the_extension() {
        assert_eq!(planned_names(&["/p/a.png", "/p/b.webp"], OutputFormat::Jpeg), ["a.jpg", "b.jpg"]);
        assert_eq!(planned_names(&["/p/a.jpg"], OutputFormat::Avif), ["a.avif"]);
    }

    #[test]
    fn converting_to_the_same_format_keeps_the_name() {
        assert_eq!(planned_names(&["/p/a.jpg", "/p/b.jpeg"], OutputFormat::Jpeg), ["a.jpg", "b.jpeg"]);
    }

    #[test]
    fn svg_ignores_conversion() {
        assert_eq!(planned_names(&["/p/logo.svg"], OutputFormat::Webp), ["logo.svg"]);
    }

    #[test]
    fn colliding_names_get_a_numeric_suffix() {
        assert_eq!(
            planned_names(&["/p/a.jpg", "/p/a.png", "/p/a.avif"], OutputFormat::Webp),
            ["a.webp", "a-2.webp", "a-3.webp"]
        );
    }

    #[test]
    fn collisions_ignore_case_but_not_folders() {
        assert_eq!(
            planned_names(&["/p/A.jpg", "/p/a.JPG", "/q/a.jpg"], OutputFormat::Original),
            ["A.jpg", "a-2.JPG", "a.jpg"]
        );
    }

    #[test]
    fn unsupported_files_fail_individually() {
        let names = planned_names(&["/p/a.jpg", "/p/notes.txt", "/p/noext"], OutputFormat::Original);
        assert_eq!(names[0], "a.jpg");
        assert!(names[1].starts_with("ERR:"), "{}", names[1]);
        assert!(names[2].starts_with("ERR:"), "{}", names[2]);
    }

    /// Regression test for the lost EXIF orientation: savers strip metadata, so
    /// the pixels themselves must be rotated.
    #[test]
    fn portrait_photo_without_resize_is_saved_upright() {
        init_vips();
        let dir = unique_dir("rotate");
        let input = dir.join("portrait.jpg");
        let output = dir.join("out.jpg");

        // Stored 40x20 with orientation 6 (rotate 90° CW), so it displays as 20x40.
        let img = ops::copy(&ops::black(40, 20).unwrap()).unwrap();
        img.set_int("orientation", 6).unwrap();
        ops::jpegsave(&img, input.to_str().unwrap()).unwrap();
        assert_eq!(
            VipsImage::new_from_file(input.to_str().unwrap()).unwrap().get_orientation(),
            6,
            "fixture must carry the orientation tag"
        );

        raster::optimize(input.to_str().unwrap(), output.to_str().unwrap(), &settings()).expect("optimize");

        let out = VipsImage::new_from_file(output.to_str().unwrap()).unwrap();
        assert_eq!((out.get_width(), out.get_height()), (20, 40));
        assert_eq!(out.get_orientation(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn batch_reports_each_failure_and_finishes_the_rest() {
        init_vips();
        let dir = unique_dir("batch");

        let mut paths = Vec::new();
        for i in 0..12 {
            let path = dir.join(format!("ok{i}.jpg"));
            ops::jpegsave(&ops::black(64, 48).unwrap(), path.to_str().unwrap()).unwrap();
            paths.push(path.to_string_lossy().to_string());
        }
        // Not a real JPEG: decoding must fail without disturbing the others.
        let corrupt = dir.join("corrupt.jpg");
        std::fs::write(&corrupt, b"definitely not a jpeg").unwrap();
        paths.push(corrupt.to_string_lossy().to_string());

        let seen = Mutex::new(Vec::new());
        let summary = run_batch(&paths, &settings(), |p| seen.lock().unwrap().push(p.completed));

        assert_eq!(summary.progress.total, 13);
        assert_eq!(summary.progress.completed, 13);
        assert_eq!(summary.progress.failed, 1);
        assert_eq!(summary.errors.len(), 1);
        assert_eq!(summary.errors[0].file_name, "corrupt.jpg");
        assert_eq!(*seen.lock().unwrap(), (1..=13).collect::<Vec<_>>(), "progress must be ordered");
        for i in 0..12 {
            assert!(dir.join("optimized").join(format!("ok{i}.jpg")).exists(), "ok{i}.jpg missing");
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Manual benchmark, never runs in CI:
    /// `BENCH_DIR=<folder of jpgs> cargo test bench_batch -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn bench_batch() {
        init_vips();
        let input_dir = std::env::var("BENCH_DIR").expect("set BENCH_DIR");
        let files: Vec<String> = std::fs::read_dir(&input_dir)
            .unwrap()
            .map(|e| e.unwrap().path().to_string_lossy().to_string())
            .collect();

        let summary = run_batch(&files, &settings(), |_| {});

        let optimized = Path::new(&input_dir).join("optimized");
        let bytes: u64 = std::fs::read_dir(&optimized)
            .unwrap()
            .map(|e| e.unwrap().metadata().unwrap().len())
            .sum();
        println!(
            "BENCH {} files in {:.2}s, {bytes} bytes out, {} failed",
            files.len(),
            summary.duration_ms as f64 / 1000.0,
            summary.progress.failed
        );
        std::fs::remove_dir_all(&optimized).ok();
    }
}
