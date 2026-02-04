use tauri::AppHandle;
use crate::core::{Progress, ProgressType};
use crate::utils::extract_filename;
use super::types::{ProgressMessage, ProgressUpdate, DetailedProgressUpdate, SharpResult};
use tracing::{debug, warn};
use tauri::Emitter;
use serde_json;

/// Handles progress reporting and message processing from the Sharp sidecar
pub struct ProgressHandler {
    app: AppHandle,
}

impl ProgressHandler {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
    
    /// Handles a progress message from the sidecar
    pub fn handle_progress(&self, message: ProgressMessage) {
        // Convert from the processing-specific type to the core progress type
        let mut progress = message.to_core_progress();
        
        // Add metadata for optimization statistics if a result is available
        if let Some(result) = &progress.result {
            let file_name = extract_filename(&result.path);
            let saved_kb = result.saved_bytes as f64 / 1024.0;
            
            let formatted_msg = format!(
                "{} optimized ({:.2} KB saved / {}% compression)",
                file_name,
                saved_kb,
                result.compression_ratio
            );
            
            let metadata = serde_json::json!({
                "formattedMessage": formatted_msg,
                "fileName": file_name,
                "originalSize": result.original_size,
                "optimizedSize": result.optimized_size,
                "savedBytes": result.saved_bytes,
                "compressionRatio": result.compression_ratio
            });
            
            progress.metadata = Some(metadata);
        }
        
        // Report progress
        self.report_progress(&progress);
    }
    
    /// Handles a simplified progress update from the sidecar
    pub fn handle_progress_update(&self, update: ProgressUpdate) {
        // Convert to core progress type
        let progress = update.to_core_progress();
        
        // Simplified updates already have metadata from the Sharp sidecar
        // Just pass them through to the frontend
        
        // Report progress
        self.report_progress(&progress);
    }
    
    /// Handles a detailed progress update from the sidecar
    pub fn handle_detailed_progress_update(&self, update: DetailedProgressUpdate) {
        // Create a progress object from the detailed update
        let completed_tasks = update.batch_metrics.completed_tasks;
        let total_tasks = update.batch_metrics.total_tasks;
        
        let mut progress = Progress::new(
            ProgressType::Complete,
            completed_tasks,
            total_tasks,
            "complete"
        );
        
        // Extract metrics - use references where possible to avoid redundant clones
        let saved_bytes = update.optimization_metrics.saved_bytes;
        let compression_ratio = &update.optimization_metrics.compression_ratio;
        let file_name = extract_filename(&update.task_id);
        
        // Create formatted message first (before moving values)
        let saved_kb = saved_bytes as f64 / 1024.0;
        let formatted_msg = format!(
            "{} optimized ({:.2} KB saved / {}% compression) - Progress: {}% ({}/{})",
            file_name,
            saved_kb,
            compression_ratio,
            update.batch_metrics.progress_percentage,
            completed_tasks,
            total_tasks
        );
        
        // Build metadata using references where json! macro allows
        let metadata = serde_json::json!({
            "formattedMessage": formatted_msg,
            "fileName": file_name,
            "originalSize": update.optimization_metrics.original_size,
            "optimizedSize": update.optimization_metrics.optimized_size,
            "savedBytes": saved_bytes,
            "compressionRatio": compression_ratio
        });
        
        // Create result object - clone only when storing into owned fields
        let result = SharpResult {
            path: update.task_id.clone(),
            original_size: update.optimization_metrics.original_size,
            optimized_size: update.optimization_metrics.optimized_size,
            saved_bytes: saved_bytes as i64,
            compression_ratio: update.optimization_metrics.compression_ratio.clone(),
            format: update.optimization_metrics.format.clone(),
            success: true,
            error: None,
        };
        
        // Set progress fields - task_id clone needed for owned Option<String>
        progress.task_id = Some(update.task_id);
        progress.result = Some(result);
        progress.metadata = Some(metadata);
        
        // Report progress
        self.report_progress(&progress);
    }
    
    /// Reports progress to the frontend via Tauri events
    pub fn report_progress(&self, progress: &Progress) {
        // Create the progress update for the frontend
        let progress_update = progress.to_progress_update();
        
        // Log formatted message if available in metadata
        if let Some(metadata) = &progress.metadata {
            // Log formatted message if available
            if let Some(msg) = metadata.get("formattedMessage") {
                if let Some(msg_str) = msg.as_str() {
                    debug!("{}", msg_str);
                }
            }
        }
        
        // Log error if needed
        if matches!(progress.progress_type, ProgressType::Error) {
            warn!("Optimization error: {}", progress.status);
        }
        
        // Emit event (same for all types)
        let _ = self.app.emit("image_optimization_progress", progress_update);
    }
} 