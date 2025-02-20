# Progress Tracking Optimization

## Progress Summary

Legend:
🔄 = In Progress
⚠️ = Blocked/Has Issues
✅ = Completed

### Current Status:
Current progress tracking updates per chunk (75 files), leading to jumpy progress bar updates.

### Next Implementation Steps:
1. Implement mini-chunk processing in batch processor
2. Add time-based update throttling
3. Update progress event system
4. Test and tune parameters

## Implementation Plan

### 1. Mini-Chunk Processing Implementation

[ ] Add mini-chunk configuration and processing
   Short description: Modify batch processor to work with smaller chunks while maintaining parallel processing benefits
   Prerequisites: None
   Files to modify: src-tauri/src/processing/batch/processor.rs
   External dependencies: None
   Code to add:
   ```rust
   pub struct BatchProcessorConfig {
       pub chunk_size: usize,
       pub mini_chunk_size: usize,  // New: size for progress updates (e.g., 10)
       pub min_update_interval: Duration,  // New: minimum time between updates
   }

   impl Default for BatchProcessorConfig {
       fn default() -> Self {
           Self {
               chunk_size: 75,
               mini_chunk_size: 10,
               min_update_interval: Duration::from_millis(100),
           }
       }
   }

   impl BatchProcessor {
       fn process_chunk(&self, chunk: Vec<Task>, chunk_index: usize) -> Result<()> {
           let mut last_update = Instant::now();
           let mut processed_in_chunk = 0;
           
           // Process mini-chunks
           for mini_chunk in chunk.chunks(self.config.mini_chunk_size) {
               // Process files in mini-chunk
               for task in mini_chunk {
                   self.process_file(task)?;
                   processed_in_chunk += 1;
               }
               
               // Check if enough time has passed since last update
               if last_update.elapsed() >= self.config.min_update_interval {
                   self.emit_progress(chunk_index, processed_in_chunk)?;
                   last_update = Instant::now();
               }
           }
           
           // Final update for chunk
           self.emit_progress(chunk_index, processed_in_chunk)?;
           Ok(())
       }
   }
   ```
   [ ] Cleanup after moving code:
    - Update BatchProcessor struct with new config
    - Update constructor and builder pattern
    - Update tests to handle mini-chunks

[ ] Update progress event emission
   Short description: Enhance progress tracking to handle mini-chunk updates
   Prerequisites: Mini-chunk processing implementation
   Files to modify: src-tauri/src/processing/batch/processor.rs
   External dependencies: None
   Code to add:
   ```rust
   impl BatchProcessor {
       fn emit_progress(&self, chunk_index: usize, processed_in_chunk: usize) -> Result<()> {
           let total_processed = self.processed_files.load(Ordering::Relaxed) + processed_in_chunk;
           
           let progress = BatchProgress {
               total_files: self.total_files,
               processed_files: total_processed,
               current_chunk: chunk_index + 1,
               total_chunks: (self.total_files + self.config.chunk_size - 1) / self.config.chunk_size,
               current_mini_chunk: processed_in_chunk / self.config.mini_chunk_size + 1,
               total_mini_chunks: (self.config.chunk_size + self.config.mini_chunk_size - 1) / self.config.mini_chunk_size,
               failed_tasks: self.failed_tasks.lock().unwrap().clone(),
           };

           self.app_handle.emit_to("main", "optimization_progress", progress)
               .map_err(|e| Error::EventEmission(e.to_string()))
       }
   }
   ```
   [ ] Cleanup after moving code:
    - Update BatchProgress struct
    - Update error handling
    - Update tests for progress emission

### 2. Progress Event System Updates

[ ] Update BatchProgress struct
   Short description: Enhance progress struct to include mini-chunk information
   Prerequisites: None
   Files to modify: src-tauri/src/processing/batch/types.rs
   External dependencies: None
   Code to add:
   ```rust
   #[derive(Debug, Clone, Serialize)]
   pub struct BatchProgress {
       pub total_files: usize,
       pub processed_files: usize,
       pub current_chunk: usize,
       pub total_chunks: usize,
       pub current_mini_chunk: usize,
       pub total_mini_chunks: usize,
       pub failed_tasks: Vec<(String, String)>,
   }
   ```

## Implementation Notes
- Mini-chunk size of 10 provides good balance between granularity and performance
- 100ms minimum update interval prevents event spam while maintaining smooth updates
- Keep parallel processing benefits by only updating progress, not changing processing logic
- Progress updates are still accurate, just more frequent
- Consider memory usage with more frequent updates

## Completed Tasks

### Example of completed task (✅)
- Original chunk-based progress tracking
- Frontend progress bar implementation
- Event system integration

## Findings

### Known Issues:
- Progress updates too infrequent with 75-file chunks
- Progress bar appears jumpy to users
- Need to balance update frequency with performance

### Technical Insights:
- Mini-chunks provide better UX without significant performance impact
- Time-based throttling prevents event spam
- Atomic operations and proper synchronization needed for accurate progress tracking
- Progress struct enhancement maintains backward compatibility
- Implementation keeps parallel processing benefits while improving progress reporting