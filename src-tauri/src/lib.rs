// Module declarations in dependency order
#[cfg(feature = "benchmarking")]
pub mod benchmarking;
pub mod commands;
pub mod core;
pub mod processing;
pub mod utils;

// Public exports for external consumers
pub use core::{AppState, ImageTask, ImageSettings, OptimizationResult};
pub use utils::{OptimizerError, OptimizerResult};
#[cfg(feature = "benchmarking")]
pub use benchmarking::metrics::BenchmarkMetrics;
#[cfg(feature = "benchmarking")]
pub use benchmarking::reporter::BenchmarkReporter;
pub use commands::*;

// This library file is used as a public API for consuming this crate as a library.
// The actual application entry point is in main.rs.
