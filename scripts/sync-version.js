#!/usr/bin/env node
// scripts/sync-version.js
// Syncs version from package.json to Cargo.toml, Cargo.lock, and tauri.conf.json

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const rootDir = join(__dirname, "..");

// Read version from package.json (single source of truth)
const packageJson = JSON.parse(
  readFileSync(join(rootDir, "package.json"), "utf8"),
);
const version = packageJson.version;

console.log(`Syncing version: ${version}`);

// Update src-tauri/Cargo.toml
const cargoPath = join(rootDir, "src-tauri", "Cargo.toml");
let cargoContent = readFileSync(cargoPath, "utf8");
cargoContent = cargoContent.replace(
  /^version = "[^"]+"/m,
  `version = "${version}"`,
);
writeFileSync(cargoPath, cargoContent);
console.log("  ✓ Updated src-tauri/Cargo.toml");

// Update src-tauri/Cargo.lock (keep lockfile in sync with Cargo.toml).
// The regex tolerates CRLF line endings: on Windows Cargo.lock is written
// with \r\n, so a regex using only \n would silently fail to match.  We
// also track whether the regex actually matched so any future drift
// surfaces as a loud error rather than a stale lockfile — a pure
// before/after compare would false-negative when the version is already
// up to date (idempotent run).
const cargoLockPath = join(rootDir, "src-tauri", "Cargo.lock");
if (existsSync(cargoLockPath)) {
  const lockContent = readFileSync(cargoLockPath, "utf8");
  let matched = false;
  const lockAfter = lockContent.replace(
    /(name = "image-optimizer"\r?\nversion = ")[^"]+"/,
    (_, prefix) => {
      matched = true;
      return `${prefix}${version}"`;
    },
  );
  if (!matched) {
    throw new Error(
      "Cargo.lock version pattern did not match; refusing to silently no-op. " +
        "Check that the [[package]] block for 'image-optimizer' still exists in src-tauri/Cargo.lock.",
    );
  }
  writeFileSync(cargoLockPath, lockAfter);
  console.log("  ✓ Updated src-tauri/Cargo.lock");
}

// Update src-tauri/tauri.conf.json
const tauriConfPath = join(rootDir, "src-tauri", "tauri.conf.json");
const tauriConf = JSON.parse(readFileSync(tauriConfPath, "utf8"));
tauriConf.version = version;
writeFileSync(tauriConfPath, `${JSON.stringify(tauriConf, null, 2)}\n`);
console.log("  ✓ Updated src-tauri/tauri.conf.json");

console.log(`\nVersion synced to ${version} across all files.`);
