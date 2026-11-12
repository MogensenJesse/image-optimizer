# GitHub Actions CI/CD Setup

The workflow in `.github/workflows/build.yml` lints and tests the code and builds the app for every platform with [tauri-action](https://github.com/tauri-apps/tauri-action).

## Triggers

| Trigger                      | What runs                                                                                                                              |
| ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Push of a `v*` tag           | Lint job and full build; a **draft** GitHub release is created                                                                         |
| Push to `main`               | Lint job and full build, but no release. These builds exist to fill the caches on the default branch, so a later tag build is not cold |
| Manual (`workflow_dispatch`) | Same as a `main` push                                                                                                                  |

Runs for tags are never cancelled mid-flight, because an aborted run can leave a half-uploaded release asset. Runs for `main` and manual runs are cancelled when a newer one starts.

## Jobs

### `lint` (ubuntu-24.04)

Runs `npx biome check .`, builds the frontend (the Tauri macro needs `dist/` at compile time), then `cargo clippy --all-targets -- -D warnings` and `cargo test`. Ubuntu 24.04 ships libvips 8.15, the version the Rust bindings target. It is independent of the build job, so a lint failure does not block a release.

### `build-tauri` (matrix)

| Platform         | Output              | Notes                                                                                        |
| ---------------- | ------------------- | -------------------------------------------------------------------------------------------- |
| `windows-latest` | NSIS installer      | x64                                                                                          |
| `macos-latest`   | `.dmg` and `.app`   | universal binary (`aarch64` + `x86_64`)                                                      |
| `ubuntu-22.04`   | AppImage and `.deb` | Ubuntu 22.04 (glibc 2.35) keeps the build usable on Debian 12+, Ubuntu 22.04+ and Fedora 36+ |

Common steps, in order: checkout, Node.js 24 (npm cache), `npm ci`, Rust toolchain, Rust cache, then the platform steps below, the changelog step and `tauri-action`.

#### Windows

1. Restore or download libvips 8.18.7 (`vips-dev-x64-web-8.18.7.zip`) into `vendor/libvips-native`
2. Add `vendor\libvips-native\bin` to PATH
3. Stage the DLLs in `src-tauri/native-dlls` and pass a `--config` override so they are bundled as resources. The NSIS hook `src-tauri/windows/hooks.nsh` copies them next to the exe when the app is installed
4. After the build, check that the DLLs were staged

#### macOS

1. Give the runner user ownership of `/usr/local` and restore the cached x86_64 Homebrew
2. Install libvips for both architectures in parallel (ARM64 in the background) and fail if the x86_64 prefix is missing
3. `scripts/collect-universal-dylibs.sh` merges the two sets into universal dylibs, and a `--config` override lists them as bundle frameworks
4. Rewrite the install names to `@rpath/...` so dyld finds the bundled copies
5. After the build, mount the `.dmg` and verify that no absolute Homebrew paths remain and that every `@rpath` reference resolves inside `Frameworks/`

#### Linux

Installs `libvips-dev`, `pkg-config`, `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev` and `patchelf`. After the build the AppImage is unpacked to check that libvips is inside it (a warning, not a failure).

## Release notes

The release body, and with it the changelog shown in the app's update panel, is the first section of `CHANGELOG.md`: everything above the first `---` line. Keep the newest version on top and separate versions with `---`.

A tag containing a hyphen (for example `v0.7.0-beta.1`) is published as a prerelease.

## Caching

| Cache                   | Key                                                                                                                                                                      |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| npm                     | `package-lock.json` (built into `setup-node`)                                                                                                                            |
| Rust `target/`          | `Swatinem/rust-cache`, shared key per platform (`rust-universal-apple-darwin`, `rust-x86_64-pc-windows-msvc`, `rust-x86_64-unknown-linux-gnu`; the lint job has its own) |
| libvips (Windows)       | `libvips-win64-8.18.7`, falls back to any `libvips-win64-*`                                                                                                              |
| x86_64 Homebrew (macOS) | `macos-x86-homebrew-vips-v1`; bump to `v2` when vips or a major dependency needs a fresh install                                                                         |

## Secrets

| Secret                               | Used for                                                                       |
| ------------------------------------ | ------------------------------------------------------------------------------ |
| `GITHUB_TOKEN`                       | Creating the release (the job has `contents: write`)                           |
| `TAURI_SIGNING_PRIVATE_KEY`          | Signing the updater artifacts; the matching public key is in `tauri.conf.json` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password of that key                                                           |

Windows and macOS installers are not code-signed.

## Triggering a release

```bash
npm version patch   # or minor / major; syncs all version files, commits and tags
git push origin main && git push origin v<x.x.x>
```

Then review the draft release on GitHub and publish it. The full checklist is in [ARCHITECTURE.md](./ARCHITECTURE.md#release-process).

## Troubleshooting

### Linux build failures

- Make sure the Linux packages above are installed (the workflow does this)
- If `gdk-3.0` or `gtk-3.0` errors appear, the GTK3 dev packages are missing

### Windows build cannot find libvips

`build.rs` stops with the download instructions when `vendor/libvips-native/lib` is missing. Check the cache key and the download step.

### Release upload issues

- Check that the tag matches the `v*` pattern
- The job needs `contents: write`, which the workflow sets
