fn main() {
    // ── libvips linking ──────────────────────────────────────────────────────
    //
    // The libvips-rs bindings crate ships no build script of its own.
    // We must tell cargo where to find the import library for each platform.
    //
    // Development setup (Windows x64):
    //   1. Download vips-dev-x64-web-8.18.7.zip from
    //      https://github.com/libvips/build-win64-mxe/releases/tag/v8.18.7
    //   2. Extract to <workspace-root>/vendor/libvips-native/
    //   3. Nothing else: debug builds copy the DLLs next to the executables
    //      (see `copy_dlls_for_dev`). Release builds bundle them in CI.
    //
    // The VIPS_DIR environment variable overrides the default location.
    link_libvips();

    // Tauri build will embed Windows resources (icons) if RC.EXE is available.
    tauri_build::build()
}

fn link_libvips() {
    // Re-run whenever the override env-var changes.
    println!("cargo:rerun-if-env-changed=VIPS_DIR");

    let vips_dir = std::env::var("VIPS_DIR").unwrap_or_else(|_| {
        // Default: <workspace-root>/vendor/libvips-native
        let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("src-tauri must be inside a workspace root");
        workspace
            .join("vendor")
            .join("libvips-native")
            .to_string_lossy()
            .to_string()
    });

    let lib_dir = std::path::Path::new(&vips_dir).join("lib");

    if cfg!(target_os = "windows") {
        // Re-run when the libvips folders appear, disappear or change (a
        // version upgrade), so a stale "not found" or old DLL copy never sticks.
        println!("cargo:rerun-if-changed={}", lib_dir.display());
        println!(
            "cargo:rerun-if-changed={}",
            std::path::Path::new(&vips_dir).join("bin").display()
        );

        assert!(
            lib_dir.exists(),
            "libvips vendor directory not found at '{vips_dir}'.\n\
             Download vips-dev-x64-web-8.18.7.zip from \
             https://github.com/libvips/build-win64-mxe/releases/tag/v8.18.7 \
             and extract it to vendor/libvips-native/ (or set VIPS_DIR)."
        );

        // Use cargo:rustc-link-arg (absolute paths) rather than
        // cargo:rustc-link-lib.  This was needed while the crate also
        // built a cdylib (the binary linker got the -L search path but
        // not the -l names).  That lib target is gone, so rustc-link-lib
        // would likely work now; the explicit paths are kept because they
        // are proven and have not been re-tested on every platform.
        let link_arg = |name: &str| {
            println!("cargo:rustc-link-arg={}", lib_dir.join(name).display());
        };
        link_arg("libvips.lib");
        // GLib symbols (g_free, g_object_unref, …) called directly by
        // libvips_rs's image.rs must also be explicitly linked.
        link_arg("libglib-2.0.lib");
        link_arg("libgobject-2.0.lib");

        copy_dlls_for_dev(&vips_dir);

        // Compile a thin C shim that provides symbols removed or renamed
        // between the libvips version that libvips-rs 8.15.1 was generated
        // against and the 8.18.7 Windows binaries we ship.
        build_compat_shim(&vips_dir);
    } else {
        // macOS / Linux: use pkg-config to discover vips and all its
        // transitive dependencies (glib, gobject, etc.).  libvips-rs
        // calls GLib symbols directly, and on macOS the linker does NOT
        // resolve them transitively through libvips.dylib.
        //
        // We emit the flags as rustc-link-arg (not rustc-link-lib); see the
        // Windows branch above for why.
        if let Some(flags) = pkg_config_libs("vips") {
            for flag in flags.split_whitespace() {
                println!("cargo:rustc-link-arg={flag}");
            }
        } else {
            // Fallback when pkg-config is unavailable or vips.pc is not found.
            // VIPS_DIR is the Homebrew prefix of vips; libgobject lives in the
            // glib formula dir, so -L<vips>/lib alone does not always find it.
            if lib_dir.exists() {
                println!("cargo:rustc-link-arg=-L{}", lib_dir.display());
            }
            println!("cargo:rustc-link-arg=-lvips");
            println!("cargo:rustc-link-arg=-lgobject-2.0");
            println!("cargo:rustc-link-arg=-lglib-2.0");
        }
    }
}

/// Debug builds only: copies the libvips DLLs next to the executables Cargo
/// produces (`target/debug` for the app, `target/debug/deps` for tests), so
/// `tauri dev` and `cargo test` start without editing PATH. A DLL is replaced
/// when its size differs, which covers a libvips upgrade.
fn copy_dlls_for_dev(vips_dir: &str) {
    if std::env::var("PROFILE").as_deref() != Ok("debug") {
        return;
    }

    // OUT_DIR is <target>/debug/build/<package>-<hash>/out
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR must be set by Cargo"));
    let Some(profile_dir) = out_dir.ancestors().nth(3) else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(std::path::Path::new(vips_dir).join("bin")) else {
        return;
    };

    for entry in entries.flatten() {
        let src = entry.path();
        if src.extension().is_none_or(|ext| !ext.eq_ignore_ascii_case("dll")) {
            continue;
        }
        let src_len = entry.metadata().map(|m| m.len()).ok();

        for dest_dir in [profile_dir.to_path_buf(), profile_dir.join("deps")] {
            let dest = dest_dir.join(entry.file_name());
            let up_to_date = std::fs::metadata(&dest).map(|m| m.len()).ok() == src_len;
            if !up_to_date && let Err(e) = std::fs::copy(&src, &dest) {
                // Most likely the dev app is running and has the DLL locked.
                println!("cargo:warning=Could not copy {}: {e}", src.display());
            }
        }
    }
}

/// Runs `pkg-config --libs <package>` and returns the flags on success.
///
/// For universal macOS builds (`--target universal-apple-darwin`), Cargo
/// invokes `build.rs` once per architecture.  ARM64 Homebrew lives at
/// `/opt/homebrew/` while x86_64 Homebrew lives at `/usr/local/`.  We
/// override `PKG_CONFIG_PATH` based on `TARGET` so the linker receives
/// the correct library search paths for each slice.
fn pkg_config_libs(package: &str) -> Option<String> {
    let mut cmd = std::process::Command::new("pkg-config");
    cmd.args(["--libs", package]);

    if let Ok(target) = std::env::var("TARGET")
        && target == "x86_64-apple-darwin"
    {
        // On Apple Silicon CI runners (macos-latest), x86_64 Homebrew
        // installs to /usr/local.  The top-level /usr/local/lib/pkgconfig
        // may only contain a sparse set of symlinks, so include the
        // formula-specific pkgconfig dirs for vips and glib (which
        // provides gobject-2.0 and glib-2.0) so all Requires: entries
        // in vips.pc can be resolved by a single pkg-config invocation.
        let paths = "/usr/local/lib/pkgconfig\
                    :/usr/local/opt/vips/lib/pkgconfig\
                    :/usr/local/opt/glib/lib/pkgconfig";
        cmd.env("PKG_CONFIG_PATH", paths);
        cmd.env("PKG_CONFIG_LIBDIR", paths);
    }

    cmd.output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Compiles `libvips_compat.c` into a static archive and links it via an
/// absolute-path link-arg so the symbol stubs reach the binary linker.
fn build_compat_shim(vips_dir: &str) {
    let include_dir = std::path::Path::new(vips_dir).join("include");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR must be set by Cargo");

    let mut build = cc::Build::new();
    build.file("libvips_compat.c");

    if include_dir.exists() {
        build.include(&include_dir);
    }

    // cc::Build::compile() emits cargo:rustc-link-lib=static=libvips_compat
    // AND cargo:rustc-link-search=native=OUT_DIR.  We additionally emit an
    // absolute-path link-arg (see link_libvips) to guarantee the binary gets it.
    build.compile("libvips_compat");

    // On MSVC, cc names the archive after the argument passed to compile():
    // `libvips_compat.lib`.  Emit an explicit link-arg so the binary picks it up.
    let shim_lib = std::path::Path::new(&out_dir).join("libvips_compat.lib");
    if shim_lib.exists() {
        println!("cargo:rustc-link-arg={}", shim_lib.display());
    }

    println!("cargo:rerun-if-changed=libvips_compat.c");
}
