// src/components/SettingsPanel.jsx

import { load } from "@tauri-apps/plugin-store";
import { useCallback, useEffect, useState } from "react";
import closeIcon from "../assets/icons/close.svg";
import { checkForUpdate, downloadAndInstall } from "../utils/updater";

const STORE_KEY = "autoCheckUpdates";

// Update check states
const UPDATE_STATE = {
  IDLE: "idle",
  CHECKING: "checking",
  AVAILABLE: "available",
  DOWNLOADING: "downloading",
  UP_TO_DATE: "upToDate",
  ERROR: "error",
};

function SettingsPanel({ show, onClose }) {
  const [autoCheck, setAutoCheck] = useState(true);
  const [updateState, setUpdateState] = useState(UPDATE_STATE.IDLE);
  const [updateInfo, setUpdateInfo] = useState(null);
  const [downloadProgress, setDownloadProgress] = useState(0);

  // Load persisted preference on mount
  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const store = await load("settings.json", { autoSave: true });
        const value = await store.get(STORE_KEY);
        if (!cancelled && value !== null && value !== undefined) {
          setAutoCheck(value);
        }
      } catch (error) {
        console.error("Failed to load settings:", error);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  // Persist toggle changes
  const handleToggle = useCallback(async () => {
    const newValue = !autoCheck;
    setAutoCheck(newValue);

    try {
      const store = await load("settings.json", { autoSave: true });
      await store.set(STORE_KEY, newValue);
    } catch (error) {
      console.error("Failed to save settings:", error);
    }
  }, [autoCheck]);

  // Manual update check
  const handleCheckForUpdates = useCallback(async () => {
    setUpdateState(UPDATE_STATE.CHECKING);
    setUpdateInfo(null);

    const result = await checkForUpdate();

    if (result) {
      setUpdateInfo(result);
      setUpdateState(UPDATE_STATE.AVAILABLE);
    } else {
      setUpdateState(UPDATE_STATE.UP_TO_DATE);
      // Reset after a few seconds
      setTimeout(() => setUpdateState(UPDATE_STATE.IDLE), 4000);
    }
  }, []);

  // Install the available update
  const handleInstallUpdate = useCallback(async () => {
    if (!updateInfo) return;

    setUpdateState(UPDATE_STATE.DOWNLOADING);
    setDownloadProgress(0);

    try {
      await downloadAndInstall(updateInfo, (event) => {
        if (event.contentLength > 0) {
          setDownloadProgress(
            Math.round((event.downloaded / event.contentLength) * 100),
          );
        }
      });
      // App will relaunch automatically after install
    } catch (error) {
      console.error("Failed to install update:", error);
      setUpdateState(UPDATE_STATE.ERROR);
      setTimeout(() => setUpdateState(UPDATE_STATE.IDLE), 4000);
    }
  }, [updateInfo]);

  const renderUpdateStatus = () => {
    switch (updateState) {
      case UPDATE_STATE.UP_TO_DATE:
        return (
          <span className="settings-panel__status settings-panel__status--success">
            Up to date
          </span>
        );
      case UPDATE_STATE.AVAILABLE:
        return (
          <span className="settings-panel__status settings-panel__status--available">
            v{updateInfo?.version} available
          </span>
        );
      case UPDATE_STATE.DOWNLOADING:
        return (
          <span className="settings-panel__status">
            Installing... {downloadProgress}%
          </span>
        );
      case UPDATE_STATE.ERROR:
        return (
          <span className="settings-panel__status settings-panel__status--error">
            Update failed
          </span>
        );
      default:
        return null;
    }
  };

  const getPrimaryAction = () => {
    if (updateState === UPDATE_STATE.AVAILABLE) {
      return {
        label: "Install update",
        onClick: handleInstallUpdate,
        disabled: false,
      };
    }

    if (updateState === UPDATE_STATE.CHECKING) {
      return {
        label: "Checking...",
        onClick: handleCheckForUpdates,
        disabled: true,
      };
    }

    if (updateState === UPDATE_STATE.DOWNLOADING) {
      return {
        label: `Installing... ${downloadProgress}%`,
        onClick: handleInstallUpdate,
        disabled: true,
      };
    }

    return {
      label: "Check for updates",
      onClick: handleCheckForUpdates,
      disabled: false,
    };
  };

  const primaryAction = getPrimaryAction();

  return (
    <div className={`settings-panel ${show ? "settings-panel--open" : ""}`}>
      <div className="settings-panel__surface" aria-hidden={!show}>
        <div className="settings-panel__header">
          <span className="settings-panel__title">Settings</span>
          <button
            type="button"
            className="settings-panel__close-btn"
            onClick={onClose}
            aria-label="Close settings"
          >
            <img
              className="settings-panel__close-icon"
              src={closeIcon}
              alt="Close"
            />
          </button>
        </div>

        <div className="settings-panel__body">
          <p className="settings-panel__section-label">App updates</p>
          <div className="settings-panel__divider" />

          <div className="settings-panel__item settings-panel__item--toggle">
            <button
              type="button"
              className={`settings-panel__toggle ${autoCheck ? "settings-panel__toggle--on" : ""}`}
              onClick={handleToggle}
              role="switch"
              aria-checked={autoCheck}
              title={autoCheck ? "Disable auto-check" : "Enable auto-check"}
            >
              <span className="settings-panel__toggle-thumb" />
            </button>
            <span className="settings-panel__toggle-label">
              Check for updates on startup
            </span>
          </div>

          <button
            type="button"
            className="settings-panel__check-btn"
            onClick={primaryAction.onClick}
            disabled={primaryAction.disabled}
          >
            {primaryAction.label}
          </button>

          <div className="settings-panel__status-row">{renderUpdateStatus()}</div>
        </div>
      </div>
    </div>
  );
}

export default SettingsPanel;
