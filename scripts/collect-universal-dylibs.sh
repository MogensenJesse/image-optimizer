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

# --- Shared collection logic: recursively walks otool -L references ----

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

# --- Merge matching pairs with lipo; bundle single-arch dylibs as-is ----
#
# When ARM64 and x86_64 Homebrew install different versions of a transitive
# dependency (e.g. libraw_r.23 vs libraw_r.25), the filenames differ and lipo
# cannot merge them.  Rather than silently discarding these dylibs, we copy
# each unmatched slice into the staging directory so it is bundled as a
# single-arch dylib.  dyld selects the correct slice at runtime: the x86_64
# binary loads x86_64-only dylibs; the arm64 binary loads arm64-only ones.
# The install-name fixup step then rewrites @rpath references in all staged
# dylibs, covering these single-arch entries as well.

MERGED=0
ARM64_ONLY=0
X86_64_ONLY=0

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
    cp "$arm_lib" "$STAGING/$name"
    ARM64_ONLY=$((ARM64_ONLY + 1))
    echo "  bundle arm64-only: $name"
  fi
done

for x86_lib in "$X86_64_DIR"/*.dylib; do
  if [[ ! -f "$x86_lib" ]]; then
    continue
  fi

  name=$(basename "$x86_lib")
  if [[ ! -f "$ARM64_DIR/$name" ]]; then
    cp "$x86_lib" "$STAGING/$name"
    X86_64_ONLY=$((X86_64_ONLY + 1))
    echo "  bundle x86_64-only: $name"
  fi
done

# --- Cleanup temp dirs ---------------------------------------------------

rm -rf "$ARM64_DIR" "$X86_64_DIR"

echo "Created $MERGED universal + $ARM64_ONLY arm64-only + $X86_64_ONLY x86_64-only dylibs in $STAGING"
