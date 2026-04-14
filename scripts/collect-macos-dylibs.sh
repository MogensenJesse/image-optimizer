#!/usr/bin/env bash
# scripts/collect-macos-dylibs.sh
#
# Recursively collects all non-system dylibs that libvips depends on,
# starting from the Homebrew-installed libvips and following otool -L
# references. Collected dylibs are staged for Tauri's macOS frameworks
# bundling, which places them in Contents/Frameworks/ and fixes @rpath.
#
# Requires bash 4+ for associative arrays (use Homebrew bash on macOS).

set -euo pipefail

STAGING="${1:-src-tauri/native-libs}"
mkdir -p "$STAGING"

declare -A SEEN

collect() {
  local lib="$1"
  local name
  name=$(basename "$lib")

  [[ -n "${SEEN[$name]:-}" ]] && return
  SEEN[$name]=1

  case "$lib" in /usr/lib/*|/System/*) return ;; esac
  [[ -f "$lib" ]] || return

  cp -L "$lib" "$STAGING/$name"

  # Process substitution keeps the while loop in the current shell,
  # so SEEN modifications propagate correctly across recursive calls.
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
