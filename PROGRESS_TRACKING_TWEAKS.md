# Image Processing System Optimizations

## Progress Summary

Legend:
🔄 = In Progress
⚠️ = Blocked/Has Issues
✅ = Completed

### Current Status:
- System functional but has room for performance improvements
- Memory utilization improved (target 70% of available) ✅
- Process pool now uses dynamic sizing based on CPU cores ✅
- Sequential chunk processing
- Uneven worker task distribution

### Next Implementation Steps:
1. ✅ Dynamic process pool sizing
2. ✅ Improved batch size configuration
3. Parallel chunk processing
4. Worker task distribution balancing
5. Process warmup and reuse
6. Memory limit optimization

## Implementation Plan

### 1. Dynamic Process Pool Sizing ✅

[x] Implement dynamic process count
   Short description: Scale process count based on CPU cores
   Prerequisites: None
   Files modified: src-tauri/src/processing/optimizer.rs
   External dependencies: num_cpus
   Changes made:
   ```rust
   // Added dynamic process count calculation
   impl ProcessPool {
       fn calculate_optimal_processes() -> usize {
           let cpu_count = num_cpus::get();
           // Use half of CPU cores, with min 2 and max 16
           (cpu_count / 2).max(2).min(16)
       }

       pub fn new(app: tauri::AppHandle) -> Self {
           let size = Self::calculate_optimal_processes();
           debug!("Creating process pool with {} processes (based on {} CPU cores)", size, num_cpus::get());
           Self::new_with_size(app, size)
       }
   }

   // Updated ImageOptimizer to use dynamic process count
   impl ImageOptimizer {
       pub fn new(app: tauri::AppHandle) -> Self {
           Self {
               active_tasks: Arc::new(Mutex::new(HashSet::new())),
               process_pool: ProcessPool::new(app),
           }
       }
   }
   ```
   [x] Cleanup completed:
    - Updated ImageOptimizer creation to use dynamic process count
    - Added better debug logging for process pool creation
    - Maintained backward compatibility with new_with_size method
    - Process count now scales with available CPU cores

### 2. Batch Size Configuration ✅

[x] Update batch size parameters
   Short description: Optimize batch sizes and memory usage
   Prerequisites: None
   Files modified: src-tauri/src/processing/optimizer.rs
   External dependencies: None
   Changes made:
   ```rust
   struct BatchSizeConfig {
       min_size: 10,          // Increased from 5
       max_size: 100,         // Increased from 75
       target_memory_usage: usize,
       target_memory_percentage: 0.7,  // Increased from 0.5
       tasks_per_process: 20,          // New parameter
   }
   ```
   Key improvements:
   - Increased memory utilization (70% of available)
   - Added process-aware batch sizing
   - Better handling of empty/small batches
   - Improved batch size calculation logic
   - Enhanced debug logging

   [x] Cleanup completed:
    - Updated batch size calculations to consider process count
    - Added process-based size calculation
    - Improved error handling for empty tasks
    - Added detailed debug logging
    - Optimized memory target (4GB limit)

### 2.1. Windows Long Path Support

[ ] Enable Windows long path support
   Short description: Add support for paths exceeding MAX_PATH (260 characters)
   Prerequisites: None
   Files to modify: 
   - src-tauri/src/utils/fs.rs
   External dependencies: None
   Implementation steps:

   1. [ ] Add path normalization utilities
   ```rust
   // src-tauri/src/utils/fs.rs
   
   /// Constants for Windows path handling
   #[cfg(windows)]
   const MAX_PATH: usize = 260;
   #[cfg(windows)]
   const UNC_PREFIX: &str = "\\\\?\\";

   /// Normalize paths to handle Windows MAX_PATH limitation
   pub fn normalize_long_path(path: &Path) -> PathBuf {
       if !cfg!(windows) {
           return path.to_path_buf();
       }
       
       let absolute = if path.is_absolute() {
           path.to_path_buf()
       } else {
           std::env::current_dir()
               .unwrap_or_default()
               .join(path)
       };
       
       // Only prefix with \\?\ if path doesn't already have it and is longer than MAX_PATH
       let path_str = absolute.to_string_lossy();
       if path_str.len() > MAX_PATH && !path_str.starts_with(UNC_PREFIX) {
           PathBuf::from(format!("{}{}", UNC_PREFIX, absolute.display()))
       } else {
           absolute
       }
   }

   // Update existing functions to use normalize_long_path
   pub async fn get_file_size(path: impl AsRef<Path>) -> OptimizerResult<u64> {
       let path = normalize_long_path(path.as_ref());
       fs::metadata(&path)
           .await
           .map(|m| m.len())
           .map_err(|e| OptimizerError::io(format!("Failed to get file size: {}", e)))
   }

   pub async fn file_exists(path: impl AsRef<Path>) -> bool {
       normalize_long_path(path.as_ref()).exists()
   }

   pub async fn dir_exists(path: impl AsRef<Path>) -> bool {
       let path = normalize_long_path(path.as_ref());
       path.exists() && path.is_dir()
   }

   pub fn get_extension(path: impl AsRef<Path>) -> OptimizerResult<String> {
       let path = normalize_long_path(path.as_ref());
       path.extension()
           .and_then(|e| e.to_str())
           .map(|e| e.to_lowercase())
           .ok_or_else(|| OptimizerError::validation(
               format!("File has no extension: {}", path.display())
           ))
   }
   ```

   [ ] Cleanup tasks:
    - Test with paths > 260 characters
    - Verify path normalization in all components
    - Add debug logging for path transformations
    - Test on non-Windows platforms to ensure compatibility

### 3. Parallel Chunk Processing

[ ] Implement parallel chunk processing
   Short description: Process multiple chunks concurrently
   Prerequisites: None
   Files to modify: src-tauri/src/processing/optimizer.rs
   External dependencies: futures crate
   Code to add/change/remove/move:
   ```rust
   use futures::stream::{FuturesUnordered, StreamExt};
   
   async fn process_chunks(&self, tasks: Vec<ImageTask>) {
       let chunk_futures: FuturesUnordered<_> = tasks
           .chunks(self.batch_size)
           .map(|chunk| self.process_chunk(chunk.to_vec()))
           .collect();
       
       while let Some(result) = chunk_futures.next().await {
           // Handle results
       }
   }
   ```
   [ ] Cleanup after moving code:
    - Update error handling
    - Ensure proper resource cleanup

### 4. Worker Distribution Improvement

[ ] Balance worker task distribution
   Short description: Evenly distribute tasks across workers
   Prerequisites: None
   Files to modify: src-tauri/src/worker/pool.rs
   External dependencies: None
   Code to add/change/remove/move:
   ```rust
   impl WorkerPool {
       fn calculate_tasks_per_worker(&self, total_tasks: usize) -> usize {
           (total_tasks as f32 / self.worker_count as f32).ceil() as usize
       }
   }
   ```
   [ ] Cleanup after moving code:
    - Update task assignment logic
    - Adjust worker metrics

### 5. Process Warmup System

[ ] Implement process warmup
   Short description: Maintain warm processes to reduce startup overhead
   Prerequisites: None
   Files to modify: src-tauri/src/processing/optimizer.rs
   External dependencies: None
   Code to add/change/remove/move:
   ```rust
   impl ProcessPool {
       async fn ensure_warm_processes(&self, min_count: usize) {
           while self.active_count.load(Ordering::Relaxed) < min_count {
               if let Ok(process) = self.spawn_process().await {
                   self.warm_processes.lock().await.push(process);
               }
           }
       }
   }
   ```
   [ ] Cleanup after moving code:
    - Add process cleanup
    - Update metrics tracking

### 6. Memory Management

[ ] Optimize memory limits
   Short description: Scale memory usage based on system capacity
   Prerequisites: None
   Files to modify: src-tauri/src/processing/optimizer.rs
   External dependencies: None
   Code to add/change/remove/move:
   ```rust
   impl ImageOptimizer {
       fn get_memory_config(&self) -> (usize, f32) {
           let available = self.get_available_memory();
           let percentage = if available > 16_000_000_000 { 0.7 } else { 0.5 };
           (available, percentage)
       }
   }
   ```
   [ ] Cleanup after moving code:
    - Update batch size calculations
    - Adjust memory metrics

## Implementation Notes
- Each optimization can be implemented independently
- Focus on maintaining stability while improving performance
- Keep error handling consistent
- Preserve existing API contracts
- Test with various batch sizes and image counts

## Technical Insights:
- Current memory usage is very low (31MB peak for 72 images)
- Process pool is underutilized (only 1-4 processes used)
- Worker distribution is uneven (3/2 split)
- Chunk processing is sequential
- Process startup overhead impacts small batches

## Completed Tasks

### Example of completed task (✅)
- Dynamic process pool sizing

## Findings

### Known Issues:

### Technical Insights: