const { execSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const os = require("node:os");

// Determine the current platform
const platform = os.platform();

// Create necessary directories for all platforms
const targetDir = path.resolve(__dirname, "../src-tauri/binaries");
const sharpReleaseDir = path.join(targetDir, "sharp/build/Release");
const sharpVendorDir = path.join(targetDir, "sharp/vendor/lib");
const libsDir = path.join(targetDir, "libs");

// Ensure directories exist
console.log("Creating necessary directories...");
[sharpReleaseDir, sharpVendorDir, libsDir].forEach((dir) => {
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
      execSync(`cp -R "${vendorSource}/" "${sharpVendorDir}/"`);
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
      copyDirRecursiveSync(vendorSource, sharpVendorDir);
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
