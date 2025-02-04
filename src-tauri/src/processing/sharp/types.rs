use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SharpResult {
    pub path: String,
    pub optimized_size: u64,
    pub original_size: u64,
    pub saved_bytes: i64,
    pub compression_ratio: String,
    #[allow(dead_code)]
    pub format: Option<String>,
    pub success: bool,
    pub error: Option<String>,
} 