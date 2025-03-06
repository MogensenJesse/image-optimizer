# Progress Tracking: Backend Support for Detailed Optimization Metrics

## Overview

This document tracks the implementation of backend support for the detailed optimization metrics produced by the Sharp sidecar. The backend has been updated to properly parse, process, and forward the new detailed progress messages to the frontend.

## Changes Made

### 1. Updated Data Structures

- Created new structs in `src-tauri/src/processing/sharp/types.rs`:
  - Updated `DetailedProgressUpdate` to match the new sidecar message format
  - Added `OptimizationMetrics` struct to hold file-specific optimization data
  - Added `BatchMetrics` struct to hold batch processing progress metrics

```rust
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
```

### 2. Enhanced Message Handling

- Updated the `handle_detailed_progress_update` method in `src-tauri/src/processing/sharp/executor.rs`:
  - Added conversion from the new format to the core Progress struct
  - Added support for formatted messages
  - Included detailed optimization metrics in the progress reports

```rust
/// Handles a detailed progress update with file-specific optimization metrics
fn handle_detailed_progress_update(&self, update: DetailedProgressUpdate) {
    // Create a progress object with the file-specific optimization data
    let mut progress = Progress::new(
        ProgressType::Progress,
        update.batch_metrics.completed_tasks,
        update.batch_metrics.total_tasks,
        "processing"
    );
    
    // ... convert data and add formatted messages ...
    
    // Report progress using the trait
    self.report_progress(&progress);
}
```

### 3. Improved Message Detection

- Updated the message detection logic in the sidecar output processing:
  - Added detection for the new "detailed_progress" message type
  - Added better error handling for unparseable messages

```rust
if line_str.contains("\"progressType\"") || line_str.contains("\"status\"") || 
   line_str.contains("\"type\":\"progress_detail\"") || line_str.contains("\"type\":\"detailed_progress\"") {
    // Try to parse as different message types...
}
```

### 4. Enhanced Terminal Logging

- Updated the `report_progress` method in `SharpExecutor` to display detailed optimization metrics in the terminal logs:
  - Added detection for formatted messages in the metadata
  - Uses the pre-formatted message for a more informative display
  - Maintains consistent log levels (INFO/DEBUG) based on progress percentage

```rust
// Check if we have detailed optimization metrics in the metadata
let has_detailed_metrics = progress.metadata.as_ref()
    .and_then(|m| m.get("formattedMessage"))
    .is_some();

if has_detailed_metrics {
    // Extract and log the formatted message with detailed metrics
    if let Some(formatted_msg) = progress.metadata.as_ref()
        .and_then(|m| m.get("formattedMessage"))
        .and_then(|m| m.as_str()) 
    {
        // Use INFO level for significant progress points
        if progress.progress_percentage % 10 == 0 || 
           progress.progress_percentage == 25 || 
           progress.progress_percentage == 50 || 
           progress.progress_percentage == 75 ||
           progress.progress_percentage >= 100 {
            info!("📊 {}", formatted_msg);
        } else {
            debug!("📊 {}", formatted_msg);
        }
    }
}
```

## Testing

To test these changes, run a batch optimization process and observe the output. The backend should correctly:

1. Parse the new detailed progress messages from the sidecar
2. Convert them to the Progress struct for internal handling
3. Forward the information to the frontend for display
4. Display detailed optimization metrics in the terminal logs

Example terminal output with enhanced logging:
```
2025-03-06T16:49:34.151281Z  INFO Received optimize_images command for 11 images
2025-03-06T16:49:34.754432Z DEBUG 🔄 Worker 8 started processing IMG20231010130037.jpg
2025-03-06T16:49:35.582304Z DEBUG 📊 IMG20231010130037.jpg optimized (2.3 MB saved / 76.5% compression) - Progress: 9% (1/11)
2025-03-06T16:49:35.779683Z DEBUG 📊 IMG20231010130045.jpg optimized (1.8 MB saved / 82.1% compression) - Progress: 18% (2/11)
```

## Next Steps

1. Update the frontend to display the detailed optimization metrics
2. Add UI components to visualize compression statistics
3. Consider adding summary metrics to show total space saved across all images 