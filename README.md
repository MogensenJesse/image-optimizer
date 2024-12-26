# Image Optimizer

A desktop application built with Tauri and React that optimizes images using Sharp. Drop your images into the application, and it will create optimized versions in an 'optimized' subfolder while preserving the original files.

## Features

- Drag and drop interface for easy image processing
- Processes multiple images in parallel
- Creates an 'optimized' folder in the same directory as the source images
- Real-time processing status and statistics
- Preserves original files
- Native performance with Tauri

## Technology Stack

- **Frontend**: React
- **Backend**: Tauri (Rust)
- **Image Processing**: Sharp (Node.js)
- **Build Tools**: Vite, pkg

## Prerequisites

- [Node.js](https://nodejs.org/) (v20 or later)
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/)

## Development Setup

1. Clone the repository:
   ```bash
   git clone [repository-url]
   cd image-optimizer
   ```

2. Install dependencies:
   ```bash
   # Install main application dependencies
   npm install
   
   # Install sidecar dependencies
   cd sidecar-app && npm install
   cd ..
   ```

3. Start the development server:
   ```bash
   npm run tauri dev
   ```
   
   This command will:
   - Build the sidecar application (Sharp binary)
   - Start the Tauri development server
   - Open the application

## Building for Production

To create a production build:
```bash
npm run tauri build
```

This will create platform-specific installers in the `src-tauri/target/release/bundle` directory.

## Acknowledgments

- [Tauri](https://tauri.app/) for the framework
- [Sharp](https://sharp.pixelplumbing.com/) for image processing
- [React](https://reactjs.org/) for the UI framework