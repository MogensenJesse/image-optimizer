# Parallel Processing Implementation

## Progress Summary

Legend:
🔄 = In Progress
⚠️ = Blocked/Has Issues
✅ = Completed

### Current Status:
Planning phase - Implementing parallel processing architecture with static batch sizing

### Next Implementation Steps:
1. Implement parallel process pool
2. Add worker thread support to Sharp sidecar
3. Update metrics collection
4. Integrate parallel processing with existing commands

## Implementation Plan

### 1. Parallel Process Pool Implementation

[ ] Create new parallel process pool structure
   Short description: Implement the core parallel processing pool structure
   Prerequisites: None
   Files to modify: src-tauri/src/processing/pool/process_pool.rs
   External dependencies: None
   Code to add:
   ```rust
   pub struct ParallelProcessPool {
       semaphore: Arc<Semaphore>,
       app: tauri::AppHandle,
       max_size: usize,
       active_count: Arc<Mutex<usize>>,
       metrics: Arc<Mutex<ProcessPoolMetrics>>,
       task_queue: Arc<Mutex<VecDeque<QueuedTask>>>,
       batch_size: usize,  // Static 75 images per batch
   }

   impl ParallelProcessPool {
       pub fn new(app: tauri::AppHandle) -> Self {
           let size = Self::calculate_optimal_processes();
           Self::new_with_size(app, size)
       }

       fn calculate_optimal_processes() -> usize {
           let cpu_count = num_cpus::get();
           ((cpu_count * 3) / 4).max(2).min(24)
       }
   }
   ```

[ ] Implement batch processing logic
   Short description: Add methods for parallel batch processing
   Prerequisites: Parallel process pool structure
   Files to modify: src-tauri/src/processing/pool/process_pool.rs
   Code to add:
   ```rust
   impl ParallelProcessPool {
       pub async fn process_batch(&self, tasks: Vec<ImageTask>) 
           -> OptimizerResult<Vec<OptimizationResult>> {
           let batches = self.create_batches(tasks);
           let mut handles = Vec::new();
           
           for batch in batches {
               let handle = self.process_single_batch(batch).await?;
               handles.push(handle);
           }
           
           self.collect_results(handles).await
       }

       fn create_batches(&self, tasks: Vec<ImageTask>) -> Vec<Vec<ImageTask>> {
           tasks.chunks(75).map(|chunk| chunk.to_vec()).collect()
       }
   }
   ```

### 2. Sharp Sidecar Worker Implementation

[ ] Add worker thread support to Sharp sidecar
   Short description: Implement worker thread pool in Node.js sidecar
   Prerequisites: None
   Files to modify: sharp-sidecar/index.js
   External dependencies: worker_threads (Node.js built-in)
   Code to add:
   ```javascript
   const { Worker, isMainThread, parentPort } = require('worker_threads');
   const sharp = require('sharp');
   const os = require('os');

   class SharpWorkerPool {
       constructor() {
           this.maxWorkers = os.cpus().length;
           this.workers = new Map();
       }

       async processImage(task) {
           // Worker implementation
       }
   }
   ```

[ ] Implement parallel image processing
   Short description: Add parallel processing support to optimize-batch command
   Prerequisites: Worker thread support
   Files to modify: sharp-sidecar/index.js
   Code to add:
   ```javascript
   async function optimizeBatch(batchJson) {
       const batch = JSON.parse(batchJson);
       const workerPool = new SharpWorkerPool();
       
       const results = await Promise.all(
           batch.map(task => workerPool.processImage(task))
       );
       
       console.log(JSON.stringify(results));
       return results;
   }
   ```

### 3. Metrics Implementation

[ ] Update metrics structure
   Short description: Add basic parallel processing metrics
   Prerequisites: None
   Files to modify: src-tauri/src/benchmarking/metrics.rs
   Code to add:
   ```rust
   pub struct ParallelProcessingMetrics {
       pub total_batches: usize,
       pub concurrent_batches: usize,
       pub batch_processing_times: Vec<Duration>,
       pub total_processed: usize,
   }

   impl ParallelProcessingMetrics {
       pub fn record_batch_completion(&mut self, duration: Duration, size: usize) {
           self.total_batches += 1;
           self.batch_processing_times.push(duration);
           self.total_processed += size;
       }
   }
   ```

[ ] Integrate metrics collection
   Short description: Add metrics collection to parallel processing
   Prerequisites: Updated metrics structure
   Files to modify: 
   - src-tauri/src/processing/pool/process_pool.rs
   - src-tauri/src/benchmarking/reporter.rs
   Code to add/change: Add metrics collection points in process_batch method

### 4. Integration and Testing

[ ] Update image commands
   Short description: Integrate parallel processing with existing commands
   Prerequisites: All above implementations
   Files to modify: src-tauri/src/commands/image.rs
   Code changes: Update optimize_images command to use parallel processing

[ ] Add basic error handling
   Short description: Implement error handling for parallel processing
   Prerequisites: Command integration
   Files to modify: src-tauri/src/processing/pool/process_pool.rs
   Code to add: Error handling for batch processing failures

### 5. Build Process Integration

[ ] Update pkg configuration
   Short description: Ensure worker threads are properly bundled
   Prerequisites: Worker thread implementation
   Files to modify: sharp-sidecar/package.json
   Code to add:
   ```json
   {
     "pkg": {
       "assets": [
         "node_modules/sharp/**/*",
         "node_modules/@img/sharp-win32-x64/**/*",
         "optimizationDefaults.js",
         "worker.js"
       ],
       "targets": ["node20-win-x64"],
       "scripts": ["worker.js"]
     }
   }
   ```

[ ] Add worker script bundling
   Short description: Create separate worker script for pkg bundling
   Prerequisites: pkg configuration update
   Files to modify: 
   - sharp-sidecar/worker.js
   - sharp-sidecar/index.js
   Code to add:
   ```javascript
   // worker.js
   const { parentPort } = require('worker_threads');
   const sharp = require('sharp');
   const { optimizeImage } = require('./optimize');

   parentPort.on('message', async (task) => {
     try {
       const result = await optimizeImage(task);
       parentPort.postMessage({ type: 'success', result });
     } catch (error) {
       parentPort.postMessage({ type: 'error', error: error.message });
     }
   });
   ```

### 6. IPC Optimization

[ ] Implement batched progress updates
   Short description: Optimize IPC communication for parallel processing
   Prerequisites: Parallel metrics implementation
   Files to modify: src-tauri/src/processing/pool/process_pool.rs
   Code to add:
   ```rust
   pub struct BatchedProgress {
       updates: Vec<ProgressUpdate>,
       last_emit: Instant,
       batch_interval: Duration,
   }

   impl BatchedProgress {
       pub fn new() -> Self {
           Self {
               updates: Vec::new(),
               last_emit: Instant::now(),
               batch_interval: Duration::from_millis(100),
           }
       }

       pub fn add_update(&mut self, update: ProgressUpdate) {
           self.updates.push(update);
           self.try_emit();
       }

       fn try_emit(&mut self) {
           if self.last_emit.elapsed() >= self.batch_interval {
               // Emit batched updates
               self.emit_updates();
           }
       }
   }
   ```

[ ] Add progress debouncing
   Short description: Prevent IPC channel flooding with progress updates
   Prerequisites: Batched progress implementation
   Files to modify: src-tauri/src/processing/pool/process_pool.rs
   Code to add:
   ```rust
   impl ParallelProcessPool {
       async fn emit_progress(&self, progress: ProgressUpdate) {
           let mut batched = self.batched_progress.lock().await;
           batched.add_update(progress);
       }
   }
   ```

[ ] Optimize metrics collection
   Short description: Implement efficient metrics aggregation
   Prerequisites: Progress debouncing
   Files to modify: src-tauri/src/benchmarking/metrics.rs
   Code to add:
   ```rust
   impl ParallelProcessingMetrics {
       pub fn aggregate_batch_metrics(&mut self, updates: Vec<ProgressUpdate>) {
           let mut total_processed = 0;
           let mut total_time = Duration::ZERO;

           for update in updates {
               total_processed += update.processed_files;
               total_time += update.processing_time;
           }

           self.update_averages(total_processed, total_time);
       }
   }
   ```

## Implementation Notes
- Keep batch size static at 75 images
- Focus on parallel processing implementation first
- Keep metrics simple and focused on essential measurements
- Maintain existing API contracts
- Test thoroughly with varying batch sizes
- Ensure worker threads are properly bundled in production builds
- Optimize IPC communication to prevent channel flooding
- Monitor memory usage with parallel processing

## Completed Tasks

## Findings

### Known Issues:
- None yet

### Technical Insights:
- Static batch size of 75 images provides a good balance
- Worker thread implementation in Sharp sidecar crucial for performance
- Simple metrics provide essential insights without overhead