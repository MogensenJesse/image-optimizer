## What's New in v0.6.0

### New Features

- **SVG optimization** — SVG files are now supported via vexy-vsvg (a native Rust port of SVGO), removing metadata, minifying paths, and collapsing groups for smaller file sizes
- **Internationalization** — the app is now available in 6 languages: English, Dutch, German, French, Spanish, and Russian
- **Toast notifications** — non-intrusive feedback when unsupported files are skipped, with auto-dismiss animation

### Improvements

- **Structured logging** — configurable log levels via `RUST_LOG`
- **UI accessibility** — consistent focus indicators on all interactive elements
- **Typography consistency** — standardized font weights and spacing across panels and menus
- **Dropdown styling** — added chevron icon to the language selector
- **Skipped-files messaging** — shows one filename with a count of others instead of listing all
- **Progress bar responsiveness** — visual now closely tracks the percentage text
- **Unified progress tracking** — progress bar now accurately reflects overall job completion
- Updated JS and Rust dependencies

### Fixes

- Fixed toast notification only appearing once for repeated actions
- Fixed progress events being dropped during fade-in animation
- Fixed processing timer not starting for fast operations


---

## v0.6.1

### Improvements

- **SVG optimization** — replaced vexy-vsvg with oxvg_optimiser for significantly better SVG minification (up to 67% compression)
- **Adaptive size display** — file savings now shown in B, KB, or MB depending on magnitude
- **Native library bundling** — libvips and its dependencies are now included in all platform installers (Windows DLLs, macOS dylibs, Linux deb dependency)
- **In-app changelog rendering** — bold and inline code markdown now renders correctly

### Fixes

- Fixed "DLL not found" errors on Windows installations (`libvips-42.dll`, `libgobject-2.0-0.dll`)
- Fixed silent launch failures on macOS and Linux due to missing native libraries
- Fixed "0 MB saved" display when optimizing small files

---

## v0.6.2 – v0.6.7

### Improvements

- **Faster CI builds** — added Rust and libvips caching, removed duplicate macOS build
- **macOS universal binary** — single download now works on both Intel and Apple Silicon Macs
- **Translated UI** — "Options" label and file-size units (B, KB, MB) are now localized in all languages

### Fixes

- Fixed native DLLs not being copied next to the executable on Windows
- Fixed macOS build script compatibility with older bash versions
- Fixed compilation failure on macOS caused by LTO and window-vibrancy conflict
- Dropped unused MSI installer to simplify Windows distribution
