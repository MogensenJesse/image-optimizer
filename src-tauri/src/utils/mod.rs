pub mod error;
pub mod validation;
pub mod formats;
pub mod fs;

pub use error::{OptimizerError, OptimizerResult};
pub use validation::validate_task;
pub use formats::{ImageFormat, format_from_extension, get_conversion_quality};
pub use fs::{
    get_file_size,
    create_dir_all,
    ensure_parent_dir,
    validate_input_path,
    validate_output_path,
}; 