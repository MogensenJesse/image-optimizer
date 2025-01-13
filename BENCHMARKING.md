## Image Optimizer Benchmarking System

### Overview
A simple yet comprehensive benchmarking system to measure performance across all layers of the application.

### Implementation Plan [🔄]

#### 1. Backend Metrics Collection []
```rust
use std::time::{Duration, Instant};

/// Fixed-size ring buffer for time-series data
pub struct RingBuffer<T> {
    data: Vec<T>,
    capacity: usize,
    position: usize,
}

/// Processing stages for accurate timing
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ProcessingStage {
    Validation,
    WorkerInit,
    ImageRead,
    Optimization,
    ImageWrite,
    Cleanup,
}

/// Statistical metrics for accurate timing analysis
pub struct TimingStats {
    min: Duration,
    max: Duration,
    sum: Duration,
    count: u32,
    variance: f64,
}

/// Image formats supported
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ImageFormat {
    JPEG,
    PNG,
    WebP,
    AVIF,
}

/// Format conversion metrics
pub struct FormatConversion {
    from: ImageFormat,
    to: ImageFormat,
    size_reduction: u64,
    quality_retained: f64,
}

/// Resource utilization snapshot
pub struct ResourceSnapshot {
    timestamp: Instant,
    cpu_usage: f64,
    memory_usage: u64,
    io_operations: u64,
}

/// Core benchmarking metrics structure
pub struct BenchmarkMetrics {
    // Time-based metrics with statistical accuracy
    start_time: Instant,
    stage_timings: EnumMap<ProcessingStage, TimingStats>,
    
    // Resource metrics with efficient storage
    resource_samples: RingBuffer<ResourceSnapshot>,
    worker_stats: Vec<WorkerStats>,
    
    // Optimization metrics with type safety
    format_metrics: EnumMap<ImageFormat, FormatMetrics>,
    conversions: Vec<FormatConversion>,
    
    // Overall statistics
    total_bytes_processed: u64,
    total_bytes_saved: u64,
    total_images: u32,
}

/// Per-worker statistics
pub struct WorkerStats {
    id: u32,
    tasks_completed: u32,
    avg_processing_time: Duration,
    cpu_utilization: f64,
}

/// Format-specific metrics
pub struct FormatMetrics {
    compression_ratio: f64,
    quality_scores: Vec<(u32, f64)>, // (quality, size_ratio)
    processing_time: TimingStats,
}

impl BenchmarkMetrics {
    /// Creates a new benchmark metrics instance
    pub fn new(buffer_capacity: usize) -> Self {
        Self {
            start_time: Instant::now(),
            stage_timings: EnumMap::new(),
            resource_samples: RingBuffer::new(buffer_capacity),
            worker_stats: Vec::new(),
            format_metrics: EnumMap::new(),
            conversions: Vec::new(),
            total_bytes_processed: 0,
            total_bytes_saved: 0,
            total_images: 0,
        }
    }

    /// Updates timing statistics for a processing stage
    pub fn record_stage_timing(&mut self, stage: ProcessingStage, duration: Duration) {
        if let Some(stats) = self.stage_timings.get_mut(stage) {
            stats.update(duration);
        }
    }

    /// Records a resource utilization snapshot
    pub fn record_resource_snapshot(&mut self, cpu: f64, memory: u64, io: u64) {
        self.resource_samples.push(ResourceSnapshot {
            timestamp: Instant::now(),
            cpu_usage: cpu,
            memory_usage: memory,
            io_operations: io,
        });
    }

    /// Generates a formatted report
    pub fn generate_report(&self) -> String {
        // Implementation for report generation
        // ...
    }
}
```

#### 2. Frontend Integration [ ]
```javascript
const benchmarkState = {
    startTime: null,
    dropToStartDuration: null,
    uiUpdateLatency: [],
    userInteractions: []
};
```

#### 3. Sidecar Integration [ ]
```javascript
const processMetrics = {
    imageProcessingStages: [],
    memoryUsage: [],
    formatSpecificTimings: {}
};
```

### Implementation Steps [🔄]

0. Modular Setup [ ]
   - [ ] Add CLI flag support
     ```json
     // In package.json
     {
       "scripts": {
         "tauri": "tauri",
         "dev": "vite",
         "dev:bench": "cross-env BENCHMARK=true tauri dev"
       }
     }
     ```
   - [ ] Add feature flag in Rust
     ```rust
     // In Cargo.toml
     [features]
     default = []
     benchmark = []

     // In main.rs
     #[cfg(feature = "benchmark")]
     use crate::benchmarking::BenchmarkMetrics;
     ```
   - [ ] Add conditional compilation
     ```rust
     // In lib.rs
     #[cfg(feature = "benchmark")]
     pub mod benchmarking;

     pub fn setup_app(app: &mut App) {
         #[cfg(feature = "benchmark")]
         if std::env::var("BENCHMARK").is_ok() {
             app.manage(BenchmarkMetrics::new(1000));
         }
     }
     ```

1. Backend Implementation [🔄]
   - [ ] Add BenchmarkMetrics struct to core/state.rs
   - [ ] Add conditional metrics collection
     ```rust
     impl WorkerPool {
         pub fn record_metrics(&mut self, task: &ImageTask) {
             #[cfg(feature = "benchmark")]
             if let Some(metrics) = self.metrics.as_mut() {
                 metrics.record_stage_timing(/* ... */);
             }
         }
     }
     ```
   - [ ] Add RingBuffer implementation
     ```rust
     impl<T> RingBuffer<T> {
         pub fn new(capacity: usize) -> Self
         pub fn push(&mut self, item: T)
         pub fn get_samples(&self) -> &[T]
     }
     ```
   - [ ] Add TimingStats implementation
     ```rust
     impl TimingStats {
         pub fn new() -> Self
         pub fn update(&mut self, duration: Duration)
         pub fn get_average(&self) -> Duration
     }
     ```
   - [ ] Integrate with worker pool
     ```rust
     // In worker/pool.rs
     impl WorkerPool {
         pub fn record_metrics(&mut self, task: &ImageTask)
     }
     ```

2. Frontend Integration [🔄]
   - [ ] Add conditional benchmarking
     ```javascript
     // In App.jsx
     const isBenchmarking = await invoke('is_benchmarking');
     const recordMetric = (event, data) => {
       if (!isBenchmarking) return;
       benchmarkState.userInteractions.push({
         event,
         timestamp: Date.now(),
         data
       });
     };
     ```
   - [ ] Add Tauri commands
     ```javascript
     // Commands to expose
     'start_benchmark'
     'end_benchmark'
     'get_benchmark_report'
     ```

3. Sidecar Enhancement [🔄]
   - [ ] Add conditional instrumentation
     ```javascript
     // In sharp-sidecar/index.js
     const shouldBenchmark = process.env.BENCHMARK === 'true';
     const timeOperation = async (name, operation) => {
       if (!shouldBenchmark) return operation();
       const start = process.hrtime.bigint();
       const result = await operation();
       const end = process.hrtime.bigint();
       processMetrics.imageProcessingStages.push({
         name,
         duration: Number(end - start) / 1e6 // Convert to ms
       });
       return result;
     };
     ```
   - [ ] Track memory usage
     ```javascript
     const recordMemoryUsage = () => {
       const usage = process.memoryUsage();
       processMetrics.memoryUsage.push({
         timestamp: Date.now(),
         heap: usage.heapUsed,
         total: usage.rss
       });
     };
     ```

4. Report Generation [🔄]
   - [ ] Add conditional command registration
     ```rust
     #[cfg(feature = "benchmark")]
     #[tauri::command]
     async fn get_benchmark_report(
         state: State<'_, AppState>,
     ) -> Result<String, String> {
         state.metrics.generate_report()
     }

     fn main() {
         tauri::Builder::default()
             .setup(|app| {
                 #[cfg(feature = "benchmark")]
                 if std::env::var("BENCHMARK").is_ok() {
                     // Register benchmark commands
                 }
                 Ok(())
             })
     }
     ```
   - [ ] Implement report formatting
     ```rust
     impl BenchmarkMetrics {
         pub fn format_duration(d: Duration) -> String {
             format!("{:.2}s", d.as_secs_f64())
         }
         
         pub fn generate_report(&self) -> String {
             // Format sections:
             // - Time-based metrics
             // - Resource metrics
             // - Optimization metrics
         }
     }
     ```

### Integration Order:
1. Modular setup and feature flags
2. Backend metrics collection
3. Report generation
4. Frontend hooks
5. Sidecar instrumentation

### Usage

1. Run with benchmarking:
```bash
npm run dev:bench
# or
npm run tauri dev -- --features benchmark
```

2. Enable benchmarking in code:
```rust
// Only compiled when benchmark feature is enabled
#[cfg(feature = "benchmark")]
let metrics = BenchmarkMetrics::new(1000);
```

### Sample Report Format
```
=== Image Optimizer Benchmark Report ===

Time-based Metrics:
- Total Duration: 12.34s
- Average Processing Time: 0.45s/image
- Worker Init Time: 0.12s
- Stage Timings:
  ├── Validation: 0.02s
  ├── Processing: 10.45s
  └── I/O: 1.87s

Resource Metrics:
- CPU Utilization: 78.5% (avg)
  ├── Peak: 92.3%
  └── Min: 45.2%
- Worker Efficiency: 92.3%
  ├── Active Time: 11.45s
  └── Idle Time: 0.89s
- I/O Operations:
  ├── Read: 0.89s (45.6 MB)
  └── Write: 0.98s (32.1 MB)

Optimization Metrics:
- Compression Ratios:
  ├── JPEG: 68% (original → optimized)
  ├── PNG: 72% (original → optimized)
  └── WebP: 65% (original → optimized)
- Size Reductions:
  ├── Total: 45.6 MB → 32.1 MB
  └── Average: 2.3 MB/image
- Format Conversions:
  ├── PNG → WebP: 75% size reduction
  └── JPEG → WebP: 70% size reduction
- Quality Metrics:
  ├── JPEG (q=85): 68% reduction
  ├── WebP (q=80): 72% reduction
  └── AVIF (q=75): 75% reduction

Memory Usage:
- Peak: 256 MB
- Average: 145 MB
- Baseline: 85 MB
```

### Notes

- Benchmarking adds minimal overhead to normal operations
- Metrics are collected asynchronously where possible
- Memory usage is sampled rather than continuously monitored
- Report generation happens after processing completes
- Zero overhead when benchmarking is disabled
- Conditional compilation ensures no runtime cost in production
