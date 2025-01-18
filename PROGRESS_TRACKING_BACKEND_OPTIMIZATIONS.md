# Backend Performance Optimizations

## Progress Summary

Legend:
🔄 = In Progress
⚠️ = Blocked/Has Issues
✅ = Completed

### Current Status:
- Initial optimization plan created
- High-impact improvements identified
- Codebase compatibility verified

### Next Implementation Steps:
1. Error handling consolidation (foundation for other changes)
2. Type-safe result processing
3. Adaptive batch processing
4. Process pool management
5. Parallel task validation
6. Task priority system

## Implementation Plan

### 1. Error Handling Consolidation

[ ] Extend existing error system
   Short description: Add new error variants to existing OptimizerError enum
   Prerequisites: None
   Files to modify:
   - src-tauri/src/utils/error.rs
   Code to add/change/remove/move:
   ```rust
   // Add to existing OptimizerError enum
   #[derive(Error, Debug, Serialize)]
   pub enum OptimizerError {
       // ... existing variants ...
       
       #[error("Batch error: {0}")]
       Batch(String),
       
       #[error("Pool error: {0}")]
       Pool(String),
   }
   ```

### 2. Type-Safe Result Processing

[ ] Implement strongly typed structures
   Short description: Replace JSON parsing with typed structures
   Prerequisites: Error handling consolidation
   Files to modify:
   - src-tauri/src/processing/optimizer.rs
   Code to add/change/remove/move:
   ```rust
   #[derive(Debug, Serialize, Deserialize)]
   pub struct SharpResult {
       path: String,
       optimized_size: u64,
       original_size: u64,
       saved_bytes: i64,
       compression_ratio: String,
       success: bool,
       error: Option<String>,
   }

   impl From<SharpResult> for OptimizationResult {
       fn from(result: SharpResult) -> Self {
           OptimizationResult {
               optimized_path: result.path,
               optimized_size: result.optimized_size,
               original_size: result.original_size,
               saved_bytes: result.saved_bytes,
               compression_ratio: result.compression_ratio.parse().unwrap_or(0.0),
               success: result.success,
               error: result.error,
               original_path: String::new(), // Set by caller
           }
       }
   }
   ```

### 3. Adaptive Batch Processing

[ ] Add dynamic batch sizing
   Short description: Implement simple but effective batch size calculation
   Prerequisites: None
   Files to modify:
   - src-tauri/src/processing/optimizer.rs
   Code to add/change/remove/move:
   ```rust
   impl ImageOptimizer {
       fn calculate_batch_size(&self, tasks: &[ImageTask]) -> usize {
           let cpu_count = num_cpus::get();
           let task_count = tasks.len();
           
           // Simple formula: CPU cores * 2, bounded by task count and limits
           ((cpu_count * 2) as usize)
               .max(4)
               .min(task_count)
               .min(20)
       }

       pub async fn process_batch(&self, app: &tauri::AppHandle, tasks: Vec<ImageTask>) -> OptimizerResult<Vec<OptimizationResult>> {
           let batch_size = self.calculate_batch_size(&tasks);
           debug!("Using batch size: {}", batch_size);
           // ... rest of implementation
       }
   }
   ```

### 4. Process Pool Management

[ ] Implement basic process pooling
   Short description: Add simple process reuse for Sharp sidecar
   Prerequisites: None
   Files to modify:
   - src-tauri/src/processing/optimizer.rs
   Code to add/change/remove/move:
   ```rust
   pub struct ProcessPool {
       processes: Arc<Mutex<Vec<Child>>>,
       max_processes: usize,
   }

   impl ProcessPool {
       pub fn new() -> Self {
           Self {
               processes: Arc::new(Mutex::new(Vec::new())),
               max_processes: num_cpus::get(),
           }
       }

       pub async fn get_process(&self, app: &tauri::AppHandle) -> OptimizerResult<Child> {
           let mut processes = self.processes.lock().await;
           if let Some(process) = processes.pop() {
               return Ok(process);
           }
           
           app.shell().sidecar("sharp-sidecar")?
               .spawn()
               .map_err(|e| OptimizerError::sidecar(e.to_string()))
       }
   }
   ```

### 5. Parallel Task Validation

[ ] Add concurrent validation
   Short description: Simple parallel validation with existing error handling
   Prerequisites: Error handling consolidation
   Files to modify:
   - src-tauri/src/processing/validation.rs
   Code to add/change/remove/move:
   ```rust
   pub async fn validate_tasks_parallel(tasks: &[ImageTask]) -> OptimizerResult<()> {
       let futures: Vec<_> = tasks.iter()
           .map(validate_task)
           .collect();
           
       futures::future::try_join_all(futures).await?;
       Ok(())
   }
   ```

### 6. Task Priority System

[ ] Add basic priority support
   Short description: Simple priority queue implementation
   Prerequisites: None
   Files to modify:
   - src-tauri/src/worker/task.rs
   Code to add/change/remove/move:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum TaskPriority {
       Normal,
       High,
   }

   #[derive(Debug)]
   pub struct PriorityQueue {
       high: VecDeque<ImageTask>,
       normal: VecDeque<ImageTask>,
   }

   impl PriorityQueue {
       pub fn new() -> Self {
           Self {
               high: VecDeque::new(),
               normal: VecDeque::new(),
           }
       }

       pub fn push(&mut self, task: ImageTask, priority: TaskPriority) {
           match priority {
               TaskPriority::High => self.high.push_back(task),
               TaskPriority::Normal => self.normal.push_back(task),
           }
       }

       pub fn pop(&mut self) -> Option<ImageTask> {
           self.high.pop_front().or_else(|| self.normal.pop_front())
       }
   }
   ```

## Implementation Notes
- Keep changes minimal and focused
- Maintain existing error handling patterns
- Preserve current benchmarking capabilities
- Test each optimization independently
- Focus on stability and reliability

## Completed Tasks

None yet.

## Findings

### Known Issues:
- Current fixed batch size may not be optimal
- JSON parsing adds overhead
- Sequential validation limits throughput

### Technical Insights:
- Existing error handling is well-structured
- Current architecture supports incremental optimization
- Benchmarking system can measure improvements