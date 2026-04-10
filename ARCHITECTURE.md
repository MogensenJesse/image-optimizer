# Image Optimizer Architecture

## Overview

This application is a Tauri-based desktop image optimizer with two main components: a React frontend and a Rust backend that processes images natively via vendored libvips bindings.

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
  - Processes images in-process via native libvips bindings
  - Handles progress event emission to frontend
  - Validates tasks and manages batch processing
- **Key Modules**:
  - `commands/image.rs`: Tauri command handlers
  - `processing/libvips/executor.rs`: Native libvips executor with batch processing
  - `processing/libvips/formats.rs`: Format-specific save options (JPEG, PNG, WebP, AVIF)
  - `processing/libvips/resize.rs`: Resize mode mapping to libvips thumbnail operations
  - `core/`: Application state, types, and task definitions
  - `utils/`: Error handling, validation, and format utilities
- **Progress Communication**: Emits `batch-progress` and `image_optimization_progress` events to frontend

### Vendored libvips Bindings (`vendor/libvips-rs/`)

- **Purpose**: Rust FFI bindings for libvips, vendored and patched for Windows compatibility
- **Components**:
  - `bindings.rs`: Auto-generated FFI bindings from `bindgen`
  - `ops.rs`: Safe Rust wrappers around libvips operations
  - `manual.rs`: Hand-written bindings for operations not covered by code generation

## Communication Flow

```
┌─────────────┐         Tauri Invoke          ┌──────────────┐
│   React     │ ────────────────────────────> │    Rust      │
│  Frontend   │ <──────────────────────────── │   Backend    │
└─────────────┘      Tauri Events (progress)  └──────────────┘
                                                      │
                                                      │ in-process FFI
                                                      ▼
                                              ┌──────────────┐
                                              │   libvips    │
                                              │  (native C)  │
                                              └──────────────┘
```

1. **Frontend → Backend**: React calls `invoke("optimize_images", { tasks })` with image paths and settings
2. **Backend → libvips**: Rust loads images via `VipsImage::new_from_file`, applies resize and format conversion through libvips ops
3. **Backend → Frontend**: Rust emits Tauri events (`image_optimization_progress`, `batch-progress`) with per-image results
4. **Frontend Updates**: React hooks listen to events and update UI progress state

## Key Design Decisions

- **Native libvips**: Images are processed in-process via vendored Rust-to-C bindings, eliminating subprocess overhead
- **Blocking Tasks on Async Runtime**: Each image is processed inside `tokio::task::spawn_blocking` so the async runtime is never blocked; libvips uses its own internal thread pool for per-image parallelism
- **Event-Driven Progress**: Real-time UI updates via Tauri events without polling
- **Batch Processing**: Images processed in chunks (500 per batch) for scalability
- **ICC Profile Handling**: sRGB fallback profiles are set on resize operations to handle images without embedded profiles

## Release Process

1. **Update `CHANGELOG.md`** with the new version's changes — CI reads this file to populate the GitHub Release body and the updater's `latest.json` notes
2. **Bump the version** — this syncs `package.json`, `Cargo.toml`, `tauri.conf.json`, and `Cargo.lock`, then commits and creates a git tag:
  ```
   npm version major/minor/patch
  ```
3. **Push the branch and tag** — pushing the `v`* tag triggers the CI build workflow:
  ```
   git push origin Development && git push origin v<x.x.x>
  ```
4. **Review the draft release** on GitHub — CI creates a draft release with build artifacts for Windows, macOS, and Linux. Verify the release notes and publish it

