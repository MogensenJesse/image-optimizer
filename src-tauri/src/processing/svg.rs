// src-tauri/src/processing/svg.rs

//! SVG optimization via oxvg (high-performance Rust port of SVGO).

use std::path::Path;
use tracing::debug;

use crate::core::{ImageTask, OptimizationResult};
use crate::utils::{OptimizerError, OptimizerResult, extract_filename};

/// Maps the global quality setting (1-100) to SVG float precision (0-8).
///
/// Higher quality preserves more decimal places in coordinates; lower quality
/// rounds more aggressively for smaller files at the cost of path fidelity.
fn quality_to_precision(quality: u32) -> u8 {
    match quality {
        95..=100 => 8,
        85..=94 => 3,
        70..=84 => 2,
        40..=69 => 1,
        _ => 0,
    }
}

/// Optimises an SVG file using oxvg (high-performance Rust port of SVGO).
///
/// Parses the SVG into an AST, runs the default SVGO-equivalent jobs with
/// float precision derived from the quality slider, and writes the result.
pub fn optimize_svg(task: &ImageTask) -> OptimizerResult<OptimizationResult> {
    use oxvg_ast::{parse::roxmltree::parse, serialize::Node as _, visitor::Info};
    use oxvg_optimiser::Jobs;

    let input_path = &task.input_path;
    let precision = quality_to_precision(task.settings.quality.global);

    let svg_content = std::fs::read_to_string(input_path)
        .map_err(|e| OptimizerError::processing(format!("Cannot read SVG file: {e}")))?;

    let original_size = svg_content.len() as u64;

    let precision_overrides: Jobs = serde_json::from_value(serde_json::json!({
        "cleanupNumericValues": { "floatPrecision": precision },
        "convertPathData": { "floatPrecision": precision },
        "cleanupListOfValues": { "floatPrecision": precision },
    }))
    .map_err(|e| OptimizerError::processing(format!("SVG jobs config failed: {e}")))?;

    let mut jobs = Jobs::default();
    jobs.extend(&precision_overrides);

    let optimized_svg = parse(&svg_content, |dom, allocator| {
        jobs.run(dom, &Info::new(allocator))
            .map_err(|e| e.to_string())?;
        dom.serialize().map_err(|e| e.to_string())
    })
    .map_err(|e| OptimizerError::processing(format!("SVG parsing failed: {e}")))?
    .map_err(|e| OptimizerError::processing(format!("SVG optimization failed: {e}")))?;

    let output_path = &task.output_path;
    if let Some(parent) = Path::new(output_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            OptimizerError::processing(format!("Cannot create output directory: {e}"))
        })?;
    }

    std::fs::write(output_path, &optimized_svg)
        .map_err(|e| OptimizerError::processing(format!("Cannot write optimized SVG: {e}")))?;

    let optimized_size = optimized_svg.len() as u64;
    let saved_bytes = original_size as i64 - optimized_size as i64;
    let compression_ratio = if original_size > 0 {
        saved_bytes as f64 / original_size as f64 * 100.0
    } else {
        0.0
    };

    debug!(
        "'{}' → {} bytes saved ({:.1}%)",
        extract_filename(input_path),
        saved_bytes,
        compression_ratio
    );

    Ok(OptimizationResult {
        original_path: input_path.clone(),
        optimized_path: output_path.clone(),
        original_size,
        optimized_size,
        success: true,
        error: None,
        saved_bytes,
        compression_ratio,
    })
}
