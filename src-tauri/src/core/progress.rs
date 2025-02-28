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

    /// Set the task ID for this progress update
    /// 
    /// This builder method is part of the fluent API design for Progress objects
    /// and is maintained for API consistency and future extensibility.
    #[allow(dead_code)]
    pub fn with_task_id(mut self, task_id: &str) -> Self {
        self.task_id = Some(task_id.to_string());
        self
    }

    /// Set the worker ID for this progress update
    /// 
    /// This builder method is part of the fluent API design for Progress objects
    /// and is maintained for API consistency and future extensibility.
    #[allow(dead_code)]
    pub fn with_worker_id(mut self, worker_id: usize) -> Self {
        self.worker_id = Some(worker_id);
        self
    }

    /// Set the result for this progress update
    /// 
    /// This builder method is part of the fluent API design for Progress objects
    /// and is maintained for API consistency and future extensibility.
    #[allow(dead_code)]
    pub fn with_result(mut self, result: SharpResult) -> Self {
        self.result = Some(result);
        self
    }

    /// Set an error message for this progress update
    /// 
    /// This builder method is part of the fluent API design for Progress objects
    /// and is maintained for API consistency and future extensibility.
    #[allow(dead_code)]
    pub fn with_error(mut self, error: &str) -> Self {
        self.error = Some(error.to_string());
        self
    }

    /// Set metadata for this progress update
    /// 
    /// This builder method is part of the fluent API design for Progress objects
    /// and is maintained for API consistency and future extensibility.
    #[allow(dead_code)]
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Create from a metrics object
    /// 
    /// This factory method allows creating a Progress object from a metrics instance,
    /// which provides a convenient way to build progress messages with metrics data.
    #[allow(dead_code)]
    pub fn from_metrics(
        progress_type: ProgressType, 
        metrics: &ProgressMetrics,
        status: &str
    ) -> Self {
        let progress_percentage = if metrics.total_tasks > 0 {
            (metrics.completed_tasks * 100) / metrics.total_tasks
        } else {
            0
        };

        Self {
            progress_type,
            completed_tasks: metrics.completed_tasks,
            total_tasks: metrics.total_tasks,
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

/// Progress reporter trait for components that need to report progress
pub trait ProgressReporter {
    /// Send a progress update
    fn report_progress(&self, progress: &Progress);
    
    /// Report the start of a task
    /// 
    /// This is a convenience method that builds and sends a properly formatted
    /// start progress message. It's included as a default implementation to provide
    /// a consistent interface for progress reporting.
    #[allow(dead_code)]
    fn report_start(&self, task_id: &str, worker_id: Option<usize>, metrics: Option<&ProgressMetrics>) {
        let status = "starting";
        let mut progress = match metrics {
            Some(m) => Progress::from_metrics(ProgressType::Start, m, status),
            None => Progress::new(ProgressType::Start, 0, 1, status),
        };
        
        progress.task_id = Some(task_id.to_string());
        if let Some(id) = worker_id {
            progress.worker_id = Some(id);
        }
        
        self.report_progress(&progress);
    }
    
    /// Report progress of an ongoing task
    /// 
    /// This is a convenience method that builds and sends a properly formatted
    /// progress message. It's included as a default implementation to provide
    /// a consistent interface for progress reporting.
    #[allow(dead_code)]
    fn report_processing(&self, completed: usize, total: usize, task_id: Option<&str>, worker_id: Option<usize>) {
        let status = "processing";
        let mut progress = Progress::new(ProgressType::Progress, completed, total, status);
        
        if let Some(id) = task_id {
            progress.task_id = Some(id.to_string());
        }
        
        if let Some(id) = worker_id {
            progress.worker_id = Some(id);
        }
        
        self.report_progress(&progress);
    }
    
    /// Report the completion of a task
    /// 
    /// This is a convenience method that builds and sends a properly formatted
    /// completion message. It's included as a default implementation to provide
    /// a consistent interface for progress reporting.
    #[allow(dead_code)]
    fn report_complete(
        &self, 
        task_id: &str, 
        worker_id: Option<usize>, 
        result: Option<SharpResult>,
        metrics: Option<&ProgressMetrics>
    ) {
        let status = "complete";
        let mut progress = match metrics {
            Some(m) => Progress::from_metrics(ProgressType::Complete, m, status),
            None => Progress::new(ProgressType::Complete, 1, 1, status),
        };
        
        progress.task_id = Some(task_id.to_string());
        
        if let Some(id) = worker_id {
            progress.worker_id = Some(id);
        }
        
        if let Some(r) = result {
            progress.result = Some(r);
        }
        
        self.report_progress(&progress);
    }
    
    /// Report an error during task processing
    /// 
    /// This is a convenience method that builds and sends a properly formatted
    /// error message. It's included as a default implementation to provide
    /// a consistent interface for progress reporting.
    #[allow(dead_code)]
    fn report_error(&self, task_id: &str, worker_id: Option<usize>, error: &str) {
        let status = "error";
        let mut progress = Progress::new(ProgressType::Error, 0, 1, status);
        
        progress.task_id = Some(task_id.to_string());
        progress.error = Some(error.to_string());
        
        if let Some(id) = worker_id {
            progress.worker_id = Some(id);
        }
        
        self.report_progress(&progress);
    }
} 