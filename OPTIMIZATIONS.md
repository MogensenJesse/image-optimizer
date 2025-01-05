# Performance Optimization Roadmap

## Prerequisites Per Section

## 1. Parallel Processing Implementation 🔄
### Required Dependencies
```toml
# Add to Cargo.toml
[dependencies]
tokio = { version = "1.42.0", features = ["full"] }
futures = "0.3.31"
num_cpus = "1.16.0"
crossbeam-channel = "0.5.14"
```

### Required Types
```rust
pub struct ImageTask {
    pub input_path: String,
    pub output_path: String,
    pub settings: ImageSettings,
    pub priority: u8,
}

pub struct WorkerPool {
    workers: Vec<Worker>,
    task_sender: Sender<ImageTask>,
    result_receiver: Receiver<OptimizationResult>,
    active_tasks: Arc<Mutex<usize>>,
}
```

- [✅] Add worker pool in Rust backend
  - [✅] Worker pool implementation
  - [✅] Task distribution system
  - [✅] Process image function
  - [✅] Error handling
- [✅] Implement batch processing
  - [✅] Queue system for multiple tasks
  - [✅] Parallel task execution
  - [✅] Result collection
- [✅] Add progress tracking per batch
  - [✅] Add ProcessingProgress struct
  - [✅] Implement progress callbacks
  - [✅] Track elapsed time
  - [✅] Track bytes processed/saved
  - [✅] Track active workers
  - [✅] Frontend progress display
- [ ] Debug points:
  - [ ] CPU usage monitoring
  - [ ] Memory usage per worker
  - [ ] Batch processing timing

## 2. Sharp Sidecar Optimization 🖼️
### Required Dependencies
```json
{
  "dependencies": {
    "sharp": "^0.33.5",
    "@img/sharp-win32-x64": "latest",
    "node-stream-zip": "^1.15.0"  // For streaming support
  }
}
```

- [ ] Implement Sharp memory limits
  ```javascript
  const sharp = require('sharp');
  // Limit cache memory to prevent memory bloat
  sharp.cache({ items: 200, files: 50 });
  // Adjust concurrency based on system
  sharp.concurrency(Math.max(1, Math.min(4, os.cpus().length - 1)));
  ```

- [ ] Enhance existing format optimizations
  - [ ] Add dynamic quality adjustment based on image content
  - [ ] Add intelligent color palette optimization for PNG
  ```javascript
  // Example of dynamic quality adjustment
  const metadata = await sharp(input).metadata();
  const quality = metadata.hasAlpha ? 
    Math.min(settings.quality, 92) : // Preserve alpha quality
    settings.quality;                // Use standard quality
  ```

- [ ] Implement streaming for large files
  ```javascript
  const transformer = sharp()
    .on('info', info => console.error('Processing:', info))
    .on('error', err => console.error('Error:', err));
  fs.createReadStream(input)
    .pipe(transformer)
    .pipe(fs.createWriteStream(output));
  ```

- [ ] Debug points:
  - Memory usage monitoring per format
  - Processing time tracking
  - Quality/size ratio analysis
  ```javascript
  const startTime = process.hrtime.bigint();
  // ... processing ...
  const endTime = process.hrtime.bigint();
  console.error('Processing time:', Number(endTime - startTime) / 1e6, 'ms');
  ```

### Notes on Existing Optimizations:
- Current format-specific settings are well-tuned:
  - JPEG: mozjpeg + chroma subsampling
  - PNG: Adaptive filtering + palette optimization
  - WebP: Balanced quality/compression
  - AVIF: Conservative effort for speed
  - TIFF: Optimized tile settings
- Lossless mode already implements optimal settings per format
- Quality settings properly cascade from global → format-specific

## 3. Rust Backend Optimizations ⚡
### Required Dependencies
```toml
[dependencies]
lru = "0.12"  # For caching
dashmap = "5.5"  # For concurrent caching
bytes = "1.5"  # For buffer optimization
```

### Required Types
```rust
pub struct OptimizationCache {
    settings_cache: dashmap::DashMap<String, ImageSettings>,
    result_cache: lru::LruCache<String, OptimizationResult>,
    path_cache: dashmap::DashMap<String, PathBuf>,
}
```

- [ ] Implement command queuing
- [ ] Add caching system
- [ ] Optimize IPC serialization
- [ ] Debug points

## 4. Frontend Optimizations 🎨
### Required Dependencies
```json
{
  "dependencies": {
    "@tanstack/react-virtual": "^3.0.0",  // Modern virtualization
    "lodash.debounce": "^4.0.8",
    "use-debounce": "^10.0.0"
  }
}
```

### Required Types
```javascript
// types.js
export type OptimizationResult = {
  inputPath: string;
  outputPath: string;
  originalSize: number;
  optimizedSize: number;
  compressionRatio: number;
  elapsedTime: number;
};
```

- [ ] Implement virtualization
- [ ] Add batch UI updates
- [ ] Optimize React renders
- [ ] Debug points

## 5. File System Operations 📁
### Required Dependencies
```toml
[dependencies]
memmap2 = "0.9"  # For memory mapping large files
cached = "0.46"  # For function result caching
walkdir = "2.4"  # For efficient directory traversal
```

### Required Types
```rust
pub struct FileOperation {
    buffer_size: usize,
    use_mmap: bool,
    cache_enabled: bool,
}

pub struct DirCache {
    entries: lru::LruCache<PathBuf, Vec<DirEntry>>,
    max_size: usize,
}
```

- [ ] Implement directory caching
- [ ] Add buffer optimization
- [ ] Optimize path operations
- [ ] Debug points

## 6. Memory Management 🧮
### Required Dependencies
```toml
[dependencies]
jemalloc-ctl = "0.5"  # For memory allocation control
metrics = "0.21"      # For memory metrics
```

### Required Configuration
```rust
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;
```

- [ ] Implement memory limits
- [ ] Add garbage collection triggers
- [ ] Optimize buffer management
- [ ] Debug points

## 7. Error Handling & Recovery 🔧
### Required Dependencies
```toml
[dependencies]
thiserror = "1.0"   # For error handling
backoff = "0.4"     # For retry mechanisms
tracing = "0.1"     # For error tracking
```

### Required Types
```rust
#[derive(thiserror::Error, Debug)]
pub enum OptimizationError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Processing error: {0}")]
    Processing(String),
    #[error("Validation error: {0}")]
    Validation(String),
}
```

- [ ] Add retry mechanisms
- [ ] Implement fallback strategies
- [ ] Add corruption detection
- [ ] Debug points

## Testing & Verification Tools 🧪
### Required Dependencies
```toml
[dev-dependencies]
criterion = "0.5"     # For benchmarking
test-case = "3.3"    # For parametrized testing
mockall = "0.12"     # For mocking
iai = "0.1"          # For allocation/instruction counting
```

### Required Configuration
```toml
[[bench]]
name = "optimization_benchmark"
harness = false
```

- [ ] Performance benchmarks
- [ ] Memory profiling
- [ ] CPU profiling 