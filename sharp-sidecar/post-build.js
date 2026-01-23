// sharp-sidecar/post-build.js
const { execSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const os = require("node:os");

// Determine the current platform
const platform = os.platform();

// Create necessary directories for all platforms
const targetDir = path.resolve(__dirname, "../src-tauri/binaries");
const sharpReleaseDir = path.join(targetDir, "sharp/build/Release");
const sharpVendorRoot = path.join(targetDir, "sharp/vendor");
const sharpVendorLibDir = path.join(sharpVendorRoot, "lib");
const libsDir = path.join(targetDir, "libs");

// Ensure directories exist
console.log("Creating necessary directories...");
[sharpReleaseDir, sharpVendorRoot, sharpVendorLibDir, libsDir].forEach((dir) => {
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
    console.log(`Created directory: ${dir}`);
  }
});

// Find the binary
const _ext = platform === "win32" ? ".exe" : "";
const _binPattern =
  platform === "win32" ? "sharp-sidecar-*.exe" : "sharp-sidecar-*";

let binaryPath = "";
try {
  // Find the binary in the target directory
  const files = fs.readdirSync(targetDir);
  const binFile = files.find(
    (file) =>
      file.startsWith("sharp-sidecar-") &&
      (platform === "win32" ? file.endsWith(".exe") : !file.endsWith(".exe")),
  );

  if (binFile) {
    binaryPath = path.join(targetDir, binFile);
    console.log(`Found binary: ${binaryPath}`);
  }
} catch (error) {
  console.error(`Error finding binary: ${error.message}`);
}

// Platform-specific operations
if (platform === "darwin") {
  console.log("Running macOS-specific post-build operations...");

  try {
    // Copy Sharp native modules to distribution directory
    console.log("Copying Sharp native modules...");
    const sharpModulePath = path.join(__dirname, "node_modules/sharp");
    const releaseSource = path.join(sharpModulePath, "build/Release");
    const vendorSource = path.join(sharpModulePath, "vendor/lib");

    if (fs.existsSync(releaseSource)) {
      execSync(`cp -R "${releaseSource}/" "${sharpReleaseDir}/"`);
      console.log("Copied Release directory");
    }

    if (fs.existsSync(vendorSource)) {
      execSync(`cp -R "${vendorSource}/" "${sharpVendorLibDir}/"`);
      console.log("Copied vendor/lib directory");

      // Copy libvips libraries to libs directory
      console.log("Copying libvips libraries...");
      execSync(
        `find "${vendorSource}" -name "libvips*.dylib" -exec cp {} "${libsDir}/" \\;`,
      );
    }

    // Set executable permissions
    if (binaryPath && fs.existsSync(binaryPath)) {
      console.log("Setting executable permissions...");
      fs.chmodSync(binaryPath, 0o755);
    }

    console.log("macOS post-build operations completed successfully");
  } catch (error) {
    console.error(`Error during macOS post-build: ${error.message}`);
  }
} else if (platform === "win32") {
  console.log("Running Windows-specific post-build operations...");

  try {
    // Windows needs the sharp native modules in the appropriate location
    console.log("Copying Sharp native modules for Windows...");
    const sharpModulePath = path.join(__dirname, "node_modules/sharp");

    // Copy necessary DLLs if needed (for Windows)
    const releaseSource = path.join(sharpModulePath, "build/Release");
    const vendorSource = path.join(sharpModulePath, "vendor/lib");

    if (fs.existsSync(releaseSource)) {
      // Windows doesn't support cp -R directly, use fs methods
      copyDirRecursiveSync(releaseSource, sharpReleaseDir);
      console.log("Copied Release directory");
    }

    if (fs.existsSync(vendorSource)) {
      copyDirRecursiveSync(vendorSource, sharpVendorLibDir);
      console.log("Copied vendor/lib directory");

      // Copy libvips DLLs to libs directory
      fs.readdirSync(vendorSource).forEach((file) => {
        if (file.startsWith("libvips") && file.endsWith(".dll")) {
          fs.copyFileSync(
            path.join(vendorSource, file),
            path.join(libsDir, file),
          );
        }
      });
    }

    console.log("Windows post-build operations completed successfully");
  } catch (error) {
    console.error(`Error during Windows post-build: ${error.message}`);
  }
} else if (platform === "linux") {
  console.log("Running Linux-specific post-build operations...");

  try {
    // Modern Sharp (0.33+) uses @img/ scoped packages for native modules
    const imgModulesPath = path.join(__dirname, "node_modules/@img");
    const sharpModulePath = path.join(__dirname, "node_modules/sharp");

    // Determine architecture suffix
    const arch = process.arch; // x64, arm64, etc.
    const sharpNativePackage = path.join(imgModulesPath, `sharp-linux-${arch}`);
    const libvipsPackage = path.join(imgModulesPath, `sharp-libvips-linux-${arch}`);

    console.log("Copying Sharp native modules from @img/ packages...");

    // Copy the native .node file from @img/sharp-linux-x64
    if (fs.existsSync(sharpNativePackage)) {
      const nodeFile = path.join(sharpNativePackage, "lib", `sharp-linux-${arch}.node`);
      if (fs.existsSync(nodeFile)) {
        const destFile = path.join(sharpReleaseDir, `sharp-linux-${arch}.node`);
        fs.copyFileSync(nodeFile, destFile);
        console.log(`Copied native module: ${nodeFile} -> ${destFile}`);
      } else {
        console.error(`Native module not found at: ${nodeFile}`);
      }
    } else {
      console.error(`Sharp native package not found: ${sharpNativePackage}`);
      // Fallback: try old structure
      const releaseSource = path.join(sharpModulePath, "build/Release");
      if (fs.existsSync(releaseSource)) {
        execSync(`cp -R "${releaseSource}/" "${sharpReleaseDir}/"`);
        console.log("Copied Release directory (legacy structure)");
      }
    }

    // Copy libvips shared libraries from @img/sharp-libvips-linux-x64
    if (fs.existsSync(libvipsPackage)) {
      const libvipsLibDir = path.join(libvipsPackage, "lib");
      if (fs.existsSync(libvipsLibDir)) {
        // Copy all .so files to libs directory
        const libFiles = fs.readdirSync(libvipsLibDir);
        for (const file of libFiles) {
          if (file.endsWith(".so") || file.includes(".so.")) {
            const srcFile = path.join(libvipsLibDir, file);
            const destFile = path.join(libsDir, file);
            // Use cp to follow symlinks and copy actual files
            try {
              execSync(`cp -L "${srcFile}" "${destFile}" 2>/dev/null || cp "${srcFile}" "${destFile}"`);
              console.log(`Copied libvips library: ${file}`);
            } catch (e) {
              // If it's a directory or special file, skip
              console.log(`Skipping: ${file}`);
            }
          }
        }

        // Also copy to vendor/lib for compatibility
        execSync(`cp -rL "${libvipsLibDir}/"* "${sharpVendorLibDir}/" 2>/dev/null || true`);
        console.log("Copied libvips to vendor/lib directory");
      } else {
        console.error(`libvips lib directory not found: ${libvipsLibDir}`);
      }
    } else {
      console.error(`libvips package not found: ${libvipsPackage}`);
      // Fallback: try old structure
      const vendorSource = path.join(sharpModulePath, "vendor/lib");
      if (fs.existsSync(vendorSource)) {
        execSync(`cp -R "${vendorSource}/" "${sharpVendorLibDir}/"`);
        console.log("Copied vendor/lib directory (legacy structure)");
        execSync(
          `find "${vendorSource}" -name "libvips*.so*" -exec cp {} "${libsDir}/" \\;`,
        );
      }
    }

    // Set executable permissions
    if (binaryPath && fs.existsSync(binaryPath)) {
      console.log("Setting executable permissions...");
      fs.chmodSync(binaryPath, 0o755);
    }

    // Set executable permissions on the .node file as well
    const nodeModulePath = path.join(sharpReleaseDir, `sharp-linux-${arch}.node`);
    if (fs.existsSync(nodeModulePath)) {
      fs.chmodSync(nodeModulePath, 0o755);
      console.log("Set executable permissions on native module");
    }

    // Create symlinks for libvips (the native module may look for different versions)
    console.log("Creating libvips symlinks...");
    const libFiles = fs.readdirSync(libsDir);
    for (const file of libFiles) {
      if (file.startsWith("libvips-cpp.so.") && !fs.lstatSync(path.join(libsDir, file)).isSymbolicLink()) {
        // Extract version info and create symlinks
        // e.g., libvips-cpp.so.8.17.3 -> libvips-cpp.so.8.17, libvips-cpp.so.8, libvips-cpp.so
        const parts = file.replace("libvips-cpp.so.", "").split(".");
        if (parts.length >= 2) {
          const majorMinor = `libvips-cpp.so.${parts[0]}.${parts[1]}`;
          const major = `libvips-cpp.so.${parts[0]}`;
          const base = "libvips-cpp.so";
          
          try {
            // Create symlinks (remove existing if any)
            for (const linkName of [majorMinor, major, base]) {
              const linkPath = path.join(libsDir, linkName);
              if (fs.existsSync(linkPath)) {
                fs.unlinkSync(linkPath);
              }
              fs.symlinkSync(file, linkPath);
              console.log(`Created symlink: ${linkName} -> ${file}`);
            }
          } catch (e) {
            console.log(`Note: Could not create some symlinks: ${e.message}`);
          }
        }
      }
    }

    console.log("Linux post-build operations completed successfully");
  } catch (error) {
    console.error(`Error during Linux post-build: ${error.message}`);
    console.error(error.stack);
  }
} else {
  console.log(`No specific post-build operations for platform: ${platform}`);
}

console.log("Post-build process completed");

// Helper function to recursively copy directories on Windows
function copyDirRecursiveSync(src, dest) {
  if (!fs.existsSync(dest)) {
    fs.mkdirSync(dest, { recursive: true });
  }

  const entries = fs.readdirSync(src, { withFileTypes: true });

  for (const entry of entries) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);

    if (entry.isDirectory()) {
      copyDirRecursiveSync(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}
