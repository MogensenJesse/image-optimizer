# Image Optimizer - Technical Documentation

## 1. Project Architecture & Workflow
The Image Optimizer is built with a modern, multi-layered architecture that combines React for the frontend, Rust for performance-critical operations, and Node.js for image processing. This design ensures both high performance and excellent user experience.

### 1.1 Frontend (React)
The frontend layer provides an intuitive user interface for image optimization, featuring drag-and-drop functionality, real-time progress tracking, and advanced settings controls.

# Core Components
These components form the backbone of the user interface, each serving a specific purpose in the application workflow.

1. **App.jsx** - Main Application Container
   The central component that orchestrates the entire application, managing state and coordinating between user interactions and backend processes.
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
   A floating panel that provides users with fine-grained control over the optimization process.
   - Quality control
   - Resize options
   - Format selection
   - Advanced settings panel

3. **CpuMetrics.jsx** - Performance Monitoring ✨
   Real-time monitoring component that helps users understand system resource usage during optimization.
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
The application uses React's state management to handle various aspects of the optimization process, from user settings to progress tracking.

1. **Processing State**
   Core state variables that track the current status of image processing operations.
   ```javascript
   const [isProcessing, setIsProcessing] = useState(false);
   const [isDragging, setIsDragging] = useState(false);
   const processingRef = useRef(false); // Prevents concurrent processing
   ```

2. **Optimization Settings**
   Comprehensive settings configuration that allows fine-tuned control over the optimization process.
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
   Real-time tracking of optimization progress, providing users with detailed feedback about the process.
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
The event system manages user interactions and provides real-time feedback during the optimization process.

1. **Progress Events** ✨
   Real-time progress updates that keep users informed about the current state of optimization.
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
   Efficient handling of multiple images, including directory creation and task distribution.
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


### 1.3 Node.js Sidecar (Sharp)
The Sharp sidecar provides professional-grade image processing capabilities through a well-optimized Node.js implementation.

# Image Processing Logic
Advanced image processing algorithms that ensure optimal quality and compression.

1. **Format-Specific Optimizations**
   Tailored optimization settings for each supported image format.
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
   Intelligent image resizing with multiple modes and aspect ratio preservation.
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
   Fine-grained quality management with format-specific optimizations.
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
Optimized Sharp.js configuration for maximum performance and quality.

1. Default Optimization Settings:
   Pre-configured optimization parameters for common use cases.
   - Format-specific presets (JPEG, PNG, WebP, AVIF, TIFF)
   - Quality and compression parameters
   - Advanced format-specific options (e.g., mozjpeg, chromaSubsampling)

2. Lossless Mode:
   Special handling for cases where maximum quality is required.
   - Activated when quality is set to 100
   - Format-specific lossless configurations
   - Maximum quality preservation settings

# Error Handling
Comprehensive error management system for reliable operation.
- Input validation and format checking
- Detailed error logging to stderr
- Process exit codes for error states
- Error propagation back to Rust backend

# Build Process
The build system ensures reliable packaging and distribution of the application across different platforms.

The sidecar is compiled into a standalone executable using @yao-pkg/pkg:
- Bundles all dependencies
- Platform-specific binaries
- Automatic renaming based on target triple
- Assets inclusion (Sharp binaries, defaults)

### 1.4 Build Process
The build process is designed to create a seamless, platform-specific executable that bundles all necessary components.

# Development Build Flow
A carefully orchestrated build sequence that ensures all components are properly compiled and packaged.

When running `npm run tauri dev`, the following process occurs:

1. The `tauri` script in root package.json first runs `build:sharp`:
   Initial build step that prepares the Sharp sidecar.
   ```json
   "scripts": {
     "build:sharp": "cd sharp-sidecar && npm run build:rename",
     "tauri": "npm run build:sharp && tauri"
   }
   ```

2. Inside sharp-sidecar directory:
   Compilation of the Sharp sidecar into a standalone executable.
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
   Platform-specific binary naming and placement.
   - Detects platform (adds .exe extension on Windows)
   - Gets Rust target triple using `rustc -vV`
   - Moves executable to Tauri's binary directory:
     `sharp-sidecar.exe → src-tauri/binaries/sharp-sidecar-{target-triple}.exe`

4. Tauri configuration includes the binary:
   Integration of the sidecar with the main application.
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

The application uses a multi-layered communication system to coordinate between the React frontend, Rust backend, and Node.js sidecar. This architecture ensures efficient image processing while maintaining a responsive user interface.

# Frontend → Backend Flow
The frontend communicates with the Rust backend through Tauri's IPC system, using both commands for direct actions and event listeners for continuous updates.

1. **Tauri Commands** - Direct function calls from frontend to backend
   ```javascript
   // Single image optimization
   const result = await invoke('optimize_image', { 
     inputPath: path, 
     outputPath: optimizedPath,
     settings: settings
   });

   // Batch optimization
   const results = await invoke('optimize_images', { tasks });

   // Worker metrics
   const metrics = await invoke('get_worker_metrics');
   ```

2. **Event Listeners** - Real-time updates and user interactions
   ```javascript
   // Progress tracking - Updates UI with current processing status
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

   // File drop handling - Processes files dropped into the application
   const unsubscribeDrop = listen("tauri://drag-drop", async (event) => {
     const paths = event.payload.paths;
     // Process dropped files...
   });
   ```

# Backend → Sidecar Flow
The Rust backend manages a pool of workers that process images using the Sharp sidecar. This system ensures optimal resource utilization and prevents system overload.

1. **Command Execution** - Worker pool management and task processing
   ```rust
   #[tauri::command]
   pub async fn optimize_image(
       app: tauri::AppHandle,
       input_path: String,
       output_path: String,
       settings: ImageSettings,
   ) -> Result<OptimizationResult, String> {
       let pool = {
           let mut pool_guard = WORKER_POOL.lock().await;
           if pool_guard.is_none() {
               let num_workers = get_optimal_worker_count();
               *pool_guard = Some(WorkerPool::new(num_workers, app.clone()));
           }
           pool_guard.as_mut().unwrap().clone()
       };

       let task = ImageTask {
           input_path,
           output_path,
           settings,
           priority: 0,
       };

       pool.process(task).await
   }
   ```

2. **Progress Events** - Real-time progress updates to frontend
   ```rust
   pool.process_batch(tasks, move |progress| {
       let _ = app_handle.emit("optimization_progress", progress);
   }).await
   ```

# Data Types
The application uses strongly-typed data structures to ensure type safety and clear communication between layers.

1. **Optimization Settings** - Configuration for image processing
   ```rust
   #[derive(Debug, Serialize, Deserialize, Clone)]
   pub struct ImageSettings {
       quality: QualitySettings,
       resize: ResizeSettings,
       #[serde(rename = "outputFormat")]
       output_format: String
   }

   #[derive(Debug, Serialize, Deserialize, Clone)]
   pub struct QualitySettings {
       global: u32,
       jpeg: Option<u32>,
       png: Option<u32>,
       webp: Option<u32>,
       avif: Option<u32>
   }
   ```

2. **Results and Progress** - Processing outcomes and status updates
   ```rust
   #[derive(Debug, Serialize, Deserialize, Clone)]
   pub struct OptimizationResult {
       pub path: String,
       #[serde(rename = "originalSize")]
       pub original_size: u64,
       #[serde(rename = "optimizedSize")]
       pub optimized_size: u64,
       #[serde(rename = "savedBytes")]
       pub saved_bytes: i64,
       #[serde(rename = "compressionRatio")]
       pub compression_ratio: String,
       pub format: String,
   }
   ```

# Error Handling
The application implements comprehensive error handling across all layers to ensure reliability and user feedback.

1. **Frontend Layer** - User-facing error handling
   ```javascript
   try {
     const result = await invoke('optimize_image', {...});
   } catch (error) {
     console.error(`Error processing ${path}:`, error);
   }
   ```

2. **Backend Layer** - System-level error handling
   ```rust
   pub async fn optimize_image(...) -> Result<OptimizationResult, String> {
     // Error handling with Result type
     pool.process(task).await.map_err(|e| e.to_string())
   }
   ```

3. **Sidecar Layer** - Image processing error handling
   - Error propagation through exit codes
   - Detailed error messages to stderr
   - Result serialization for frontend display

# Security Considerations
The application implements several security measures to protect user data and system resources.

1. **Command Validation** - Ensures safe execution of the sidecar
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

2. **File System Access** - Implements secure file handling
   - Restricted to user-selected directories
   - Output paths validated
   - Permissions checked before operations