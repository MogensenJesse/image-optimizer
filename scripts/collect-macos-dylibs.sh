#!/usr/bin/env bash
# scripts/collect-macos-dylibs.sh
#
# Recursively collects all non-system dylibs that libvips depends on,
# starting from the Homebrew-installed libvips and following otool -L
# references. Collected dylibs are staged for Tauri's macOS frameworks
# bundling, which places them in Contents/Frameworks/ and fixes @rpath.
#
# Compatible with macOS system bash 3.2 — avoids associative arrays
# and [[ ]] && / || patterns that interact poorly with set -e.

set -euo pipefail

STAGING="${1:-src-tauri/native-libs}"
mkdir -p "$STAGING"

collect() {
  local lib="$1"
  local name
  name=$(basename "$lib")

  # Already collected — file exists in staging
  if [[ -f "$STAGING/$name" ]]; then
    return 0
  fi

  # Skip macOS system libraries
  case "$lib" in /usr/lib/*|/System/*) return 0 ;; esac

  if [[ ! -f "$lib" ]]; then
    return 0
  fi

  # Copy first so recursive calls see it and won't re-process
  cp -L "$lib" "$STAGING/$name"

  # Capture dependency list, then iterate (avoids subshell/pipe issues)
  local deps
  deps=$(otool -L "$lib" | awk 'NR>1 {print $1}')

  local dep
  for dep in $deps; do
    if [[ -f "$dep" ]]; then
      collect "$dep"
    fi
  done

  return 0
}

VIPS_PREFIX=$(brew --prefix vips)
for lib in "$VIPS_PREFIX"/lib/libvips*.dylib; do
  if [[ -f "$lib" ]]; then
    collect "$lib"
  fi
done

COUNT=$(find "$STAGING" -name "*.dylib" -maxdepth 1 | wc -l | tr -d ' ')
echo "Collected $COUNT dylibs to $STAGING"
