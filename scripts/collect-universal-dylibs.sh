#!/usr/bin/env bash
# scripts/collect-universal-dylibs.sh
#
# Produces universal (fat) dylibs for macOS by collecting ARM64 and x86_64
# dylibs from their respective Homebrew prefixes and merging each matching
# pair with lipo. The output is suitable for Tauri's bundle.macOS.frameworks
# config so the app runs on both Intel and Apple Silicon Macs.
#
# Usage:  ./scripts/collect-universal-dylibs.sh [staging-dir]
#
# Requires both Homebrew installations:
#   ARM64  → /opt/homebrew/  (default on Apple Silicon)
#   x86_64 → /usr/local/     (installed via: arch -x86_64 ... brew install vips)
#
# Compatible with macOS system bash 3.2.

set -euo pipefail

STAGING="${1:-src-tauri/native-libs}"
ARM64_DIR="$STAGING/tmp-arm64"
X86_64_DIR="$STAGING/tmp-x86_64"

mkdir -p "$STAGING" "$ARM64_DIR" "$X86_64_DIR"

# --- Shared collection logic (same as collect-macos-dylibs.sh) ----------

collect() {
  local lib="$1"
  local dest="$2"
  local name
  name=$(basename "$lib")

  if [[ -f "$dest/$name" ]]; then
    return 0
  fi

  case "$lib" in /usr/lib/*|/System/*) return 0 ;; esac

  if [[ ! -f "$lib" ]]; then
    return 0
  fi

  cp -L "$lib" "$dest/$name"

  local deps
  deps=$(otool -L "$lib" | awk 'NR>1 {print $1}')

  local dep
  for dep in $deps; do
    if [[ -f "$dep" ]]; then
      collect "$dep" "$dest"
    fi
  done

  return 0
}

# --- Collect ARM64 dylibs -----------------------------------------------

ARM64_VIPS=$(/opt/homebrew/bin/brew --prefix vips 2>/dev/null || true)
if [[ -n "$ARM64_VIPS" ]]; then
  for lib in "$ARM64_VIPS"/lib/libvips*.dylib; do
    if [[ -f "$lib" ]]; then
      collect "$lib" "$ARM64_DIR"
    fi
  done
  ARM64_COUNT=$(find "$ARM64_DIR" -name "*.dylib" -maxdepth 1 | wc -l | tr -d ' ')
  echo "Collected $ARM64_COUNT ARM64 dylibs"
else
  echo "::warning::ARM64 Homebrew vips not found at /opt/homebrew/"
fi

# --- Collect x86_64 dylibs ----------------------------------------------

X86_64_VIPS=$(arch -x86_64 /usr/local/bin/brew --prefix vips 2>/dev/null || true)
if [[ -n "$X86_64_VIPS" ]]; then
  for lib in "$X86_64_VIPS"/lib/libvips*.dylib; do
    if [[ -f "$lib" ]]; then
      collect "$lib" "$X86_64_DIR"
    fi
  done
  X86_64_COUNT=$(find "$X86_64_DIR" -name "*.dylib" -maxdepth 1 | wc -l | tr -d ' ')
  echo "Collected $X86_64_COUNT x86_64 dylibs"
else
  echo "::warning::x86_64 Homebrew vips not found at /usr/local/"
fi

# --- Merge matching pairs with lipo -------------------------------------

MERGED=0
SKIPPED=0

for arm_lib in "$ARM64_DIR"/*.dylib; do
  if [[ ! -f "$arm_lib" ]]; then
    continue
  fi

  name=$(basename "$arm_lib")
  x86_lib="$X86_64_DIR/$name"

  if [[ -f "$x86_lib" ]]; then
    lipo -create "$arm_lib" "$x86_lib" -output "$STAGING/$name"
    MERGED=$((MERGED + 1))
  else
    SKIPPED=$((SKIPPED + 1))
    echo "  skip: $name (ARM64 only, no x86_64 match)"
  fi
done

for x86_lib in "$X86_64_DIR"/*.dylib; do
  if [[ ! -f "$x86_lib" ]]; then
    continue
  fi

  name=$(basename "$x86_lib")
  if [[ ! -f "$ARM64_DIR/$name" ]]; then
    SKIPPED=$((SKIPPED + 1))
    echo "  skip: $name (x86_64 only, no ARM64 match)"
  fi
done

# --- Cleanup temp dirs ---------------------------------------------------

rm -rf "$ARM64_DIR" "$X86_64_DIR"

echo "Created $MERGED universal dylibs ($SKIPPED skipped) in $STAGING"
