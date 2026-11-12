## v0.7.0

### Improvements

- **Parallel batches** — images are optimized on several CPU cores at once instead of one after another; in a test with 200 large photos on an 8-core machine a batch took 8.8 s before and 6.0 s now, with the smaller files described below
- **Smaller JPEGs** — JPEG output is now progressive with optimised scans, which saves about 8% more at the same quality
- **Failures are visible** — a file that cannot be optimized no longer counts as done: a message names the first failed file and how many others failed, and the summary only counts files that were really optimized
- **No more name clashes** — converting `a.jpg` and `a.png` to WebP now writes `a.webp` and `a-2.webp` instead of overwriting one with the other
- **Clearer update check** — a failed check now says so instead of reporting "Up to date"
- **Accessibility** — hidden panels can no longer be reached with the keyboard, the progress indicator is announced in the chosen language and the page language follows the app language
- **Updated dependencies** — all JS and Rust packages, and libvips 8.18.7

### Fixes

- Fixed portrait photos coming out sideways when they were not resized
- Fixed wide-gamut photos (such as Display-P3) looking washed out; they are now converted to sRGB
- Fixed "longest" and "shortest" side resizing on rotated photos using the unrotated size
- Fixed the quality slider reaching 0, which rejected the whole batch
- Fixed a negative resize size silently rejecting the whole batch
- Fixed the processing timer never stopping when the last file failed
- Fixed toast messages never leaving the screen
- Fixed "1 images optimized" for a single image
- Fixed an unexpected crash while processing one file taking down the whole app

### Under the hood

- One `optimize_images` call per batch with typed progress events, replacing hundreds of small calls from the UI
- Removed unused commands, plugins and settings, and flattened the Rust modules
- Linting and tests now run in CI, and the Windows dev build copies the libvips DLLs itself

---

## v0.6.10

### Improvements

- **Faster release builds** — better caching of Rust and native-library dependencies in CI

---

## v0.6.2 - v0.6.9

### Improvements

- **Faster CI builds** — added Rust and libvips caching, removed duplicate macOS build
- **macOS universal binary** — single download now works on both Intel and Apple Silicon Macs
- **Translated UI** — "Options" label and file-size units (B, KB, MB) are now localized in all languages
- **More reliable releases** — every macOS build is now automatically inspected before publishing to confirm all required libraries are bundled

### Fixes

- Fixed native DLLs not being copied next to the executable on Windows
- Fixed macOS build script compatibility with older bash versions
- Fixed compilation failure on macOS caused by LTO and window-vibrancy conflict
- Fixed macOS app crash on launch caused by a missing native library inside the bundle
- Dropped unused MSI installer to simplify Windows distribution

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

## v0.6.0

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
