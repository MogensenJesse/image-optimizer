use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSettings {
    pub quality: QualitySettings,
    pub resize: ResizeSettings,
    #[serde(rename = "outputFormat")]
    pub output_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitySettings {
    pub global: u32,
    pub jpeg: Option<u32>,
    pub png: Option<u32>,
    pub webp: Option<u32>,
    pub avif: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResizeSettings {
    pub width: Option<u32>,
    pub height: Option<u32>,
    #[serde(rename = "maintainAspect")]
    pub maintain_aspect: bool,
    pub mode: String,
    pub size: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OptimizationResult {
    pub original_path: String,
    pub optimized_path: String,
    pub original_size: u64,
    pub optimized_size: u64,
    pub success: bool,
    pub error: Option<String>,
    #[serde(rename = "savedBytes")]
    pub saved_bytes: i64,
    #[serde(rename = "compressionRatio")]
    pub compression_ratio: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageTask {
    pub input_path: String,
    pub output_path: String,
    pub settings: ImageSettings,
} 