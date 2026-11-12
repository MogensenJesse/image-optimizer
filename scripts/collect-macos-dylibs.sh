#!/usr/bin/env bash
# scripts/collect-macos-dylibs.sh
#
# Copies libvips and every Homebrew dylib it depends on into a staging
# directory, ready for Tauri's bundle.macOS.frameworks config, so the app runs
# on Macs without Homebrew. The dylibs are those of the machine running the
# script, so the result only runs on that CPU architecture: Apple Silicon and
# Intel are built on separate runners.
#
# Usage:  ./scripts/collect-macos-dylibs.sh [staging-dir]
#
# Compatible with macOS system bash 3.2.

set -euo pipefail

STAGING="${1:-src-tauri/native-libs}"
mkdir -p "$STAGING"

BREW_ROOT="$(brew --prefix)"
VIPS_PREFIX="$(brew --prefix vips 2>/dev/null || true)"
if [[ -z "$VIPS_PREFIX" || ! -d "$VIPS_PREFIX/lib" ]]; then
  echo "::error::Homebrew vips not found (brew --prefix vips gave '$VIPS_PREFIX')"
  exit 1
fi

# Modern Homebrew bottles are relocatable: deps are recorded as
# @rpath/<name>, @loader_path/<name>, or @executable_path/<name> rather
# than absolute Cellar paths.  For those we look up the real file by
# (1) checking the referencing dylib's own directory (covers the common
# intra-formula case like libwebp -> libsharpyuv living in the same lib
# dir), and (2) walking the Homebrew opt tree.

resolve_dep() {
  local dep="$1"
  local src_lib="$2"
  local base
  base=$(basename "$dep")

  case "$dep" in
    @rpath/*|@loader_path/*|@executable_path/*)
      local src_dir
      src_dir=$(dirname "$src_lib")
      if [[ -f "$src_dir/$base" ]]; then
        echo "$src_dir/$base"
        return 0
      fi
      local candidate
      for candidate in "$BREW_ROOT"/opt/*/lib/"$base"; do
        if [[ -f "$candidate" ]]; then
          echo "$candidate"
          return 0
        fi
      done
      return 0
      ;;
    *)
      echo "$dep"
      return 0
      ;;
  esac
}

collect() {
  local lib="$1"
  local name
  name=$(basename "$lib")

  if [[ -f "$STAGING/$name" ]]; then
    return 0
  fi

  case "$lib" in /usr/lib/*|/System/*) return 0 ;; esac

  if [[ ! -f "$lib" ]]; then
    return 0
  fi

  cp -L "$lib" "$STAGING/$name"

  local deps
  deps=$(otool -L "$lib" | awk 'NR>1 {print $1}')

  local dep
  local resolved
  for dep in $deps; do
    resolved=$(resolve_dep "$dep" "$lib")
    if [[ -n "$resolved" && -f "$resolved" ]]; then
      collect "$resolved"
    fi
  done

  return 0
}

for lib in "$VIPS_PREFIX"/lib/libvips*.dylib; do
  if [[ -f "$lib" ]]; then
    collect "$lib"
  fi
done

COUNT=$(find "$STAGING" -name "*.dylib" -maxdepth 1 | wc -l | tr -d ' ')
if [[ "$COUNT" -eq 0 ]]; then
  echo "::error::No dylibs collected from $VIPS_PREFIX"
  exit 1
fi
echo "Collected $COUNT $(uname -m) dylibs in $STAGING"
