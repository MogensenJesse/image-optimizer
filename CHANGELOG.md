## What's New in v0.5.3

Image optimization is now powered by a native Rust engine (libvips), replacing the Node.js sidecar. This results in a **20–50% increase in optimization speed**, lower memory usage, and a simpler architecture.

### New Features

- **Native image processing engine** — all optimization now runs natively in Rust using libvips, eliminating the separate Node.js process
- **Smart lossless compression** — quality 100% now uses maximum compression effort across all formats (JPEG, PNG, WebP, AVIF) for the smallest possible file sizes

### Improvements

- **Shrink-on-load** for JPEG resizing — only decodes the pixels needed, significantly faster for large images
- **Sequential file access** for compression-only operations, reducing I/O overhead
- Optimized build settings for faster image processing
- Lossy mode encoding tuned to prioritize speed while maintaining great quality
- Settings UI improvements
- Updated JS and Rust dependencies

### Fixes

- Resolved AVIF color shifts in lossless mode
- Fixed AVIF bit-depth compatibility issues
- Fixed CI builds for Windows, Linux, and macOS

### Other

- Removed Node.js sidecar and all related IPC/serialization code
- Simplified project architecture to a single-process model
