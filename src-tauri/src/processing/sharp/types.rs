use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Clone)]
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

/// Progress message type from the sidecar
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum ProgressType {
    Start,
    Progress,
    Complete,
    Error,
}

/// Progress message from the sidecar
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProgressMessage {
    pub progress_type: ProgressType,
    pub task_id: String,
    pub worker_id: usize,
    #[serde(default)]
    pub result: Option<SharpResult>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub metrics: Option<ProgressMetrics>,
}

/// Metrics included in progress messages
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProgressMetrics {
    pub completed_tasks: usize,
    pub total_tasks: usize,
    pub queue_length: usize,
}

/// Simplified progress update for frontend progress bar
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressUpdate {
    pub completed_tasks: usize,
    pub total_tasks: usize,
    pub progress_percentage: usize,
    pub status: String,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
} 