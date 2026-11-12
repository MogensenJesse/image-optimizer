// src/utils/store.js
import { load } from "@tauri-apps/plugin-store";

/**
 * Shared handle (a promise) to the persisted app settings: the chosen
 * language and the "check for updates on startup" toggle.
 */
export const settingsStore = load("settings.json", { autoSave: true });
