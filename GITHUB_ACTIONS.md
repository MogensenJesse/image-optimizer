# GitHub Actions CI/CD Setup

This project uses GitHub Actions with `[tauri-action](https://github.com/tauri-apps/tauri-action)` to build cross-platform binaries automatically.

## Workflow Overview

The workflow (`.github/workflows/build.yml`) builds the app for:

- **Windows** (x64)
- **macOS** (x64 Intel and arm64 Apple Silicon)
- **Linux** (x64)

## Build Process

1. **Checkout** repository code
2. **Setup** Node.js 20 (with npm cache) and Rust toolchain
3. **Restore caches** (Rust dependencies)
4. **Install Linux dependencies** (for Linux builds only: `libvips-dev`, `pkg-config`, `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`)
5. **Install npm dependencies** (root)
6. **Build frontend** (Vite)
7. **Build Tauri app** using `tauri-action` (which handles bundling)

## Caching Strategy

The workflow uses caching to significantly reduce build times:


| Cache          | Action                   | Key Based On                 |
| -------------- | ------------------------ | ---------------------------- |
| **npm (root)** | `setup-node` built-in    | `package-lock.json`          |
| **Rust/Cargo** | `Swatinem/rust-cache@v2` | `Cargo.lock` + rustc version |


**Expected savings**: 60-70% reduction in build time on cache hits.

## Triggering Builds

### Automatic (on tag push)

Push a tag starting with `v`*:

```bash
git tag v1.0.0
git push origin v1.0.0
```

### Manual (workflow_dispatch)

Go to Actions → Build Tauri App → Run workflow

## Build Artifacts

Built artifacts are automatically uploaded to GitHub Releases as draft releases. You can:

- Review the artifacts
- Publish the release when ready
- Download platform-specific installers

## Platform-Specific Notes

### Windows

- Builds NSIS installer

### macOS

- Builds for both Intel (`x86_64-apple-darwin`) and Apple Silicon (`aarch64-apple-darwin`)
- Creates `.dmg` and `.app` bundles

### Linux

- Builds AppImage (default) or other formats based on Tauri config
- Requires system dependencies (installed automatically in CI):
  - `libvips-dev` - Native image processing library
  - `libgtk-3-dev`, `libwebkit2gtk-4.1-dev` - GTK3/WebKit for Tauri
  - `libappindicator3-dev`, `librsvg2-dev`, `patchelf` - Additional Tauri requirements

## Troubleshooting

### Linux Build Failures

- Ensure all Linux dependencies are installed (handled automatically in CI):
  - `libvips-dev`, `pkg-config` for native image processing
  - `libgtk-3-dev`, `libwebkit2gtk-4.1-dev` for Tauri GTK/WebKit
  - `libappindicator3-dev`, `librsvg2-dev`, `patchelf` for Tauri
- If `gdk-3.0` or `gtk-3.0` errors occur, ensure GTK3 dev packages are installed

### Release Upload Issues

- Ensure `GITHUB_TOKEN` has `contents: write` permission (handled automatically)
- Check that tag format matches `v*` pattern

