# Image Optimizer - Technical Documentation

## 1. Project Architecture & Workflow

### 1.1 Frontend (React)
# Core Components

1. **App.jsx** - Main Application Container
   - Manages global state and event handling
   - Implements drag & drop interface
   - Coordinates image processing workflow
   ```javascript
   function App() {
     const [isProcessing, setIsProcessing] = useState(false);
     const [isDragging, setIsDragging] = useState(false);
     const [optimizationStats, setOptimizationStats] = useState({
       totalFiles: 0,
       processedFiles: 0,
       elapsedTime: 0,
       currentFile: '',
       bytesProcessed: 0,
       bytesSaved: 0,
       estimatedTimeRemaining: 0,
       activeWorkers: 0
     });
     const [optimizationResults, setOptimizationResults] = useState([]);
     const processingRef = useRef(false);
     const [settings, setSettings] = useState({
       quality: {
         global: 90,
         jpeg: null,
         png: null,
         webp: null,
         avif: null
       },
       resize: {
         width: null,
         height: null,
         maintainAspect: true,
         mode: 'none',
         size: null
       },
       outputFormat: 'original'
     });
   }
   ```

2. **FloatingMenu.jsx** - Settings Interface
   - Quality control
   - Resize options
   - Format selection
   - Advanced settings panel

3. **CpuMetrics.jsx** - Performance Monitoring ✨
   - Real-time CPU usage tracking
   - Worker performance metrics
   - Task processing statistics
   ```javascript
   function CpuMetrics() {
     const [metrics, setMetrics] = useState([]);
     // Updates metrics every second
     useEffect(() => {
       const interval = setInterval(fetchMetrics, 1000);
       return () => clearInterval(interval);
     }, []);
   }
   ```

# State Management

1. **Processing State**
   ```javascript
   const [isProcessing, setIsProcessing] = useState(false);
   const [isDragging, setIsDragging] = useState(false);
   const processingRef = useRef(false); // Prevents concurrent processing
   ```

2. **Optimization Settings**
   ```javascript
   const [settings, setSettings] = useState({
     quality: {
       global: 90,
       jpeg: null,
       png: null,
       webp: null,
       avif: null
     },
     resize: {
       width: null,
       height: null,
       maintainAspect: true,
       mode: 'none',
       size: null
     },
     outputFormat: 'original'
   });
   ```

3. **Progress Tracking** ✨
   ```javascript
   const [optimizationStats, setOptimizationStats] = useState({
     totalFiles: 0,
     processedFiles: 0,
     elapsedTime: 0,
     currentFile: '',
     bytesProcessed: 0,
     bytesSaved: 0,
     estimatedTimeRemaining: 0,
     activeWorkers: 0
   });
   ```

# Event Handling & Data Flow

1. **Progress Events** ✨
   ```javascript
   useEffect(() => {
     const unsubscribeProgress = listen("optimization_progress", (event) => {
       const progress = event.payload;
       setOptimizationStats({
         totalFiles: progress.total_files,
         processedFiles: progress.processed_files,
         currentFile: progress.current_file,
         elapsedTime: progress.elapsed_time,
         bytesProcessed: progress.bytes_processed,
         bytesSaved: progress.bytes_saved,
         estimatedTimeRemaining: progress.estimated_time_remaining,
         activeWorkers: progress.active_workers
       });
     });
   }, []);
   ```

2. **Batch Processing Pipeline** ✨
   ```javascript
   // Create all required directories first
   await Promise.all(paths.map(async (path) => {
     const parentDir = await dirname(path);
     const optimizedPath = await join(parentDir, 'optimized');
     await mkdir(optimizedPath, { recursive: true });
   }));

   // Create batch tasks
   const tasks = await Promise.all(paths.map(async (path) => {
     const parentDir = await dirname(path);
     const fileName = path.split('\\').pop();
     const optimizedPath = await join(parentDir, 'optimized', fileName);
     return [path, optimizedPath, settings];
   }));

   // Process batch
   const results = await invoke('optimize_images', { tasks });
   ```

### 1.2 Backend (Tauri/Rust)
# Worker Pool System ✨

1. **Dynamic Worker Management**
   ```rust
   pub struct WorkerPool {
       workers: Vec<Worker>,
       task_sender: Sender<ImageTask>,
       result_receiver: Receiver<OptimizationResult>,
       active_tasks: Arc<Mutex<usize>>,
       metrics: Arc<Mutex<Vec<WorkerMetrics>>>,
       sys: Arc<Mutex<System>>,
   }
   ```

2. **Adaptive Buffer Sizing**
   ```rust
   let buffer_size = if total_memory_gb == 0 || total_memory_gb > 1024 {
       size * 2
   } else {
       let base_size = if total_memory_gb < 8 {
           size * 2
       } else if total_memory_gb < 16 {
           size * 3
       } else {
           size * 4
       };
       // Adjust based on CPU frequency
       if avg_freq > 3000 { base_size + size } else { base_size }
   };
   ```

3. **Performance Monitoring**
   ```rust
   pub struct WorkerMetrics {
       pub cpu_usage: f64,
       pub thread_id: usize,
       pub task_count: usize,
       pub avg_processing_time: f64,
   }
   ```

# Task Processing ✨

1. **Batch Processing with Backpressure**
   ```rust
   while self.task_sender.is_full() {
       println!("Channel full, waiting for space...");
       tokio::time::sleep(std::time::Duration::from_millis(100)).await;
       // Collect results while waiting
       match self.result_receiver.try_recv() {
           Ok(result) => {
               processed += 1;
               // ... handle result
           }
           Err(crossbeam_channel::TryRecvError::Empty) => (),
           Err(e) => eprintln!("Error receiving result while waiting: {}", e),
       }
   }
   ```

2. **Adaptive Worker Cooldown**
   ```rust
   let cooldown = if processing_time > 2.5 {
       20
   } else if processing_time > 2.0 {
       15
   } else {
       10
   };
   tokio::time::sleep(std::time::Duration::from_millis(cooldown)).await;
   ```

3. **CPU Load Management**
   ```rust
   if cpu_usage > 90.0 && consecutive_long_tasks > 2 {
       println!("Worker {} taking a brief break due to high CPU usage", id);
       tokio::time::sleep(std::time::Duration::from_millis(50)).await;
       consecutive_long_tasks = 0;
   }
   ```

### 1.3 Node.js Sidecar (Sharp)
# Image Processing Logic

1. **Format-Specific Optimizations**
   ```javascript
   const getLosslessSettings = (format) => {
     switch (format) {
       case 'jpeg':
         return {
           quality: 100,
           mozjpeg: true,
           chromaSubsampling: '4:4:4',
           optimiseCoding: true
         };
       case 'png':
         return {
           compressionLevel: 9,
           palette: false,
           quality: 100,
           effort: 10,
           adaptiveFiltering: true,
         };
       // ... other formats
     }
   };
   ```

2. **Resize Operations**
   ```javascript
   switch (settings.resize.mode) {
     case 'width':
       image = image.resize(size, null, { 
         withoutEnlargement: true,
         fit: 'inside'
       });
       break;
     case 'height':
       image = image.resize(null, size, { 
         withoutEnlargement: true,
         fit: 'inside'
       });
       break;
     // ... other modes
   }
   ```

3. **Quality Control**
   ```javascript
   let formatOptions;
   if (settings?.quality?.global === 100) {
     formatOptions = getLosslessSettings(outputFormat);
   } else {
     formatOptions = { ...optimizationDefaults[outputFormat] };
     if (settings?.quality) {
       if (settings.quality[outputFormat] !== null) {
         formatOptions.quality = settings.quality[outputFormat];
       } else if (settings.quality.global !== null) {
         formatOptions.quality = settings.quality.global;
       }
     }
   }
   ```

# Sharp Configuration

1. Default Optimization Settings:
   - Format-specific presets (JPEG, PNG, WebP, AVIF, TIFF)
   - Quality and compression parameters
   - Advanced format-specific options (e.g., mozjpeg, chromaSubsampling)

2. Lossless Mode:
   - Activated when quality is set to 100
   - Format-specific lossless configurations
   - Maximum quality preservation settings

# Error Handling

- Input validation and format checking
- Detailed error logging to stderr
- Process exit codes for error states
- Error propagation back to Rust backend

# Build Process

The sidecar is compiled into a standalone executable using @yao-pkg/pkg:
- Bundles all dependencies
- Platform-specific binaries
- Automatic renaming based on target triple
- Assets inclusion (Sharp binaries, defaults)

### 1.4 Build Process
# Development Build Flow

When running `npm run tauri dev`, the following process occurs:

1. The `tauri` script in root package.json first runs `build:sharp`:
   ```json
   "scripts": {
     "build:sharp": "cd sharp-sidecar && npm run build:rename",
     "tauri": "npm run build:sharp && tauri"
   }
   ```

2. Inside sharp-sidecar directory:
   - `build:rename` script executes: `npm run build && node rename.js`
   - `build` uses @yao-pkg/pkg to create standalone executable:
     ```json
     "pkg": {
       "assets": [
         "node_modules/sharp/**/*",
         "node_modules/@img/sharp-win32-x64/**/*",
         "optimizationDefaults.js"
       ],
       "targets": ["node20-win-x64"]
     }
     ```

3. The rename script:
   - Detects platform (adds .exe extension on Windows)
   - Gets Rust target triple using `rustc -vV`
   - Moves executable to Tauri's binary directory:
     `sharp-sidecar.exe → src-tauri/binaries/sharp-sidecar-{target-triple}.exe`

4. Tauri configuration includes the binary:
   ```json
   "bundle": {
     "externalBin": [
       "binaries/sharp-sidecar"
     ]
   }
   ```

This process ensures the Sharp sidecar is:
- Compiled as a standalone executable
- Named correctly for the target platform
- Placed where Tauri can access it
- Bundled with the final application

### 1.5 Inter-Process Communication
# Frontend → Backend Flow

1. **Tauri Commands**
   ```javascript
   const result = await invoke('optimize_image', { 
     inputPath: path, 
     outputPath: optimizedPath,
     settings: settings
   });
   ```

2. **Event Listeners**
   ```javascript
   useEffect(() => {
     const unsubscribeDrop = listen("tauri://drag-drop", async (event) => {
       if (processingRef.current) return;
       processingRef.current = true;
       // Process dropped files...
     });
   }, [settings]);
   ```

# Backend → Sidecar Flow

1. **Command Execution**
   ```rust
   let command = app
     .shell()
     .sidecar("sharp-sidecar")
     .args(&[
       "optimize",
       &input_path,
       &output_path,
       &settings_json
     ]);
   ```

2. **Result Handling**
   ```rust
   if output.status.success() {
     let result: OptimizationResult = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
     Ok(result)
   } else {
     Err(String::from_utf8_lossy(&output.stderr).to_string())
   }
   ```

# Data Serialization

1. **Frontend → Backend**
   - Settings serialized as JSON
   - File paths passed as strings
   - Progress updates via state management

2. **Backend → Sidecar**
   - Settings re-serialized for CLI
   - File paths validated and normalized
   - Results parsed from stdout JSON

# Error Handling

1. **Frontend Layer**
   ```javascript
   try {
     const result = await invoke('optimize_image', {...});
   } catch (error) {
     console.error(`Error processing ${path}:`, error);
   }
   ```

2. **Backend Layer**
   ```rust
   pub async fn optimize_image(...) -> Result<OptimizationResult, String> {
     // Error handling with ? operator
     let output = command.output().await.map_err(|e| e.to_string())?;
   }
   ```

3. **Sidecar Layer**
   - Detailed error messages to stderr
   - Process exit codes for different errors
   - Error propagation through layers

# Security Considerations

1. **Command Validation**
   ```json
   {
     "identifier": "shell:allow-execute",
     "allow": [{
       "name": "binaries/sharp-sidecar",
       "sidecar": true,
       "args": [
         "optimize",
         {"validator": "\\S+"},
         {"validator": "\\S+"},
         {"validator": ".*"}
       ]
     }]
   }
   ```

2. **File System Access**
   - Restricted to user-selected directories
   - Output paths validated
   - Permissions checked before operations


## 2. External Documentation & References

### 2.1 Core Technologies

1. **Tauri v2**
   - [Core Concepts](https://v2.tauri.app/concepts/architecture)
   - [Security Model](https://v2.tauri.app/concepts/security)
   - [IPC System](https://v2.tauri.app/concepts/ipc)
   - [Capabilities System](https://v2.tauri.app/concepts/capabilities)

2. **Sharp**
   - [API Documentation](https://sharp.pixelplumbing.com/)
   - [Image Formats](https://sharp.pixelplumbing.com/api-output)
   - [Performance Guide](https://sharp.pixelplumbing.com/performance)

### 2.2 Build Tools

1. **pkg**
   - [@yao-pkg/pkg Documentation](https://github.com/vercel/pkg)
   - [Asset Management](https://github.com/vercel/pkg#detecting-assets-in-source-code)
   - [Binary Compilation](https://github.com/vercel/pkg#targets)

2. **Vite**
   - [Configuration Reference](https://vitejs.dev/config/)
   - [Tauri Integration](https://v2.tauri.app/guides/getting-started/setup/vite)

### 2.3 Development Resources

1. **Rust Crates**
   ```toml
   [dependencies]
   tauri = { version = "2", features = [] }
   tauri-plugin-shell = "2"
   tauri-plugin-fs = "2"
   tauri-plugin-dialog = "2"
   tauri-plugin-process = "2"
   tauri-plugin-opener = "2"
   ```
   - [tauri-apps/plugins-workspace](https://github.com/tauri-apps/plugins-workspace)
   - [Plugin Documentation](https://v2.tauri.app/plugins)

2. **React Libraries**
   ```json
   {
     "dependencies": {
       "@tauri-apps/api": "^2",
       "react-dropzone": "^14.3.5"
     }
   }
   ```

### 2.4 Security Guidelines

1. **Tauri Security Best Practices**
   - [Capability-based Security](https://v2.tauri.app/concepts/security/capabilities)
   - [Process Isolation](https://v2.tauri.app/concepts/security/processes)
   - [Asset Handling](https://v2.tauri.app/concepts/security/assets)

2. **File System Security**
   - [Scope-limited Access](https://v2.tauri.app/concepts/security/fs)
   - [Path Validation](https://v2.tauri.app/concepts/security/fs-scope) 