# Backend Optimizations

## Progress Summary
🔄 = In Progress
⚠️ = Blocked/Has Issues
✅ = Completed

Current Status:
- [🔄] Worker Module Restructuring
- [⚠️] Benchmarking Logic Separation
- [🔄] Error Handling Improvements

## Implementation Plan

### 1. Worker Module Restructuring
**Purpose**: Improve module independence and code organization

#### 1.1 Create New Module Structure
```rust
// worker/mod.rs
mod pool;
mod task;
mod error;

pub use pool::WorkerPool;
pub use task::ImageTask;
pub use error::{WorkerError, WorkerResult};
```

#### 1.2 Move ImageTask Definition
Move from: `core/types.rs`
To: `worker/task.rs`
```rust
#[derive(Debug, Clone)]
pub struct ImageTask {
    pub input_path: String,
    pub output_path: Option<String>,
    pub settings: ImageSettings,
    pub task_id: String,
}

impl ImageTask {
    pub fn new(input: String, output: Option<String>, settings: ImageSettings) -> Self {
        Self {
            input_path: input,
            output_path: output,
            settings,
            task_id: Uuid::new_v4().to_string(),
        }
    }
    
    // Move all associated methods
    pub fn get_output_path(&self) -> String { ... }
    pub fn validate(&self) -> Result<(), ValidationError> { ... }
}
```

#### 1.3 Update Pool Implementation
In: `worker/pool.rs`
```rust
// Update imports
use super::task::ImageTask;
use super::error::{WorkerError, WorkerResult};
use crate::benchmarking::{BenchmarkMetrics, Duration, ProcessingStage};

pub struct WorkerPool {
    optimizer: ImageOptimizer,
    app: AppHandle,
    active_workers: Arc<Mutex<usize>>,
    semaphore: Arc<Semaphore>,
    worker_count: usize,
    benchmark_metrics: Arc<Mutex<Option<BenchmarkMetrics>>>,
}

// Keep existing implementations but update error handling
```

#### 1.4 Update Import References
Files to update:
```rust
// lib.rs
pub mod worker;
pub use worker::{WorkerPool, ImageTask, WorkerError, WorkerResult};

// core/types.rs
- Remove ImageTask
+ pub use crate::worker::ImageTask;

// processing/optimizer.rs
- use crate::core::types::ImageTask;
+ use crate::worker::ImageTask;

// commands/image.rs
- use crate::core::types::ImageTask;
+ use crate::worker::ImageTask;

// utils/validation.rs
- use crate::core::types::ImageTask;
+ use crate::worker::ImageTask;
```

Tasks:
- [ ] Create new module files and directory structure
- [ ] Move ImageTask from core/types.rs to worker/task.rs
- [ ] Update all import references across the codebase
- [ ] Add comprehensive module documentation
- [ ] Verify all tests pass after restructuring

Dependencies:
- None (can be implemented independently)

### 2. Benchmarking Logic Separation
**Purpose**: Decouple benchmarking from worker implementation

#### 2.1 Create Benchmarking Trait
In: `benchmarking/metrics.rs`
```rust
pub trait Benchmarkable {
    fn enable_benchmarking(&mut self);
    fn record_processing_time(&mut self, duration: Duration);
    fn record_worker_metrics(&mut self, worker_id: usize, idle_time: Duration, busy_time: Duration);
    fn record_stage_time(&mut self, stage: ProcessingStage, duration: Duration);
    fn record_queue_metrics(&mut self, length: usize, contention: bool);
    fn finalize_benchmarking(&mut self) -> BenchmarkMetrics;
}

// Add implementation for existing BenchmarkMetrics
impl BenchmarkMetrics {
    // Move worker-specific methods here from WorkerPool
    pub fn record_worker_metrics(&mut self, worker_id: usize, idle_time: Duration, busy_time: Duration) { ... }
    pub fn record_queue_metrics(&mut self, length: usize, contention: bool) { ... }
}
```

#### 2.2 Update WorkerPool Implementation
In: `worker/pool.rs`
```rust
impl Benchmarkable for WorkerPool {
    fn enable_benchmarking(&mut self) {
        let mut metrics = self.benchmark_metrics.try_lock()
            .expect("Failed to lock benchmark metrics");
        *metrics = Some(BenchmarkMetrics::new_with_capacity(self.worker_count));
    }

    fn record_worker_metrics(&mut self, worker_id: usize, idle_time: Duration, busy_time: Duration) {
        if let Ok(mut metrics) = self.benchmark_metrics.try_lock() {
            if let Some(ref mut m) = *metrics {
                m.record_worker_metrics(worker_id, idle_time, busy_time);
            }
        }
    }
    // Implement other trait methods
}
```

Tasks:
- [ ] Add Benchmarkable trait to benchmarking/metrics.rs
- [ ] Move worker-specific metrics methods to BenchmarkMetrics
- [ ] Implement Benchmarkable for WorkerPool
- [ ] Update existing benchmarking calls in WorkerPool
- [ ] Add comprehensive documentation for benchmarking trait

Dependencies:
- None (can be implemented independently)

### 3. Error Handling Improvements
**Purpose**: Better error separation and handling

#### 3.1 Create Worker Error Type
In: `worker/error.rs`
```rust
#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    #[error("Worker initialization failed: {0}")]
    InitializationError(String),
    
    #[error("Task processing failed: {0}")]
    ProcessingError(String),
    
    #[error("Worker pool is at capacity: {0}")]
    CapacityError(String),
    
    #[error("Worker state error: {0}")]
    StateError(String),
    
    #[error(transparent)]
    OptimizerError(#[from] crate::utils::OptimizerError),
}

pub type WorkerResult<T> = Result<T, WorkerError>;
```

#### 3.2 Update Error Conversions
In: `worker/pool.rs`
```rust
use super::error::{WorkerError, WorkerResult};

impl From<tokio::sync::AcquireError> for WorkerError {
    fn from(err: tokio::sync::AcquireError) -> Self {
        WorkerError::StateError(format!("Failed to acquire semaphore: {}", err))
    }
}

impl From<std::io::Error> for WorkerError {
    fn from(err: std::io::Error) -> Self {
        WorkerError::ProcessingError(format!("IO error during processing: {}", err))
    }
}
```

#### 3.3 Update Worker Pool Error Handling
In: `worker/pool.rs`
```rust
impl WorkerPool {
    pub async fn process(&self, task: ImageTask) -> WorkerResult<OptimizationResult> {
        let _permit = self.semaphore.acquire().await
            .map_err(|e| WorkerError::CapacityError(format!("Failed to acquire worker: {}", e)))?;

        // Update error handling in processing logic
        match self.optimizer.process_batch(&self.app, vec![task]).await {
            Ok((results, _)) => {
                results.into_iter().next()
                    .ok_or_else(|| WorkerError::ProcessingError("No result returned".into()))
            }
            Err(e) => Err(WorkerError::from(e))
        }
    }
}
```

#### 3.4 Add Error Context
In: `worker/pool.rs`
```rust
// Add error context helper
fn with_context<T, E>(result: Result<T, E>, context: impl FnOnce() -> String) -> WorkerResult<T>
where
    E: std::error::Error + Send + Sync + 'static
{
    result.map_err(|e| WorkerError::ProcessingError(format!("{}: {}", context(), e)))
}

// Usage in methods
async fn initialize_worker(&self) -> WorkerResult<()> {
    with_context(
        self.setup_worker_resources().await,
        || format!("Failed to initialize worker {}", self.worker_id)
    )
}
```

#### 3.5 Improve Error Logging
In: `worker/pool.rs`
```rust
use tracing::{error, warn, info, debug};

impl WorkerPool {
    async fn handle_task_error(&self, error: WorkerError, task: &ImageTask) {
        error!(
            task_id = %task.task_id,
            error = %error,
            input_path = %task.input_path,
            "Task processing failed"
        );
        
        // Additional error context for specific error types
        match &error {
            WorkerError::CapacityError(_) => {
                warn!("Worker pool capacity reached, consider scaling workers");
            }
            WorkerError::StateError(_) => {
                error!("Worker pool in invalid state, may require restart");
            }
            _ => {}
        }
    }
}
```

Tasks:
- [ ] Create dedicated WorkerError type
- [ ] Implement proper error conversion traits
- [ ] Update error handling in worker pool
- [ ] Add context to error messages
- [ ] Improve error logging

Dependencies:
- None (can be implemented independently)

### 4. State Management Consolidation
**Purpose**: Cleaner state handling in worker pool

#### 4.1 Add State Management to WorkerPool
In: `worker/pool.rs`
```rust
impl WorkerPool {
    // Add state validation and management methods
    fn validate_state(&self) -> WorkerResult<()> {
        if self.worker_count < MIN_WORKERS || self.worker_count > MAX_WORKERS {
            return Err(WorkerError::StateError(
                format!("Invalid worker count: {}", self.worker_count)
            ));
        }
        Ok(())
    }

    fn update_worker_state(&self, worker_id: usize, state: WorkerState) -> WorkerResult<()> {
        let mut count = self.active_workers.lock().await;
        match state {
            WorkerState::Active => *count += 1,
            WorkerState::Inactive => *count = count.saturating_sub(1),
        }
        Ok(())
    }
}

// Add worker state enum
#[derive(Debug, Clone, Copy)]
enum WorkerState {
    Active,
    Inactive,
}
```

Tasks:
- [ ] Add state validation methods to WorkerPool
- [ ] Implement worker state management
- [ ] Add state transition logging
- [ ] Update process method to use state management
- [ ] Add state recovery mechanisms

Dependencies:
- Requires Worker Module Restructuring

## Completed Tasks
None yet - Implementation starting

## Notes
- Keep existing thread-safe patterns (Arc<Mutex<>>)
- Maintain current logging detail level
- Ensure backward compatibility during refactoring
- Consider adding integration tests for new structure
- State management remains in WorkerPool for tight coupling
- Benchmarking trait integrates with existing BenchmarkMetrics