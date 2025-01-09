## 9. Backend Modularization 🏗️

### Dependencies ✨
The modularization will use existing dependencies from `Cargo.toml`, no changes required:
- Async runtime: `tokio`
- Channel communication: `crossbeam-channel`
- Synchronization: `parking_lot`
- System monitoring: `sysinfo`

### Required Types ✨
```rust
// Core interfaces using existing patterns
pub trait ImageProcessor {
    fn process(&self, task: ImageTask) -> impl Future<Output = Result<OptimizationResult, String>>;
}

pub trait ProgressTracker {
    fn update(&self, progress: ProcessingProgress) -> impl Future<Output = Result<(), String>>;
    fn get_metrics(&self) -> impl Future<Output = Result<ProgressMetrics, String>>;
}

pub trait WorkerManager {
    fn spawn_worker(&self, id: usize) -> impl Future<Output = Result<(), String>>;
    fn monitor_health(&self) -> impl Future<Output = Result<WorkerHealth, String>>;
}

// Core types matching current implementation
pub struct ImageTask {
    pub input_path: String,
    pub output_path: String,
    pub settings: ImageSettings,
    pub priority: u8,
}

pub struct WorkerHealth {
    pub is_healthy: bool,
    pub active_workers: usize,
    pub last_active: Instant,
}

pub struct ProgressMetrics {
    pub processed_files: usize,
    pub total_files: usize,
    pub bytes_processed: u64,
    pub bytes_saved: i64,
    pub elapsed_time: f64,
    pub estimated_remaining: f64,
}
```

### Module Structure 🔄
```
src/
├── lib.rs               # Module declarations and public exports
├── commands/
│   ├── mod.rs          # Command module exports
│   ├── image.rs        # Image optimization commands (~50 lines)
│   └── worker.rs       # Worker management commands (~30 lines)
├── core/
│   ├── mod.rs          # Core module exports
│   ├── types.rs        # Shared type definitions from image.rs
│   └── state.rs        # AppState and WORKER_POOL from image.rs
├── worker/
│   ├── mod.rs          # Worker module exports
│   ├── pool.rs         # WorkerPool core logic (~200 lines)
│   ├── manager.rs      # Worker lifecycle & tasks (~200 lines)
│   └── metrics.rs      # Worker performance tracking (~100 lines)
├── progress/
│   ├── mod.rs          # Progress module exports
│   ├── tracker.rs      # Progress state management (~100 lines)
│   └── debouncer.rs    # Event debouncing from progress_debouncer.rs
└── processing/
    ├── mod.rs          # Processing module exports
    ├── optimizer.rs    # Image optimization logic from worker_pool.rs
    └── validation.rs   # Input/output validation
```

### File Content Migration 📦

1. **From image.rs to core/types.rs**
```rust
pub struct OptimizationResult {...}
pub struct ResizeSettings {...}
pub struct QualitySettings {...}
pub struct ImageSettings {...}
```

2. **From image.rs to core/state.rs**
```rust
lazy_static! {
    static ref WORKER_POOL: Arc<Mutex<Option<WorkerPool>>> = ...;
}

pub struct AppState {
    worker_pool: Arc<Mutex<Option<WorkerPool>>>,
    config: Arc<AppConfig>,
    metrics: Arc<Mutex<MetricsCollector>>,
}
```

3. **From worker_pool.rs to worker/pool.rs**
```rust
pub struct WorkerPool {
    manager: Arc<WorkerManager>,
    metrics: Arc<WorkerMetrics>,
    config: WorkerConfig,
}

impl WorkerPool {
    pub fn new(size: usize, app: tauri::AppHandle) -> Self {...}
    pub async fn process(&self, task: ImageTask) -> Result<...> {...}
    pub async fn process_batch(&self, tasks: Vec<ImageTask>) -> Result<...> {...}
}
```

4. **From worker_pool.rs to worker/manager.rs**
```rust
pub struct Worker {
    id: usize,
    handle: tokio::task::JoinHandle<()>,
}

pub struct WorkerManager {
    workers: Vec<Worker>,
    task_sender: Sender<ImageTask>,
    result_receiver: Receiver<OptimizationResult>,
}
```

5. **From progress_debouncer.rs to progress/debouncer.rs**
```rust
pub struct ProgressDebouncer {
    config: DebouncerConfig,
    channel: ProgressChannel,
}

impl ProgressDebouncer {
    pub fn new(config: Option<DebouncerConfig>) -> Self {...}
    pub fn start<F>(&self, emit_fn: F) {...}
}
```

6. **From worker_pool.rs to processing/optimizer.rs**
```rust
pub struct ImageOptimizer {
    config: OptimizerConfig,
    sidecar: Arc<SharpSidecar>,
}

impl ImageOptimizer {
    async fn process_image(app: &tauri::AppHandle, task: ImageTask) 
        -> Result<OptimizationResult, String> {...}
}
```

### Updated lib.rs Structure
```rust
// Module declarations
mod commands;
mod core;
mod worker;
mod progress;
mod processing;
mod utils;

// Public exports
pub use commands::*;
pub use core::{AppState, ImageSettings, OptimizationResult};
pub use worker::{WorkerPool, WorkerMetrics};
pub use progress::ProgressTracker;
pub use processing::ImageProcessor;
```

### Debug Points
- [✓] Module boundary validation
- [✓] Interface completeness
- [✓] Error propagation paths
- [✓] Resource cleanup
- [✓] State synchronization
- [✓] Performance impact

### Expected Outcomes
1. **Maintainability** 📈
   - Reduced file sizes (< 200 lines per file)
   - Clear module boundaries
   - Better code organization

2. **Flexibility** 🔄
   - Pluggable components
   - Easier feature additions
   - Better dependency management

3. **Performance** ⚡
   - Optimized resource usage
   - Better state management
   - Reduced lock contention
   - Improved error recovery

### Implementation Plan
1. Create new module structure
2. Migrate core types and interfaces
3. Implement new modules incrementally
4. Document new architecture

### Migration & Cleanup Strategy 🧹

1. **Phase 1: Preparation**
   ```bash
   # Create new directory structure
   src/
   ├── core/
   ├── worker/
   ├── progress/
   └── processing/
   ```

2. **Phase 2: File Migration** 📦
   - Migrate files in this order:
     1. Core types and interfaces (no dependencies)
     2. Progress tracking (depends on core)
     3. Worker management (depends on core)
     4. Processing pipeline (depends on all)
     5. Command layer (final integration)

3. **Phase 3: Import Updates** 🔄
   ```rust
   // Old imports to replace
   use crate::worker_pool::{WorkerPool, WorkerMetrics};
   use crate::progress_debouncer::ProgressDebouncer;

   // New modular imports
   use crate::core::{AppState, ImageSettings};
   use crate::worker::{WorkerPool, WorkerMetrics};
   use crate::progress::ProgressTracker;
   use crate::processing::ImageProcessor;
   ```

4. **Phase 4: Code Removal** 🗑️
   Checklist for safe removal:
   - [ ] Verify new implementation works
   - [ ] Check all imports are updated
   - [ ] Ensure no remaining references
   - [ ] Remove old files:
     ```bash
     rm src/worker_pool.rs
     rm src/progress_debouncer.rs
     rm src/commands/image.rs  # After migration
     ```

### Migration Checklist

#### Core Types Migration
- [ ] Move `ImageSettings` to `core/types.rs`
- [ ] Move `OptimizationResult` to `core/types.rs`
- [ ] Create `core/state.rs`
- [ ] Update all imports

#### Worker Management Migration
- [ ] Create `worker/pool.rs`
- [ ] Create `worker/manager.rs`
- [ ] Create `worker/metrics.rs`
- [ ] Move worker logic
- [ ] Update state management

#### Progress Tracking Migration
- [ ] Create `progress/tracker.rs`
- [ ] Create `progress/debouncer.rs`
- [ ] Move progress logic
- [ ] Update event emission

#### Processing Pipeline Migration
- [ ] Create `processing/optimizer.rs`
- [ ] Create `processing/validation.rs`
- [ ] Move processing logic
- [ ] Update sidecar integration

#### Command Layer Migration
- [ ] Create new command modules
- [ ] Update command handlers
- [ ] Remove old command files

#### Final Cleanup
- [ ] Remove old files
- [ ] Update main.rs imports
- [ ] Update lib.rs exports
- [ ] Verify no dead code