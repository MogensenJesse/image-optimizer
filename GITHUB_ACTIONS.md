# GitHub Actions CI/CD Setup

This project uses GitHub Actions with [`tauri-action`](https://github.com/tauri-apps/tauri-action) to build cross-platform binaries automatically.

## Workflow Overview

The workflow (`.github/workflows/build.yml`) builds the app for:
- **Windows** (x64)
- **macOS** (x64 Intel and arm64 Apple Silicon)
- **Linux** (x64)

## Build Process

1. **Checkout** repository code
2. **Setup** Node.js 20 and Rust toolchain
3. **Install Linux dependencies** (for Linux builds only: `libvips-dev`, `pkg-config`)
4. **Install npm dependencies** (root and sidecar)
5. **Build sidecar** binary using `pkg` (creates platform-specific executables)
6. **Build frontend** (Vite)
7. **Build Tauri app** using `tauri-action` (which handles bundling)

## Triggering Builds

### Automatic (on tag push)
Push a tag starting with `v*`:
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
- Sidecar binary: `sharp-sidecar-x86_64-pc-windows-msvc.exe`

### macOS
- Builds for both Intel (`x86_64-apple-darwin`) and Apple Silicon (`aarch64-apple-darwin`)
- Creates `.dmg` and `.app` bundles
- Sidecar binaries: `sharp-sidecar-x86_64-apple-darwin` or `sharp-sidecar-aarch64-apple-darwin`

### Linux
- Builds AppImage (default) or other formats based on Tauri config
- Requires `libvips-dev` system dependency (installed automatically in CI)
- Sidecar binary: `sharp-sidecar-x86_64-unknown-linux-gnu`

## Troubleshooting

### Sidecar Build Issues
- Ensure `pkg` builds binaries for all target platforms
- Check that Sharp native modules are copied correctly in `post-build.js`
- Verify platform-specific handling in `rename.js`

### Linux Build Failures
- Ensure `libvips-dev` is installed (handled automatically in CI)
- Check that Sharp's Linux binaries are available

### Release Upload Issues
- Ensure `GITHUB_TOKEN` has `contents: write` permission (handled automatically)
- Check that tag format matches `v*` pattern

