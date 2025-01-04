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
     const [optimizationStats, setOptimizationStats] = useState({...});
     const [optimizationResults, setOptimizationResults] = useState([]);
     const [settings, setSettings] = useState({...});
   }
   ```

2. **FloatingMenu.jsx** - Settings Interface
   - Quality control
   - Resize options
   - Format selection
   ```javascript
   function FloatingMenu({ settings, onSettingsChange }) {
     const [isOpen, setIsOpen] = useState(false);
     // Settings controls...
   }
   ```

# State Management

1. **Processing State**
   ```javascript
   const [isProcessing, setIsProcessing] = useState(false);
   const processingRef = useRef(false); // Prevents concurrent processing
   ```

2. **Optimization Settings**
   ```javascript
   const [settings, setSettings] = useState({
     quality: {
       global: 90,
       jpeg: null, png: null, webp: null, avif: null
     },
     resize: {
       mode: 'none',
       size: null,
       maintainAspect: true
     },
     outputFormat: 'original'
   });
   ```

3. **Results Tracking**
   ```javascript
   const [optimizationStats, setOptimizationStats] = useState({
     totalFiles: 0,
     processedFiles: 0,
     elapsedTime: 0
   });
   const [optimizationResults, setOptimizationResults] = useState([]);
   ```

# Event Handling & Data Flow

1. **Drag & Drop Events**
   ```javascript
   useEffect(() => {
     const unsubscribeDrop = listen("tauri://drag-drop", async (event) => {
       // Handle dropped files...
     });
     const unsubscribeEnter = listen("tauri://drag-enter", () => {...});
     const unsubscribeLeave = listen("tauri://drag-leave", () => {...});
   }, [settings]);
   ```

2. **Processing Pipeline**
   - Create output directories
   - Process images sequentially
   - Update progress in real-time
   ```javascript
   await Promise.all(paths.map(async (path) => {
     const parentDir = await dirname(path);
     const optimizedPath = await join(parentDir, 'optimized');
     await mkdir(optimizedPath, { recursive: true });
   }));
   ```

3. **Backend Communication**
   ```javascript
   const result = await invoke('optimize_image', { 
     inputPath: path, 
     outputPath: optimizedPath,
     settings: settings
   });
   ```

# Styling Architecture

1. **Base Styles**
   - `variables.scss`: Global design tokens
   - `reset.scss`: CSS reset and normalization
   - `typography.scss`: Font system

2. **Component Styles**
   - `app.scss`: Main container and dropzone
   - `FloatingMenu.scss`: Settings panel

3. **Design System**
   - Consistent color scheme
   - Responsive layout
   - Dark mode support
   ```scss
   @media (prefers-color-scheme: dark) {
     // Dark mode styles...
   }
   ```

### 1.2 Backend (Tauri/Rust)
# Command Handlers

The Rust backend is structured around command handlers that bridge the frontend and sidecar:

```rust
#[tauri::command]
pub async fn optimize_image(
    app: tauri::AppHandle, 
    input_path: String, 
    output_path: String,
    settings: ImageSettings
) -> Result<OptimizationResult, String>
```

# Data Flow

1. Frontend → Rust:
   - Frontend invokes `optimize_image` command via Tauri IPC
   - Passes input/output paths and settings as serialized JSON
   - Settings are deserialized into Rust structs:
     ```rust
     pub struct ImageSettings {
         quality: QualitySettings,
         resize: ResizeSettings,
         output_format: String
     }
     ```

2. Rust → Sidecar:
   - Rust serializes settings back to JSON
   - Uses `tauri_plugin_shell` to execute sidecar:
     ```rust
     app.shell()
        .sidecar("sharp-sidecar")
        .args(&["optimize", &input_path, &output_path, &settings_json])
     ```

3. Sidecar → Rust:
   - Sidecar processes image and outputs JSON result to stdout
   - Rust deserializes output into `OptimizationResult`
   - Result is returned to frontend

# Security & Permissions

Configured in `capabilities/default.json`:
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

- Restricts sidecar execution to specific command format
- Validates argument patterns
- Enforces filesystem access boundaries

### 1.3 Node.js Sidecar (Sharp)
# Image Processing Logic
The sidecar uses Sharp to handle image optimization with the following workflow:

1. Receives commands via CLI arguments:
   - Command type (e.g., 'optimize')
   - Input path
   - Output path
   - Settings JSON string

2. Processes images with configurable options:
   - Format conversion
   - Quality adjustment
   - Resize operations
   - Lossless optimization

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