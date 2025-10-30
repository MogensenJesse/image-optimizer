pub mod error;
pub mod validation;
pub mod formats;

pub use error::{OptimizerError, OptimizerResult};
pub use validation::{validate_task, extract_filename};
pub use formats::format_from_extension; 