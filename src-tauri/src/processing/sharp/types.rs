use serde::Deserialize;
use serde::Serialize;
use crate::core::{Progress as CoreProgress, ProgressType as CoreProgressType};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct SharpResult {
    pub path: String,
    pub optimized_size: u64,
    pub original_size: u64,
    pub saved_bytes: i64,
    pub compression_ratio: String,
    /// The format of the output image (e.g., "jpeg", "png")
    /// 
    /// This field is set by the JavaScript sidecar but not currently accessed in Rust.
    /// It must be preserved for proper deserialization of SharpResult objects from
    /// the sidecar's JSON responses.
    pub format: Option<String>,
    pub success: bool,
    pub error: Option<String>,
}

/// Progress message type from the sidecar
#[derive(Debug, Deserialize, Clone, Serialize)]
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
    /// Metrics from the JavaScript sidecar worker
    /// 
    /// This field is required for proper deserialization of messages from the sidecar.
    /// The worker-pool.js sends metrics data in each progress message that includes:
    /// - completedTasks
    /// - totalTasks
    ///
    /// While these fields aren't directly used in Rust, they must remain for
    /// proper JSON deserialization from the sidecar.
    #[serde(default)]
    pub metrics: Option<ProgressMetrics>,
}

/// Metrics included in progress messages from the JavaScript sidecar
/// These field names match the JavaScript camelCase naming convention
/// and must be preserved for proper deserialization. See the sendProgressUpdate 
/// and message handlers in the worker-pool.js file.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProgressMetrics {
    /// Number of completed tasks, set by the sidecar
    pub completed_tasks: usize,
    /// Total number of tasks in the batch, set by the sidecar
    pub total_tasks: usize,
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

// Define type aliases to help with the transition to the centralized system
// This alias is maintained for potential backward compatibility with code that
// might reference this type directly in the future.
#[allow(dead_code)]
pub type Progress = CoreProgress;

// Add conversion implementations between progress types
impl From<ProgressType> for CoreProgressType {
    fn from(progress_type: ProgressType) -> Self {
        match progress_type {
            ProgressType::Start => CoreProgressType::Start,
            ProgressType::Progress => CoreProgressType::Progress,
            ProgressType::Complete => CoreProgressType::Complete,
            ProgressType::Error => CoreProgressType::Error,
        }
    }
}

impl From<CoreProgressType> for ProgressType {
    fn from(progress_type: CoreProgressType) -> Self {
        match progress_type {
            CoreProgressType::Start => ProgressType::Start,
            CoreProgressType::Progress => ProgressType::Progress,
            CoreProgressType::Complete => ProgressType::Complete,
            CoreProgressType::Error => ProgressType::Error,
        }
    }
}

// Add additional conversion helpers for the transition
impl ProgressMessage {
    /// Convert to the centralized Progress type
    pub fn to_core_progress(&self) -> CoreProgress {
        let (completed_tasks, total_tasks) = if let Some(metrics) = &self.metrics {
            (
                metrics.completed_tasks,
                metrics.total_tasks
            )
        } else {
            (0, 0)
        };

        let status = match self.progress_type {
            ProgressType::Start => "starting",
            ProgressType::Progress => "processing",
            ProgressType::Complete => "complete",
            ProgressType::Error => "error",
        }.to_string();

        let mut progress = CoreProgress::new(
            self.progress_type.clone().into(),
            completed_tasks,
            total_tasks,
            &status
        );
        
        progress.task_id = Some(self.task_id.clone());
        progress.worker_id = Some(self.worker_id);
        progress.result = self.result.clone();
        progress.error = self.error.clone();
        
        progress
    }
}

impl ProgressUpdate {
    /// Convert to the centralized Progress type
    pub fn to_core_progress(&self) -> CoreProgress {
        let progress_type = match self.status.as_str() {
            "complete" => CoreProgressType::Complete,
            "error" => CoreProgressType::Error,
            _ => CoreProgressType::Progress,
        };

        let mut progress = CoreProgress::new(
            progress_type,
            self.completed_tasks,
            self.total_tasks,
            &self.status
        );
        
        if let Some(metadata) = &self.metadata {
            progress.metadata = Some(metadata.clone());
        }
        
        progress
    }
} 