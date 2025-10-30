use serde::{Deserialize, Serialize};
use crate::processing::SharpResult;

/// Progress message type
#[derive(Debug, Deserialize, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ProgressType {
    Start,
    Progress,
    Complete,
    Error,
}

/// Metrics included in progress messages
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressMetrics {
    /// Number of completed tasks
    pub completed_tasks: usize,
    /// Total number of tasks in the batch
    pub total_tasks: usize,
}

/// Unified progress struct for tracking progress throughout the application
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    /// Progress type (start, progress, complete, error)
    pub progress_type: ProgressType,
    /// Number of completed tasks
    pub completed_tasks: usize,
    /// Total number of tasks
    pub total_tasks: usize, 
    /// Progress percentage (0-100)
    pub progress_percentage: usize,
    /// Current status message
    pub status: String,
    /// Optional task ID for individual task progress
    #[serde(default)]
    pub task_id: Option<String>,
    /// Optional worker ID that processed the task
    #[serde(default)]
    pub worker_id: Option<usize>,
    /// Optional result for completed tasks
    #[serde(default)]
    pub result: Option<SharpResult>,
    /// Optional error message
    #[serde(default)]
    pub error: Option<String>,
    /// Optional additional metadata
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
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

impl Progress {
    /// Create a new Progress instance with basic information
    pub fn new(
        progress_type: ProgressType,
        completed_tasks: usize,
        total_tasks: usize,
        status: &str,
    ) -> Self {
        let progress_percentage = if total_tasks > 0 {
            (completed_tasks * 100) / total_tasks
        } else {
            0
        };

        Self {
            progress_type,
            completed_tasks,
            total_tasks,
            progress_percentage,
            status: status.to_string(),
            task_id: None,
            worker_id: None,
            result: None,
            error: None,
            metadata: None,
        }
    }

    /// Convert to a ProgressUpdate for frontend consumption
    pub fn to_progress_update(&self) -> ProgressUpdate {
        ProgressUpdate {
            completed_tasks: self.completed_tasks,
            total_tasks: self.total_tasks,
            progress_percentage: self.progress_percentage,
            status: self.status.clone(),
            metadata: self.metadata.clone(),
        }
    }
}
