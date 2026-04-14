#!/usr/bin/env bash
# scripts/collect-macos-dylibs.sh
#
# Recursively collects all non-system dylibs that libvips depends on,
# starting from the Homebrew-installed libvips and following otool -L
# references. Collected dylibs are staged for Tauri's macOS frameworks
# bundling, which places them in Contents/Frameworks/ and fixes @rpath.
#
# Compatible with bash 3.2+ (macOS system bash). Uses the staging
# directory itself to track already-collected libraries instead of
# associative arrays (which require bash 4+).

set -euo pipefail

STAGING="${1:-src-tauri/native-libs}"
mkdir -p "$STAGING"

collect() {
  local lib="$1"
  local name
  name=$(basename "$lib")

  # Already collected — file exists in staging
  [[ -f "$STAGING/$name" ]] && return

  # Skip macOS system libraries
  case "$lib" in /usr/lib/*|/System/*) return ;; esac
  [[ -f "$lib" ]] || return

  # Copy first so recursive calls see it and won't re-process
  cp -L "$lib" "$STAGING/$name"

  # Recurse into this dylib's own dependencies.
  # Process substitution keeps the while loop in the current shell.
  while read -r dep; do
    [[ -f "$dep" ]] && collect "$dep"
  done < <(otool -L "$lib" | awk 'NR>1 {print $1}')
}

VIPS_PREFIX=$(brew --prefix vips)
for lib in "$VIPS_PREFIX"/lib/libvips*.dylib; do
  [[ -f "$lib" ]] && collect "$lib"
done

COUNT=$(find "$STAGING" -name "*.dylib" -maxdepth 1 | wc -l | tr -d ' ')
echo "Collected $COUNT dylibs to $STAGING"
