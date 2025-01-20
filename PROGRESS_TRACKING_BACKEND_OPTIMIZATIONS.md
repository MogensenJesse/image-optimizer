# Backend Optimization Progress

## Progress Summary

Legend:
🔄 = In Progress
⚠️ = Blocked/Has Issues
✅ = Completed

### Current Status:
- Format handling improvements completed
- Ready to implement high-impact optimizations

### Next Implementation Steps:
1. Implement adaptive batch sizing
2. Add strongly typed result structures
3. Consolidate error handling
4. Implement parallel task validation
5. Add process pooling for Sharp sidecar
6. Implement task priority support

## Implementation Plan

### 1. Adaptive Batch Sizing

[ ] Implement dynamic batch size calculation
   Short description: Replace fixed batch size with dynamic calculation based on system resources and image sizes
   Prerequisites: None
   Files to modify: 
   - src-tauri/src/processing/optimizer.rs
   - src-tauri/src/core/state.rs
   External dependencies: None
   Code to add:
   ```rust
   // In src-tauri/src/processing/optimizer.rs
   
   struct BatchSizeConfig {
       min_size: usize,
       max_size: usize,
       target_memory_usage: usize, // in bytes
   }
   
   impl ImageOptimizer {
       fn calculate_batch_size(&self, tasks: &[ImageTask]) -> usize {
           let config = BatchSizeConfig {
               min_size: 5,
               max_size: 20,
               target_memory_usage: 1024 * 1024 * 512, // 512MB target
           };
           
           // Calculate average task size
           let avg_size: u64 = tasks.iter()
               .filter_map(|t| std::fs::metadata(&t.input_path).ok())
               .map(|m| m.len())
               .sum::<u64>() / tasks.len() as u64;
           
           // Calculate batch size based on memory target
           let calculated_size = config.target_memory_usage / avg_size as usize;
           calculated_size.clamp(config.min_size, config.max_size)
       }
   
       pub async fn process_batch(&self, app: &tauri::AppHandle, tasks: Vec<ImageTask>) -> OptimizerResult<Vec<OptimizationResult>> {
           let batch_size = self.calculate_batch_size(&tasks);
           debug!("Calculated optimal batch size: {}", batch_size);
           
           // Rest of the implementation...
       }
   }
   ```

### 2. Strongly Typed Result Structures

[ ] Replace JSON value parsing with typed structures
   Short description: Implement strongly typed structures for Sharp sidecar results
   Prerequisites: None
   Files to modify:
   - src-tauri/src/processing/optimizer.rs
   - sharp-sidecar/index.js
   External dependencies: None
   Code to add:
   ```rust
   // In src-tauri/src/processing/optimizer.rs
   
   #[derive(Debug, Deserialize)]
   struct SharpResult {
       path: String,
       optimized_size: u64,
       original_size: u64,
       saved_bytes: i64,
       compression_ratio: String,
       success: bool,
       error: Option<String>,
   }
   
   impl ImageOptimizer {
       async fn process_chunk(&self, app: &tauri::AppHandle, tasks: Vec<ImageTask>) -> OptimizerResult<Vec<OptimizationResult>> {
           // ... existing code ...
           
           let stdout = String::from_utf8_lossy(&output.stdout);
           let sharp_results: Vec<SharpResult> = serde_json::from_str(&stdout)
               .map_err(|e| OptimizerError::processing(format!(
                   "Failed to parse Sharp output: {}", e
               )))?;
           
           // Convert to OptimizationResult
           let results = sharp_results.into_iter()
               .map(|sr| OptimizationResult {
                   original_path: task.input_path,
                   optimized_path: sr.path,
                   original_size: sr.original_size,
                   optimized_size: sr.optimized_size,
                   success: sr.success,
                   error: sr.error,
                   saved_bytes: sr.saved_bytes,
                   compression_ratio: sr.compression_ratio.parse().unwrap_or(0.0),
               })
               .collect();
           
           Ok(results)
       }
   }
   ```

### 3. Parallel Task Validation

[ ] Implement concurrent task validation
   Short description: Validate tasks in parallel using tokio tasks
   Prerequisites: None
   Files to modify:
   - src-tauri/src/processing/optimizer.rs
   - src-tauri/src/processing/validation.rs
   External dependencies: None
   Code to add:
   ```rust
   // In src-tauri/src/processing/optimizer.rs
   
   use futures::future::try_join_all;
   
   impl ImageOptimizer {
       async fn validate_tasks(&self, tasks: &[ImageTask]) -> OptimizerResult<()> {
           let validation_tasks: Vec<_> = tasks.iter()
               .map(|task| {
                   let task = task.clone();
                   tokio::spawn(async move {
                       validate_task(&task).await
                   })
               })
               .collect();
   
           // Wait for all validations to complete
           let results = try_join_all(validation_tasks).await
               .map_err(|e| OptimizerError::processing(format!("Task validation failed: {}", e)))?;
   
           // Check results and collect any errors
           let errors: Vec<_> = results
               .into_iter()
               .filter_map(|r| r.err())
               .collect();
   
           if !errors.is_empty() {
               return Err(OptimizerError::processing(format!(
                   "Validation failed for {} tasks: {:?}",
                   errors.len(),
                   errors
               )));
           }
   
           Ok(())
       }
   
       async fn process_chunk(&self, app: &tauri::AppHandle, tasks: Vec<ImageTask>) -> OptimizerResult<Vec<OptimizationResult>> {
           // Validate all tasks in parallel
           self.validate_tasks(&tasks).await?;
           
           // Rest of the implementation...
       }
   }
   ```

### 4. Sharp Sidecar Process Pooling

[ ] Implement process pool for Sharp sidecar
   Short description: Create and manage a pool of Sharp sidecar processes for better resource utilization
   Prerequisites: None
   Files to modify:
   - src-tauri/src/processing/optimizer.rs
   External dependencies: None
   Code to add:
   ```rust
   // In src-tauri/src/processing/optimizer.rs
   
   use tokio::sync::Semaphore;
   
   struct ProcessPool {
       processes: Arc<Mutex<Vec<tauri::api::process::Command>>>,
       semaphore: Arc<Semaphore>,
   }
   
   impl ProcessPool {
       fn new(size: usize) -> Self {
           Self {
               processes: Arc::new(Mutex::new(Vec::with_capacity(size))),
               semaphore: Arc::new(Semaphore::new(size)),
           }
       }
   
       async fn acquire(&self, app: &tauri::AppHandle) -> OptimizerResult<tauri::api::process::Command> {
           let _permit = self.semaphore.acquire().await.map_err(|e| 
               OptimizerError::sidecar(format!("Failed to acquire process: {}", e))
           )?;
   
           let mut processes = self.processes.lock().await;
           if let Some(process) = processes.pop() {
               Ok(process)
           } else {
               Ok(app.shell().sidecar("sharp-sidecar").map_err(|e| 
                   OptimizerError::sidecar(format!("Failed to create Sharp sidecar: {}", e))
               )?)
           }
       }
   
       async fn release(&self, process: tauri::api::process::Command) {
           let mut processes = self.processes.lock().await;
           processes.push(process);
       }
   }
   ```

[ ] Cleanup after moving code (if applicable):
    - Update imports in affected files
    - Update function calls to use new pooling system
    - Add proper error handling for pool operations

### 5. Format Handling Improvements

[ ] Implement original format preservation
   Short description: Add support for 'original' format selection
   Prerequisites: None
   Files to modify:
   - src-tauri/src/utils/validation.rs
   - sharp-sidecar/index.js
   - src-tauri/src/processing/optimizer.rs
   External dependencies: None
   Code to add:
   ```rust
   // In src-tauri/src/utils/validation.rs
   if !["jpeg", "jpg", "png", "webp", "avif", "original"].contains(&format.as_str()) {
       return Err(ValidationError::settings(
           format!("Unsupported output format: {}", format)
       ).into());
   }
   ```

   ```javascript
   // In sharp-sidecar/index.js
   const outputFormat = settings?.outputFormat === 'original' ? 
       metadata.format : 
       settings.outputFormat;
   ```

   ```rust
   // In src-tauri/src/processing/optimizer.rs
   if task.settings.output_format.to_lowercase() == "original" {
       let format = format_from_extension(&task.input_path)?;
       task.settings.output_format = format.to_string();
   }
   ```

## Implementation Notes
- Each optimization should be implemented and tested independently
- Maintain backward compatibility during implementation
- Add comprehensive logging for debugging
- Include proper error handling for each new feature
- Test with various image sizes and formats

## Findings

### Known Issues:
- Fixed batch size may not be optimal for all scenarios
- JSON parsing overhead in result processing
- Sequential task validation creates bottleneck
- Single Sharp process may limit throughput
- Original format handling was incomplete

### Technical Insights:
- Adaptive batch sizing can significantly improve memory usage
- Strongly typed structures reduce runtime errors
- Parallel validation can improve processing speed
- Process pooling can better utilize system resources
- Proper format handling improves user experience