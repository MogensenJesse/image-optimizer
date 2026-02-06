# Image Optimizer Architecture

## Overview

This application is a Tauri-based desktop image optimizer with three main components: a React frontend, a Rust backend, and a Node.js sidecar process for image processing.

## Components

### React Frontend (`src/`)

- **Framework**: React 19 with Vite
- **UI**: Drag-and-drop interface with progress tracking
- **Communication**: Uses Tauri's `invoke()` API to call Rust commands and `listen()` API for event-driven progress updates
- **Key Files**:
  - `App.jsx`: Main application component managing state and file processing
  - `hooks/useProgressTracker.js`: Tracks optimization progress via Tauri events
- **Features**: File selection, drag-and-drop, settings configuration, real-time progress display

### Rust Backend (`src-tauri/src/`)

- **Framework**: Tauri 2 with Tokio async runtime
- **Responsibilities**: 
  - Exposes Tauri commands (`optimize_image`, `optimize_images`) to the frontend
  - Manages sidecar process lifecycle and communication
  - Handles progress event emission to frontend
  - Validates tasks and manages batch processing
- **Key Modules**:
  - `commands/image.rs`: Tauri command handlers
  - `processing/sharp/memory_map_executor.rs`: Sidecar executor with memory-mapped file communication
  - `core/`: Application state and task management
- **Progress Communication**: Emits `batch-progress` and `image_optimization_progress` events to frontend

### Node.js Sidecar (`sharp-sidecar/`)

- **Runtime**: Node.js packaged as standalone executable (`pkg`)
- **Purpose**: Image processing using Sharp library (libvips)
- **Communication**: 
  - Receives tasks via command-line arguments or memory-mapped JSON files
  - Outputs results as JSON to stdout (between `BATCH_RESULT_START`/`BATCH_RESULT_END` markers)
  - Sends progress updates via stdout for real-time feedback
- **Entry Point**: `index.js` handles command parsing and worker pool management
- **Processing**: Uses worker threads for parallel image optimization

## Communication Flow

```
┌─────────────┐         Tauri Invoke          ┌──────────────┐
│   React     │ ────────────────────────────> │    Rust      │
│  Frontend   │ <──────────────────────────── │   Backend    │
└─────────────┘      Tauri Events (progress)  └──────────────┘
                                                      │
                                                      │ spawn sidecar
                                                      │ stdin/stdout
                                                      ▼
                                              ┌──────────────┐
                                              │   Node.js     │
                                              │   Sidecar     │
                                              │  (Sharp)      │
                                              └──────────────┘
```

1. **Frontend → Backend**: React calls `invoke("optimize_images", { tasks })` with image paths and settings
2. **Backend → Sidecar**: Rust spawns sidecar process via `tauri_plugin_shell`, passes batch data via memory-mapped file or command-line args
3. **Sidecar → Backend**: Node.js processes images, outputs JSON results to stdout, sends progress messages
4. **Backend → Frontend**: Rust parses sidecar output, emits Tauri events (`image_optimization_progress`, `batch-progress`)
5. **Frontend Updates**: React hooks listen to events and update UI progress state

## Key Design Decisions

- **Sidecar Architecture**: Node.js sidecar isolates Sharp dependency and enables independent updates
- **Memory-Mapped Files**: Used for large batch transfers to avoid command-line length limits
- **Event-Driven Progress**: Real-time UI updates via Tauri events without polling
- **Batch Processing**: Images processed in chunks (500 per batch) for scalability
- **Worker Threads**: Sidecar uses Node.js worker threads for parallel processing

## Useful CLI commands

- npm version major/minor/patch
- git push origin Development && git push origin vx.x.x (push to Development branch and tag the version)