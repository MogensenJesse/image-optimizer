//! SVG optimization via oxvg, a high-performance Rust port of SVGO.

use super::{Outcome, extract_filename};
use crate::error::{OptimizerError, OptimizerResult};

/// Maps the global quality setting (1–100) to SVG float precision (0–8).
///
/// Higher quality preserves more decimal places in coordinates; lower quality
/// rounds more aggressively for smaller files at the cost of path fidelity.
fn quality_to_precision(quality: u8) -> u8 {
    match quality {
        95..=100 => 8,
        85..=94 => 3,
        70..=84 => 2,
        40..=69 => 1,
        _ => 0,
    }
}

/// Optimises an SVG file.
///
/// Parses the SVG into an AST, runs the default SVGO-equivalent jobs with
/// float precision derived from the quality slider, and writes the result.
pub fn optimize(input_path: &str, output_path: &str, quality: u8) -> OptimizerResult<Outcome> {
    use oxvg_ast::{parse::roxmltree::parse, serialize::Node as _, visitor::Info};
    use oxvg_optimiser::Jobs;

    let precision = quality_to_precision(quality);

    let svg_content = std::fs::read_to_string(input_path)
        .map_err(|e| OptimizerError::processing(format!("Cannot read SVG file: {e}")))?;

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

    std::fs::write(output_path, &optimized_svg)
        .map_err(|e| OptimizerError::processing(format!("Cannot write optimized SVG: {e}")))?;

    Ok(Outcome::new(
        extract_filename(input_path),
        svg_content.len() as u64,
        optimized_svg.len() as u64,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_maps_to_decreasing_precision() {
        assert_eq!(quality_to_precision(100), 8);
        assert_eq!(quality_to_precision(90), 3);
        assert_eq!(quality_to_precision(75), 2);
        assert_eq!(quality_to_precision(50), 1);
        assert_eq!(quality_to_precision(10), 0);
    }
}
