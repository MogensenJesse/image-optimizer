# Image Optimizer Architecture

## Overview

A Tauri 2 desktop app with two parts: a React frontend and a Rust backend that processes images in-process through vendored libvips bindings (raster formats) and oxvg (SVG). The user drops files on the window or picks them in a dialog; the backend writes every result to an `optimized` folder next to the source file and reports progress back through Tauri events.

## Frontend (`src/`)

React 19 with Vite and SCSS. It talks to Rust through `invoke()` and `listen()` from `@tauri-apps/api`.

| File                          | Role                                                                                                                                     |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `main.jsx`                    | Entry point. Adds the `platform-linux` body class (Linux has no window vibrancy) and mounts `I18nProvider` and `App`                     |
| `App.jsx`                     | Layout, the settings state (`quality`, `resize`, `outputFormat`), the file picker and the startup update check                           |
| `hooks/useOptimizer.js`       | Everything around one batch: the phase state machine, drag-and-drop listeners, file filtering, the `optimize_images` call and the toasts |
| `hooks/useProgressTracker.js` | Holds the running totals from `image_optimization_progress` events and the final summary                                                 |
| `components/`                 | `ProgressBar`, `FloatingMenu` (quality, resize, convert), `SettingsPanel` (language, updates), `TitleBar`, `Toast`                       |
| `utils/store.js`              | The shared handle to the persisted `settings.json` store                                                                                 |
| `utils/updater.js`            | `checkForUpdate` and `downloadAndInstall` around `@tauri-apps/plugin-updater`                                                            |
| `i18n/`                       | `t(key, params)` with six catalogs in `locales/` (en, nl, de, fr, es, ru); also sets `<html lang>`                                       |

### Batch phases

`useOptimizer` moves through `IDLE → FADE_IN → PROCESSING → COMPLETED → FADE_OUT → IDLE`, with `DRAGGING` while files hover over the window. The phase decides which CSS classes are applied (`dropzone--dragging`, `dropzone--processing`, `progress-container fade-in|fade-out`, ...), so those class names are tied to the state machine. A batch is finished when the `optimize_images` call resolves, not when the last event arrives; the events only drive the live progress.

### Persisted settings and updates

`settings.json` (via `tauri-plugin-store`) holds `language` and `autoCheckUpdates`. The quality, resize and format settings are not persisted. On startup `App` checks GitHub Releases for an update (unless switched off) and, when one exists, puts a badge on the settings button and hands the result to `SettingsPanel`. Installing downloads the signed artifact, verifies it against the public key in `tauri.conf.json` and relaunches the app. The changelog shown in the panel is the GitHub release body.

## Backend (`src-tauri/src/`)

Tauri 2 with a minimal Tokio runtime (`spawn_blocking` only).

| File                 | Role                                                                                          |
| -------------------- | --------------------------------------------------------------------------------------------- |
| `main.rs`            | Entry point: logging, plugins, the command handler, libvips startup, vibrancy on macOS        |
| `commands.rs`        | The `optimize_images` command                                                                 |
| `error.rs`           | `OptimizerError` (validation / processing / format), serialized as a plain string for the UI  |
| `types.rs`           | `ImageSettings`, `ImageFormat`, `OutputFormat`, `ResizeMode`, `BatchProgress`, `BatchSummary` |
| `optimize/mod.rs`    | `VipsRuntime`, the parallel batch runner and output planning                                  |
| `optimize/raster.rs` | Load, orient, colour-convert, resize                                                          |
| `optimize/encode.rs` | Quality setting to format-specific save options                                               |
| `optimize/svg.rs`    | SVG minification with oxvg                                                                    |

### Command contract

```text
optimize_images(paths: string[], settings: ImageSettings) -> BatchSummary
```

```text
ImageSettings = {
  quality: 1..100,
  resize: { mode: "none" | "width" | "height" | "longest" | "shortest", size: number | null },
  outputFormat: "original" | "jpeg" | "png" | "webp" | "avif"
}
BatchProgress = { completed, total, failed, savedBytes, originalBytes }
BatchSummary  = BatchProgress + { durationMs, errors: [{ fileName, error }] }  // first 20 errors
```

While a batch runs, the command emits `image_optimization_progress` with a `BatchProgress` after every finished file. The emit happens under the progress lock, so `completed` only ever goes up. A single failing file is reported in `errors` and never stops the batch. A rejected call (for example an invalid quality) reaches the frontend as a string and is shown in a toast.

### Output files

Each result is written to `<source folder>/optimized/<name>`. The full list of output paths is planned before any worker starts, so workers never race on a name:

- SVGs, "original" and a conversion to the file's own format keep the file name
- any other conversion swaps the extension (`jpeg` becomes `jpg`)
- a name already taken in the same folder (for example `a.jpg` and `a.png` both converted to WebP) gets `-2`, `-3`, ... appended; the comparison is case-insensitive

### Parallel runner

The encoders (mozjpeg, libpng, libwebp) are single-threaded and libvips' own thread pool only speeds up pixel work such as resizing, so one image at a time leaves most cores idle. `run_batch` therefore spreads the files over `min(CPU cores, files, 8)` scoped worker threads that pull the next file from an atomic counter. The cap of 8 limits memory, because large AVIF and PNG jobs hold a full decoded image per worker. Each batch also sets the libvips thread count to `cores / workers`, and the libvips operation cache is off since every file is opened once. A panic inside one file is caught and counted as that file's failure (release builds use `panic = "unwind"` for this).

### Raster pipeline

1. **Load.** With resize, `vips_thumbnail` opens the file once and shrinks while decoding (shrink-on-load); it never enlarges and applies the EXIF orientation itself, so the target box is fitted against the displayed image. Without resize the file is read sequentially and rotated upright when its orientation tag is above 1.
2. **Colour.** An embedded ICC profile is converted to sRGB; images without a profile pass through.
3. **Encode.** Metadata is stripped, so steps 1 and 2 are what keep the picture upright and correctly coloured.

| Format | Lossy (quality below 100)                                   | Quality 100                                                                                           |
| ------ | ----------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| JPEG   | mozjpeg, progressive with optimised scans, optimised coding | adds trellis quantisation and overshoot deringing (still lossy)                                       |
| PNG    | palette quantisation                                        | lossless, max deflate compression, adaptive filtering                                                 |
| WebP   | lossy                                                       | lossless                                                                                              |
| AVIF   | AV1, 4:2:0 chroma (4:4:4 from quality 90)                   | q=100 with 4:4:4 chroma and max effort (the encoder's `lossless` flag is avoided, it shifted colours) |

Trellis quantisation is off below quality 100 because it made a 200-photo batch about 1.6 times slower for roughly 8% more savings.

### SVG pipeline

`optimize/svg.rs` parses the file and runs the default oxvg jobs. The quality slider only sets the float precision of coordinates (8 at 95 and above, 3 from 85, 2 from 70, 1 from 40, otherwise 0). SVGs ignore "convert to" and resizing.

## Native libraries

- **`vendor/libvips-rs/`**: libvips-rs 8.15.1, vendored and patched: fixed-width integer types for Windows, `thumbnail*` wrappers that skip empty profile strings, and three small accessors (`get_orientation`, `orientation_swaps_dims`, `has_icc_profile`). It must not be replaced by the crates.io copy.
- **`src-tauri/libvips_compat.c`**: a small C shim (Windows only) that provides symbols removed or renamed between libvips 8.15, which the bindings were generated against, and the 8.18.7 binaries that are linked.
- **`src-tauri/build.rs`**: on Windows links `vendor/libvips-native/lib` (or `VIPS_DIR`) and stops with download instructions when it is missing; in debug builds it also copies the DLLs next to the executables. On macOS and Linux it takes the flags from `pkg-config vips`.
- **Bundling**: Windows ships the DLLs as bundle resources that `src-tauri/windows/hooks.nsh` copies next to the exe at install time. macOS bundles universal dylibs staged by `scripts/collect-universal-dylibs.sh`. The Linux `.deb` depends on `libvips42` and the AppImage bundles libvips.

## Communication flow

```text
┌─────────────┐   invoke("optimize_images")   ┌──────────────┐
│   React     │ ────────────────────────────> │    Rust      │
│  Frontend   │ <──────────────────────────── │   Backend    │
└─────────────┘  image_optimization_progress  └──────────────┘
                 events, then the summary             │
                                                      │ in-process FFI
                                                      ▼
                                              ┌──────────────┐
                                              │   libvips    │
                                              │  (native C)  │
                                              └──────────────┘
```

1. **Frontend to backend**: `invoke("optimize_images", { paths, settings })` once per batch
2. **Backend**: plans the output paths, then the workers optimize files in parallel
3. **Backend to frontend**: one `image_optimization_progress` event per finished file, then the `BatchSummary` as the call's result
4. **Frontend**: the hooks store the totals and the UI follows the batch phases

## Permissions

`src-tauri/capabilities/default.json` grants the main window: core window, event and path defaults, `dialog` (file picker), `process` (relaunch after an update), `updater` and `store`. Files are read and written by the Rust backend, so the frontend has no filesystem permission.

## Testing and linting

- `cargo test` in `src-tauri/` runs the unit tests (output planning, resize boxes, settings parsing, a portrait photo saved upright, failure handling). Debug builds copy the libvips DLLs, so no PATH change is needed on Windows.
- `BENCH_DIR=<folder> cargo test bench_batch -- --ignored --nocapture` times a batch of real images.
- `npm run lint` runs `biome check` and Clippy with warnings as errors; `npm run lint:fix` also writes Biome fixes.
- The `lint` job in `build.yml` runs Biome, Clippy and the tests on every push, on Ubuntu 24.04, Ubuntu 22.04 and macOS, so the code is exercised against the libvips of each platform.

## Release process

1. **Update `CHANGELOG.md`**: put the new version's section first. CI publishes only that first section (everything above the first `---` line) as the GitHub release body, which is also what the in-app changelog shows.
2. **Bump the version**: this syncs `package.json`, `Cargo.toml`, `Cargo.lock` and `tauri.conf.json`, then commits and creates a git tag:

   ```bash
   npm version major|minor|patch
   ```

3. **Push the branch and tag**: pushing the `v*` tag starts the build workflow:

   ```bash
   git push origin main && git push origin v<x.x.x>
   ```

4. **Review the draft release** on GitHub: CI creates a draft with the artifacts for Windows, macOS and Linux. Check the notes and publish it.

See [GITHUB_ACTIONS.md](./GITHUB_ACTIONS.md) for what the workflow does.

## Known limitations

- The bindings target libvips 8.15 and the Windows build links 8.18.7. The Linux release build installs the libvips of Ubuntu 22.04 (8.12), where the newer save options are not expected to work; this is untested.
- The macOS minimum version in `tauri.conf.json` (10.13) is older than what the Homebrew libraries bundled by CI target; it has not been verified on a Mac.
- Animated WebP and AVIF inputs are reduced to their first frame, and SVGs ignore resize and conversion.
- An output can be larger than its input; it is still written and shows as negative savings.
