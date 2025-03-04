# Image Optimization Statistics Implementation

## Progress Summary

Legend:
🔄 = In Progress
⚠️ = Blocked/Has Issues
✅ = Completed

### Current Status:
🔄 Planning implementation of real-time optimization statistics display

### Next Implementation Steps:
1. Modify Rust backend to include statistics in progress events
2. Update frontend to extract and display statistics


## Implementation Plan

### 1. Add Statistics Data to Progress Events (Backend)

[ ] Enhance `ProgressUpdate` struct to include optimization statistics
   Short description: Add fields for original size, optimized size, saved size, and compression ratio to the ProgressUpdate struct
   Prerequisites: None
   Files to modify: `src-tauri/src/core/progress.rs`
   External dependencies: None
   Code to add/change/remove/move:
   ```rust
   // Update the ProgressUpdate struct to include statistics
   #[derive(Debug, Deserialize, Clone, Serialize)]
   #[serde(rename_all = "camelCase")]
   pub struct ProgressUpdate {
       pub completed_tasks: usize,
       pub total_tasks: usize,
       pub progress_percentage: usize,
       pub status: String,
       // Add fields for optimization statistics
       #[serde(default)]
       pub original_size: u64,
       #[serde(default)]
       pub optimized_size: u64,
       #[serde(default)]
       pub saved_bytes: i64,
       #[serde(default)]
       pub compression_ratio: f64,
       #[serde(default)]
       pub metadata: Option<serde_json::Value>,
   }
   ```

[ ] Update the `to_progress_update` method to include statistics
   Short description: Modify the implementation of to_progress_update to include statistics from results
   Prerequisites: Enhanced ProgressUpdate struct
   Files to modify: `src-tauri/src/core/progress.rs`
   External dependencies: None
   Code to add/change/remove/move:
   ```rust
   // Update the to_progress_update method in the Progress impl
   pub fn to_progress_update(&self) -> ProgressUpdate {
       // Extract statistics from result if available
       let (original_size, optimized_size, saved_bytes, compression_ratio) = if let Some(result) = &self.result {
           (
               result.original_size,
               result.optimized_size,
               result.saved_bytes,
               result.compression_ratio.parse::<f64>().unwrap_or(0.0),
           )
       } else {
           (0, 0, 0, 0.0)
       };

       ProgressUpdate {
           completed_tasks: self.completed_tasks,
           total_tasks: self.total_tasks,
           progress_percentage: self.progress_percentage,
           status: self.status.clone(),
           original_size,
           optimized_size,
           saved_bytes,
           compression_ratio,
           metadata: self.metadata.clone(),
       }
   }
   ```

[ ] Add Batch Statistics Tracking to Pool Processor
   Short description: Track cumulative statistics across the entire batch processing operation
   Prerequisites: None
   Files to modify: `src-tauri/src/processing/batch/processor.rs`
   External dependencies: None
   Code to add/change/remove/move:
   ```rust
   // Add statistics tracking to BatchProgress struct
   #[derive(Debug, Clone)]
   pub struct BatchProgress {
       pub total_files: usize,
       pub processed_files: usize,
       pub current_chunk: usize,
       pub total_chunks: usize,
       pub failed_tasks: Vec<(String, String)>, // (file_path, error_message)
       // Add statistics tracking
       pub total_original_size: u64,
       pub total_optimized_size: u64,
       pub total_saved_bytes: i64,
       pub compression_ratio: f64,
   }

   // Initialize the new fields in the process_batch method
   // Inside the BatchProcessor impl:
   let mut progress = BatchProgress {
       total_files: tasks.len(),
       processed_files: 0,
       current_chunk: 0,
       total_chunks: chunks.len(),
       failed_tasks: Vec::new(),
       total_original_size: 0,
       total_optimized_size: 0,
       total_saved_bytes: 0,
       compression_ratio: 0.0,
   };

   // Update the statistics after each chunk is processed
   for result in chunk_results {
       progress.total_original_size += result.original_size;
       progress.total_optimized_size += result.optimized_size;
       progress.total_saved_bytes += result.saved_bytes;
   }

   // Calculate the overall compression ratio
   if progress.total_original_size > 0 {
       progress.compression_ratio = (progress.total_saved_bytes as f64 / progress.total_original_size as f64) * 100.0;
   }
   ```

[ ] Add Progress Updates with Statistics in Benchmarking and Production Mode
   Short description: Ensure statistics are included in progress events in both modes
   Prerequisites: Previous steps completed
   Files to modify: `src-tauri/src/processing/batch/processor.rs`
   External dependencies: None
   Code to add/change/remove/move:
   ```rust
   // Inside process_batch method, ensure progress events include statistics
   // Update the progress event emission to include the statistics
   if let Some(app) = state.app.clone() {
       // Create a comprehensive progress update
       let update = Progress::new(
           ProgressType::Progress,
           progress.processed_files,
           progress.total_files,
           "processing"
       )
       .with_metadata(serde_json::json!({
           "originalSize": progress.total_original_size,
           "optimizedSize": progress.total_optimized_size,
           "savedBytes": progress.total_saved_bytes,
           "compressionRatio": progress.compression_ratio,
       }))
       .to_progress_update();
       
       let _ = app.emit("image_optimization_progress", update);
   }
   ```

### 2. Update Frontend to Display Statistics (Frontend)

[ ] Update useProgressTracker Hook to Extract Statistics
   Short description: Modify the hook to extract and process statistics from the progress events
   Prerequisites: Backend changes completed
   Files to modify: `src/hooks/useProgressTracker.js`
   External dependencies: None
   Code to add/change/remove/move:
   ```javascript
   // Update the event listener in useProgressTracker.js
   const unsubscribeProgress = listen('image_optimization_progress', (event) => {
     // Update cumulative progress
     const { 
       completedTasks, 
       totalTasks, 
       status, 
       originalSize, 
       optimizedSize, 
       savedBytes, 
       compressionRatio 
     } = event.payload;
     
     // Use processingRef instead of isProcessing to avoid state timing issues
     if (processingRef.current) {
       const currentBatch = batchProgressRef.current;
       
       // Update statistics if available
       if (savedBytes) {
         currentBatch.totalOriginalSize = originalSize || currentBatch.totalOriginalSize;
         currentBatch.totalOptimizedSize = optimizedSize || currentBatch.totalOptimizedSize;
         currentBatch.totalSavedBytes = savedBytes || currentBatch.totalSavedBytes;
         currentBatch.compressionRatio = compressionRatio || currentBatch.compressionRatio;
       }
       
       // Rest of the existing event handling code
       // ...
       
       // Calculate saved size in MB and saved percentage
       const savedSizeMB = currentBatch.totalSavedBytes / (1024 * 1024);
       const savedPercentage = compressionRatio || 
         (currentBatch.totalOriginalSize > 0 
           ? Math.round((currentBatch.totalSavedBytes / currentBatch.totalOriginalSize) * 100)
           : 0);
       
       // Update the progress state with cumulative values and statistics
       setProgress({
         completedTasks: currentBatch.processedImages,
         totalTasks: totalForCalculation,
         progressPercentage: overallPercentage,
         status: status,
         lastUpdated: Date.now(),
         savedSize: parseFloat(savedSizeMB.toFixed(1)),
         savedPercentage: savedPercentage
       });
     }
   });
   ```

[ ] Initialize Progress State with Statistics Fields
   Short description: Update the initial state to include statistics fields
   Prerequisites: None
   Files to modify: `src/hooks/useProgressTracker.js`
   External dependencies: None
   Code to add/change/remove/move:
   ```javascript
   // Update the initial progress state
   const [progress, setProgress] = useState({
     completedTasks: 0,
     totalTasks: 0,
     progressPercentage: 0,
     status: 'idle',
     lastUpdated: Date.now(),
     savedSize: 0,
     savedPercentage: 0
   });
   
   // Update the batch progress ref to include statistics
   const batchProgressRef = useRef({
     totalImages: 0,
     processedImages: 0,
     currentBatchId: null,
     lastCompletedInBatch: 0,
     lastStatus: null,
     batchCount: 0,
     knownTotalImages: null,
     totalOriginalSize: 0,
     totalOptimizedSize: 0,
     totalSavedBytes: 0,
     compressionRatio: 0
   });
   
   // Update initProgress to reset statistics
   const initProgress = (fileCount) => {
     setProgress({
       completedTasks: 0,
       totalTasks: fileCount,
       progressPercentage: 0,
       status: 'idle',
       lastUpdated: Date.now(),
       savedSize: 0,
       savedPercentage: 0
     });
     
     batchProgressRef.current = {
       totalImages: 0,
       processedImages: 0,
       currentBatchId: null,
       lastCompletedInBatch: 0,
       lastStatus: null,
       batchCount: 0,
       knownTotalImages: fileCount,
       totalOriginalSize: 0,
       totalOptimizedSize: 0,
       totalSavedBytes: 0,
       compressionRatio: 0
     };
   };
   ```

[ ] Ensure Statistics are Displayed in ProgressBar Component
   Short description: Verify that the ProgressBar component correctly displays the statistics
   Prerequisites: Statistics available in progress state
   Files to modify: No changes needed if the ProgressBar already accepts savedSize and savedPercentage props
   External dependencies: None
   Code to add/change/remove/move: None needed if the component already displays these values

## Implementation Notes
- The solution keeps statistics tracking simple by using the existing events and data structures
- Both benchmarking and production modes will include statistics in progress events
- The approach gracefully handles cases where statistics might not be available yet
- The frontend implementation aggregates statistics as they arrive, ensuring accurate representation
- The solution avoids creating new events or commands, maintaining the existing application architecture

## Completed Tasks

None yet

## Findings

### Known Issues:
- The current implementation uses estimated statistics instead of actual values
- Statistics aren't currently being passed from the backend to the frontend

### Technical Insights:
- The Rust backend already calculates accurate statistics (compression ratio, original/optimized sizes)
- The `OptimizationResult` struct already contains all the needed statistics
- The Tauri event system can efficiently transmit these statistics to the frontend
- Integrating real statistics will provide users with accurate feedback on optimization performance