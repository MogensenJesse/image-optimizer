# Image Optimizer

A desktop application built with Tauri and React that optimizes images using Sharp. Drop your images into the application, and it will create optimized versions in an 'optimized' subfolder while preserving the original files.

## Features

- Drag and drop interface for easy image processing
- Processes multiple images in parallel
- Creates an 'optimized' folder in the same directory as the source images
- Real-time processing status and statistics
- Preserves original files
- Native performance with Tauri
- Dark mode support

## Technology Stack

- **Frontend**: React
- **Backend**: Tauri (Rust)
- **Image Processing**: Sharp (Node.js)
- **Build Tools**: Vite, pkg

## Prerequisites

- [Node.js](https://nodejs.org/) (v20 or later)
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri Prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites)

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

## Project Structure

.
├── src/ # React frontend code
├── src-tauri/ # Rust backend code
│ ├── src/
│ │ ├── commands/ # Tauri commands
│ │ └── lib.rs # Main Rust entry point
│ └── binaries/ # Location for sidecar binaries
├── sidecar-app/ # Node.js sidecar application
│ ├── index.js # Sharp image processing logic
│ └── package.json # Sidecar dependencies
└── package.json # Main application dependencies


## Building for Production

To create a production build:
bash
npm run tauri build


This will create platform-specific installers in the `src-tauri/target/release/bundle` directory.

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

[License Type] - See LICENSE file for details

## Acknowledgments

- [Tauri](https://tauri.app/) for the framework
- [Sharp](https://sharp.pixelplumbing.com/) for image processing
- [React](https://reactjs.org/) for the UI framework