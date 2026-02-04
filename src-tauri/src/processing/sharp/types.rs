//! Types for Sharp sidecar communication.
//!
//! These types mirror the JSON structures used by the Node.js Sharp sidecar
//! for serialization/deserialization of messages.

use serde::Deserialize;
use serde::Serialize;
use crate::core::{Progress as CoreProgress, ProgressType as CoreProgressType};

/// Result of a single image optimization from the Sharp sidecar.
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
#[derive(Debug, Deserialize, Clone, Serialize)]
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

/// Detailed progress update with file-specific optimization metrics
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailedProgressUpdate {
    /// File name without path
    pub file_name: String,
    /// Full task identifier (usually the input file path)
    pub task_id: String,
    /// Detailed optimization metrics for this specific file
    pub optimization_metrics: OptimizationMetrics,
    /// Batch progress metrics
    pub batch_metrics: BatchMetrics,
    /// Optional formatted message for direct display
    #[serde(default)]
    pub formatted_message: Option<String>,
}

/// Detailed optimization metrics for a specific file
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizationMetrics {
    /// Original file size in bytes
    pub original_size: u64,
    /// Optimized file size in bytes
    pub optimized_size: u64,
    /// Bytes saved during optimization
    pub saved_bytes: u64,
    /// Compression ratio as a string percentage
    pub compression_ratio: String,
    /// The format of the output image (e.g., "jpeg", "png")
    #[serde(default)]
    pub format: Option<String>,
}

/// Batch progress metrics
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchMetrics {
    /// Number of completed tasks 
    pub completed_tasks: usize,
    /// Total number of tasks
    pub total_tasks: usize,
    /// Progress percentage (0-100)
    pub progress_percentage: usize,
}

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
        
        // Clone values once and reuse where possible
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