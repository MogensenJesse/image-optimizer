const { execSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const os = require("node:os");

// Get the Rust target triple
const rustInfo = execSync("rustc -vV");
const targetTriple = /host: (\S+)/g.exec(rustInfo)[1];
if (!targetTriple) {
  console.error("Failed to determine platform target triple");
  process.exit(1);
}

// Determine platform and architecture
const platform = os.platform();
const arch = os.arch();

// Get platform-specific extension
const ext = platform === "win32" ? ".exe" : "";

// Ensure the target directory exists
const targetDir = path.resolve(__dirname, "../src-tauri/binaries");
if (!fs.existsSync(targetDir)) {
  console.log(`Creating directory: ${targetDir}`);
  fs.mkdirSync(targetDir, { recursive: true });
}

// Map of possible source file names, prioritizing current platform
const possibleSourceFiles = [];

// First add platform-specific files
if (platform === "darwin") {
  // macOS specific, prioritize correct architecture
  if (arch === "arm64") {
    possibleSourceFiles.push("sharp-sidecar-macos-arm64");
  } else {
    possibleSourceFiles.push("sharp-sidecar-macos-x64");
  }
  possibleSourceFiles.push(
    "sharp-sidecar-macos-arm64",
    "sharp-sidecar-macos-x64",
  );
} else if (platform === "win32") {
  // Windows specific
  possibleSourceFiles.push("sharp-sidecar-win-x64.exe", "sharp-sidecar.exe");
} else if (platform === "linux") {
  // Linux specific
  possibleSourceFiles.push("sharp-sidecar-linux-x64", "sharp-sidecar");
}

// Then add generic names
possibleSourceFiles.push(
  `sharp-sidecar-${platform}-${arch}${ext}`,
  `sharp-sidecar${ext}`,
);

// Display what files we're looking for
console.log("Looking for files in priority order:", possibleSourceFiles);
console.log("Directory contains:", fs.readdirSync("."));

// Find the first existing source file
let sourcePath = null;
for (const file of possibleSourceFiles) {
  if (fs.existsSync(file)) {
    sourcePath = file;
    break;
  }
}

// If no source file found, display error
if (!sourcePath) {
  console.error("Error: No suitable source file found!");
  process.exit(1);
}

const destPath = path.join(targetDir, `sharp-sidecar-${targetTriple}${ext}`);

// Perform the rename
console.log(`Renaming: ${sourcePath} -> ${destPath}`);
fs.renameSync(sourcePath, destPath);
console.log(`Successfully renamed to ${destPath}`);

// Clean up any remaining sidecar binaries
console.log("Cleaning up unused sidecar binaries...");
const filesToCleanup = [
  "sharp-sidecar-macos-arm64",
  "sharp-sidecar-macos-x64",
  "sharp-sidecar-win-x64.exe",
  "sharp-sidecar-linux-x64",
  "sharp-sidecar.exe",
  "sharp-sidecar",
];

filesToCleanup.forEach((file) => {
  if (fs.existsSync(file)) {
    console.log(`Removing unused binary: ${file}`);
    fs.unlinkSync(file);
  }
});

console.log("Cleanup completed successfully.");
