use serde::Serialize;
use crate::core::ImageSettings;

#[derive(Debug, Clone, Serialize)]
pub struct ImageTask {
    pub input_path: String,
    pub output_path: String,
    pub settings: ImageSettings,
} 