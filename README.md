# Image Optimizer

A cross-platform desktop application built with Tauri v2 that optimizes images while maintaining quality. The app features a modern React frontend and leverages Sharp for efficient image processing.

## Features

- 🖼️ Drag and drop interface for image optimization
- 📁 Batch processing support
- 🚀 High-performance image processing using Sharp
- 💾 Automatic creation of optimized output directories
- 📊 Real-time optimization statistics
- 🎨 Native OS integration with window effects
- 🔒 Secure architecture using Tauri's security model

## Tech Stack

- **Frontend**: React 18
- **Backend**: Rust + Tauri v2
- **Image Processing**: Sharp (via Node.js sidecar)
- **Build Tools**: Vite, pkg

## Project Structure

```
image-optimizer/
├── src/               # React frontend
├── src-tauri/         # Rust backend
├── sharp-sidecar/     # Node.js image processing service
└── dist/              # Built frontend files
```

## Prerequisites

- [Node.js](https://nodejs.org/) (v20 or later)
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/)

## Development Setup

1. Install dependencies:
    ```bash
    # Install frontend dependencies
    npm install

    # Install sharp-sidecar dependencies
    cd sharp-sidecar
    npm install
    ```

2. Run the development server:
    ```bash
    npm run tauri dev
    ```

## Building

1. Build the application:
    ```bash
    npm run tauri build
    ```

This will:
- Build the Sharp sidecar executable
- Compile the React frontend
- Package everything into a native executable

## Architecture

### Frontend (React)
- Handles drag and drop functionality
- Manages optimization state and progress
- Provides real-time feedback on optimization process

### Backend (Rust/Tauri)
- Manages file system operations
- Handles IPC between frontend and sidecar
- Implements security boundaries
- Controls process lifecycle

### Sidecar (Node.js/Sharp)
- Performs image optimization
- Handles various image formats
- Implements optimization algorithms

## Plugins Used

- `tauri-plugin-process`: Process management
- `tauri-plugin-dialog`: Native dialogs
- `tauri-plugin-fs`: File system operations
- `tauri-plugin-shell`: Shell command execution
- `tauri-plugin-opener`: File opening capabilities

## Security

- Implements Tauri's security model
- Controlled file system access
- Sandboxed image processing
- Type-safe IPC communication