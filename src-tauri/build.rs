fn main() {
    // ── libvips linking ──────────────────────────────────────────────────────
    //
    // The libvips-rs bindings crate ships no build script of its own.
    // We must tell cargo where to find the import library for each platform.
    //
    // Development setup (Windows x64):
    //   1. Download vips-dev-w64-web-8.18.0.zip from
    //      https://github.com/libvips/build-win64-mxe/releases/tag/v8.18.0
    //   2. Extract to <workspace-root>/vendor/libvips-native/
    //   3. Add vendor/libvips-native/bin to your PATH (or copy DLLs next to
    //      the built .exe) so they are found at runtime.
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
        if lib_dir.exists() {
            // Use cargo:rustc-link-arg (absolute paths) rather than
            // cargo:rustc-link-lib.  The latter has a known propagation gap
            // when a crate builds both a cdylib and a binary target: the
            // binary linker receives the -L search path but NOT the -l lib
            // names, so the symbols stay unresolved.
            // cargo:rustc-link-arg explicitly applies to both cdylib and bin.
            let link_arg = |name: &str| {
                println!(
                    "cargo:rustc-link-arg={}",
                    lib_dir.join(name).display()
                );
            };
            link_arg("libvips.lib");
            // GLib symbols (g_free, g_object_unref, …) called directly by
            // libvips_rs's image.rs must also be explicitly linked.
            link_arg("libglib-2.0.lib");
            link_arg("libgobject-2.0.lib");
        } else {
            println!("cargo:warning=libvips vendor directory not found at '{vips_dir}'.");
            println!(
                "cargo:warning=Download vips-dev-w64-web-8.18.0.zip from \
                 https://github.com/libvips/build-win64-mxe/releases/tag/v8.18.0 \
                 and extract to vendor/libvips-native/"
            );
        }

        // Compile a thin C shim that provides symbols removed or renamed
        // between the libvips version that libvips-rs 8.15.1 was generated
        // against and the 8.18.0 Windows binaries we ship.
        build_compat_shim(&vips_dir);
    } else if cfg!(target_os = "macos") {
        if lib_dir.exists() {
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
        }
        println!("cargo:rustc-link-lib=dylib=vips");
    } else {
        // Linux: system libvips-dev package is sufficient.
        println!("cargo:rustc-link-lib=dylib=vips");
    }
}

/// Compiles `libvips_compat.c` into a static archive and links it via an
/// absolute-path link-arg so the symbol stubs reach BOTH the cdylib and the
/// binary linker.
fn build_compat_shim(vips_dir: &str) {
    let include_dir = std::path::Path::new(vips_dir).join("include");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR must be set by Cargo");

    let mut build = cc::Build::new();
    build.file("libvips_compat.c");

    if include_dir.exists() {
        build.include(&include_dir);
    }

    // cc::Build::compile() emits cargo:rustc-link-lib=static=libvips_compat
    // AND cargo:rustc-link-search=native=OUT_DIR.  Those alone don't reach the
    // binary target on Windows (same propagation gap as rustc-link-lib).
    // We additionally emit an absolute-path link-arg to guarantee it.
    build.compile("libvips_compat");

    // On MSVC, cc names the archive after the argument passed to compile():
    // `libvips_compat.lib`.  Emit an explicit link-arg so the binary picks it up.
    let shim_lib = std::path::Path::new(&out_dir).join("libvips_compat.lib");
    if shim_lib.exists() {
        println!("cargo:rustc-link-arg={}", shim_lib.display());
    }

    println!("cargo:rerun-if-changed=libvips_compat.c");
}
