#!/bin/bash
# This script fixes permissions and runs install_name_tool on macOS binaries
# to ensure library paths are correctly set.

if [ "$(uname)" != "Darwin" ]; then
  echo "This script is only for macOS. Skipping..."
  exit 0
fi

echo "Running macOS post-build fixup script"
BINARY_PATH="$1"

# If no binary path provided, try to find it in the target directory
if [ -z "$BINARY_PATH" ]; then
  TARGET_DIR="../src-tauri/binaries"
  BINARY_PATH=$(find "$TARGET_DIR" -name "sharp-sidecar-*" -type f -not -name "*.exe" | head -n 1)
  if [ -n "$BINARY_PATH" ]; then
    echo "Found binary: $BINARY_PATH"
  fi
fi

if [ ! -f "$BINARY_PATH" ]; then
  echo "Binary not found: $BINARY_PATH"
  # Continue anyway to ensure directories are created
fi

# Create directories needed for Sharp's native modules
mkdir -p "../src-tauri/binaries/sharp/build/Release"
mkdir -p "../src-tauri/binaries/sharp/vendor/lib"
mkdir -p "../src-tauri/binaries/libs"

# Copy native modules from node_modules to the distribution directory
echo "Copying Sharp native modules to distribution directory..."
cp -R "./node_modules/sharp/build/Release/" "../src-tauri/binaries/sharp/build/Release/" || echo "Warning: Failed to copy Release directory"
cp -R "./node_modules/sharp/vendor/lib/" "../src-tauri/binaries/sharp/vendor/lib/" || echo "Warning: Failed to copy vendor/lib directory"

# Copy libvips libraries to the libs directory
echo "Copying libvips libraries to libs directory..."
find "./node_modules/sharp/vendor/lib" -name "libvips*.dylib" -exec cp {} "../src-tauri/binaries/libs/" \; || echo "Warning: Failed to copy libvips libraries"

# If binary exists, fix permissions
if [ -f "$BINARY_PATH" ]; then
  echo "Setting executable permissions..."
  chmod +x "$BINARY_PATH"
fi

echo "Post-build steps completed successfully."
