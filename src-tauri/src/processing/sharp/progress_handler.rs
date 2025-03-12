use tauri::AppHandle;
use crate::core::{Progress, ProgressType};
use super::types::{ProgressMessage, ProgressUpdate, DetailedProgressUpdate, SharpResult};
use tracing::{debug, warn};
use tauri::Emitter;
use serde_json;
use std::path::Path;

/// Handles progress reporting and message processing from the Sharp sidecar
pub struct ProgressHandler {
    app: AppHandle,
}

impl ProgressHandler {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
    
    /// Extract filename from a path
    pub fn extract_filename<'b>(&self, path: &'b str) -> &'b str {
        Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path)
    }
    
    /// Handles a progress message from the sidecar
    pub fn handle_progress(&self, message: ProgressMessage) {
        // Convert from the processing-specific type to the core progress type
        let mut progress = message.to_core_progress();
        
        // Add metadata for optimization statistics if a result is available
        if let Some(result) = &progress.result {
            let file_name = self.extract_filename(&result.path);
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
        let progress_type = ProgressType::Complete;
        let completed_tasks = update.batch_metrics.completed_tasks;
        let total_tasks = update.batch_metrics.total_tasks;
        
        let mut progress = Progress::new(
            progress_type,
            completed_tasks,
            total_tasks,
            "complete"
        );
        
        // Set task ID
        progress.task_id = Some(update.task_id.clone());
        
        // Calculate saved bytes and retrieve other metrics
        let saved_bytes = update.optimization_metrics.saved_bytes;
        let compression_ratio = update.optimization_metrics.compression_ratio.clone();
        let file_name = self.extract_filename(&update.task_id);
        
        // Create a result object
        let result = SharpResult {
            path: update.task_id.clone(),
            original_size: update.optimization_metrics.original_size,
            optimized_size: update.optimization_metrics.optimized_size,
            saved_bytes: saved_bytes as i64,
            compression_ratio: compression_ratio.clone(),
            format: update.optimization_metrics.format.clone(),
            success: true,
            error: None,
        };
        
        progress.result = Some(result);
        
        // Create formatted message and metadata for the frontend
        let saved_kb = saved_bytes as f64 / 1024.0;
        let formatted_msg = format!(
            "{} optimized ({:.2} KB saved / {}% compression) - Progress: {}% ({}/{})",
            file_name,
            saved_kb,
            compression_ratio,
            update.batch_metrics.progress_percentage,
            update.batch_metrics.completed_tasks,
            update.batch_metrics.total_tasks
        );
        
        // Add detailed metadata for the frontend
        let metadata = serde_json::json!({
            "formattedMessage": formatted_msg,
            "fileName": file_name,
            "originalSize": update.optimization_metrics.original_size,
            "optimizedSize": update.optimization_metrics.optimized_size,
            "savedBytes": saved_bytes,
            "compressionRatio": compression_ratio
        });
        
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
        
        match progress.progress_type {
            ProgressType::Start => {
                // Emit event without logging
                let _ = self.app.emit("image_optimization_progress", progress_update);
            }
            ProgressType::Error => {
                warn!("Optimization error: {}", progress.status);
                let _ = self.app.emit("image_optimization_progress", progress_update);
            }
            _ => {
                // Emit event without logging progress status
                let _ = self.app.emit("image_optimization_progress", progress_update);
            }
        }
    }
} 