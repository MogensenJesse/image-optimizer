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

// Update src-tauri/Cargo.lock (keep lockfile in sync with Cargo.toml)
const cargoLockPath = join(rootDir, "src-tauri", "Cargo.lock");
if (existsSync(cargoLockPath)) {
  let lockContent = readFileSync(cargoLockPath, "utf8");
  // Match the [[package]] block for our crate and update its version
  lockContent = lockContent.replace(
    /(name = "image-optimizer"\nversion = ")[^"]+"/,
    `$1${version}"`,
  );
  writeFileSync(cargoLockPath, lockContent);
  console.log("  ✓ Updated src-tauri/Cargo.lock");
}

// Update src-tauri/tauri.conf.json
const tauriConfPath = join(rootDir, "src-tauri", "tauri.conf.json");
const tauriConf = JSON.parse(readFileSync(tauriConfPath, "utf8"));
tauriConf.version = version;
writeFileSync(tauriConfPath, `${JSON.stringify(tauriConf, null, 2)}\n`);
console.log("  ✓ Updated src-tauri/tauri.conf.json");

console.log(`\nVersion synced to ${version} across all files.`);
