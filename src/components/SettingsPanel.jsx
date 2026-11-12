// src/components/SettingsPanel.jsx

import { useCallback, useEffect, useRef, useState } from "react";
import closeIcon from "../assets/icons/close.svg";
import { LANGUAGES, useTranslation } from "../i18n";
import { settingsStore } from "../utils/store";
import { checkForUpdate, downloadAndInstall } from "../utils/updater";

const STORE_KEY = "autoCheckUpdates";

// Update check states
const UPDATE_STATE = {
  IDLE: "idle",
  CHECKING: "checking",
  AVAILABLE: "available",
  DOWNLOADING: "downloading",
  UP_TO_DATE: "upToDate",
  CHECK_ERROR: "checkError",
  ERROR: "error",
};

const STATUS_RESET_MS = 4000;

function SettingsPanel({ show, onClose, startupUpdate }) {
  const { t, language, setLanguage } = useTranslation();
  const [autoCheck, setAutoCheck] = useState(true);
  const [updateState, setUpdateState] = useState(UPDATE_STATE.IDLE);
  const [updateInfo, setUpdateInfo] = useState(null);
  const [downloadProgress, setDownloadProgress] = useState(0);

  // One reset timer for all transient statuses, so an old timer can never
  // overwrite a newer state.
  const resetTimerRef = useRef(null);
  const clearResetTimer = useCallback(() => {
    clearTimeout(resetTimerRef.current);
  }, []);
  const resetStatusLater = useCallback(() => {
    clearTimeout(resetTimerRef.current);
    resetTimerRef.current = setTimeout(
      () => setUpdateState(UPDATE_STATE.IDLE),
      STATUS_RESET_MS,
    );
  }, []);
  useEffect(() => clearResetTimer, [clearResetTimer]);

  // The startup check already found an update: show it without checking again
  useEffect(() => {
    if (!startupUpdate) return;
    setUpdateInfo(startupUpdate);
    setUpdateState((s) =>
      s === UPDATE_STATE.IDLE ? UPDATE_STATE.AVAILABLE : s,
    );
  }, [startupUpdate]);

  // Load persisted preference on mount
  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const store = await settingsStore;
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
      const store = await settingsStore;
      await store.set(STORE_KEY, newValue);
    } catch (error) {
      console.error("Failed to save settings:", error);
    }
  }, [autoCheck]);

  // Manual update check
  const handleCheckForUpdates = useCallback(async () => {
    clearResetTimer();
    setUpdateState(UPDATE_STATE.CHECKING);
    setUpdateInfo(null);

    try {
      const result = await checkForUpdate();

      if (result) {
        setUpdateInfo(result);
        setUpdateState(UPDATE_STATE.AVAILABLE);
      } else {
        setUpdateState(UPDATE_STATE.UP_TO_DATE);
        resetStatusLater();
      }
    } catch (error) {
      console.error("Failed to check for updates:", error);
      setUpdateState(UPDATE_STATE.CHECK_ERROR);
      resetStatusLater();
    }
  }, [clearResetTimer, resetStatusLater]);

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
      resetStatusLater();
    }
  }, [updateInfo, resetStatusLater]);

  const renderUpdateStatus = () => {
    switch (updateState) {
      case UPDATE_STATE.UP_TO_DATE:
        return (
          <span className="settings-panel__status settings-panel__status--success">
            {t("settings.upToDate")}
          </span>
        );
      case UPDATE_STATE.AVAILABLE:
        return (
          <span className="settings-panel__status settings-panel__status--available">
            {t("settings.versionAvailable", { version: updateInfo?.version })}
          </span>
        );
      case UPDATE_STATE.DOWNLOADING:
        return (
          <span className="settings-panel__status">
            {t("settings.installing", { percent: downloadProgress })}
          </span>
        );
      case UPDATE_STATE.CHECK_ERROR:
        return (
          <span className="settings-panel__status settings-panel__status--error">
            {t("settings.checkFailed")}
          </span>
        );
      case UPDATE_STATE.ERROR:
        return (
          <span className="settings-panel__status settings-panel__status--error">
            {t("settings.updateFailed")}
          </span>
        );
      default:
        return null;
    }
  };

  const getPrimaryAction = () => {
    if (updateState === UPDATE_STATE.AVAILABLE) {
      return {
        label: t("settings.installUpdate"),
        onClick: handleInstallUpdate,
        disabled: false,
      };
    }

    if (updateState === UPDATE_STATE.CHECKING) {
      return {
        label: t("settings.checking"),
        onClick: handleCheckForUpdates,
        disabled: true,
      };
    }

    if (updateState === UPDATE_STATE.DOWNLOADING) {
      return {
        label: t("settings.installing", { percent: downloadProgress }),
        onClick: handleInstallUpdate,
        disabled: true,
      };
    }

    return {
      label: t("settings.checkForUpdates"),
      onClick: handleCheckForUpdates,
      disabled: false,
    };
  };

  const primaryAction = getPrimaryAction();

  /** Render inline markdown (**bold**, `code`) as React elements. */
  const renderInline = (text) => {
    const parts = text.split(/(\*\*.+?\*\*|`.+?`)/g);
    return parts.map((part, i) => {
      if (part.startsWith("**") && part.endsWith("**")) {
        // biome-ignore lint/suspicious/noArrayIndexKey: parts of an immutable string
        return <strong key={i}>{part.slice(2, -2)}</strong>;
      }
      if (part.startsWith("`") && part.endsWith("`")) {
        // biome-ignore lint/suspicious/noArrayIndexKey: parts of an immutable string
        return <code key={i}>{part.slice(1, -1)}</code>;
      }
      return part;
    });
  };

  /** Parse a GitHub release markdown body into structured elements. */
  const renderChangelog = (body) => {
    const elements = [];
    let idx = 0;

    for (const line of body.split("\n")) {
      const trimmed = line.trim();
      if (!trimmed) continue;

      if (trimmed.startsWith("##")) {
        elements.push(
          <p key={`h${idx++}`} className="settings-panel__changelog-section">
            {trimmed.replace(/^#{1,4}\s*/, "")}
          </p>,
        );
      } else if (trimmed.startsWith("- ")) {
        elements.push(
          <p key={`i${idx++}`} className="settings-panel__changelog-item">
            {renderInline(trimmed.slice(2))}
          </p>,
        );
      } else {
        elements.push(
          <p key={`p${idx++}`} className="settings-panel__changelog-paragraph">
            {renderInline(trimmed)}
          </p>,
        );
      }
    }

    return elements;
  };

  return (
    <div className={`settings-panel ${show ? "settings-panel--open" : ""}`}>
      <div className="settings-panel__surface" inert={!show}>
        <div className="settings-panel__header">
          <span>{t("settings.title")}</span>
          <button
            type="button"
            className="settings-panel__close-btn"
            onClick={onClose}
            aria-label={t("settings.close")}
          >
            <img
              className="settings-panel__close-icon"
              src={closeIcon}
              alt=""
            />
          </button>
        </div>

        <div className="settings-panel__body">
          <span className="settings-panel__section-label">
            {t("settings.language")}
          </span>

          <div className="settings-panel__item">
            <select
              className="settings-panel__lang-select"
              value={language}
              onChange={(e) => setLanguage(e.target.value)}
            >
              {LANGUAGES.map((lang) => (
                <option key={lang.code} value={lang.code}>
                  {lang.label}
                </option>
              ))}
            </select>
          </div>

          <div className="settings-panel__divider" />

          <span className="settings-panel__section-label">
            {t("settings.updates")}
          </span>

          <div className="settings-panel__item settings-panel__item--toggle">
            <button
              type="button"
              className={`settings-panel__toggle ${autoCheck ? "settings-panel__toggle--on" : ""}`}
              onClick={handleToggle}
              role="switch"
              aria-checked={autoCheck}
              title={
                autoCheck
                  ? t("settings.disableAutoCheck")
                  : t("settings.enableAutoCheck")
              }
            >
              <span className="settings-panel__toggle-thumb" />
            </button>
            <span className="settings-panel__toggle-label">
              {t("settings.checkOnStartup")}
            </span>
          </div>

          <button
            type="button"
            className={`settings-panel__check-btn ${updateState === UPDATE_STATE.AVAILABLE ? "settings-panel__check-btn--available" : ""}`}
            onClick={primaryAction.onClick}
            disabled={primaryAction.disabled}
          >
            {primaryAction.label}
          </button>

          {updateState !== UPDATE_STATE.AVAILABLE && (
            <div className="settings-panel__status-row">
              {renderUpdateStatus()}
            </div>
          )}

          {updateState === UPDATE_STATE.AVAILABLE && updateInfo?.body && (
            <div className="settings-panel__changelog">
              <span className="settings-panel__changelog-title">
                {t("settings.changelog", { version: updateInfo.version })}
              </span>
              <div className="settings-panel__changelog-body">
                {renderChangelog(updateInfo.body)}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default SettingsPanel;
