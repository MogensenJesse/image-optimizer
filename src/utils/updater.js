// src/utils/updater.js
//
// Wraps @tauri-apps/plugin-updater with a simple API for the settings panel.

import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";

/**
 * Check for an available update.
 * @returns {{ version: string, body: string | null, date: string | null } | null}
 */
export async function checkForUpdate() {
  try {
    const update = await check();
    if (!update) return null;

    return {
      version: update.version,
      body: update.body ?? null,
      date: update.date ?? null,
      // Keep the raw update object so we can download/install later
      _update: update,
    };
  } catch (error) {
    console.error("Failed to check for updates:", error);
    return null;
  }
}

/**
 * Download and install a previously fetched update, then restart the app.
 * @param {object} updateInfo - The object returned by `checkForUpdate()`
 * @param {(event: { event: string, data?: object }) => void} onProgress - Progress callback
 */
export async function downloadAndInstall(updateInfo, onProgress) {
  if (!updateInfo?._update) {
    throw new Error("No update to install");
  }

  let downloaded = 0;
  let contentLength = 0;

  await updateInfo._update.downloadAndInstall((event) => {
    switch (event.event) {
      case "Started":
        contentLength = event.data.contentLength ?? 0;
        break;
      case "Progress":
        downloaded += event.data.chunkLength;
        break;
      case "Finished":
        break;
    }

    // Forward all events to the caller
    if (onProgress) {
      onProgress({
        ...event,
        downloaded,
        contentLength,
      });
    }
  });

  await relaunch();
}
