// src/App.jsx
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import optionsIcon from "./assets/icons/options.svg";
import plusIcon from "./assets/icons/plus.svg";
import FloatingMenu from "./components/FloatingMenu";
import ProgressBar from "./components/ProgressBar";
import SettingsPanel from "./components/SettingsPanel";
import TitleBar from "./components/TitleBar";
import Toast from "./components/Toast";
import useOptimizer, {
  APP_STATE,
  SUPPORTED_EXTENSIONS,
} from "./hooks/useOptimizer";
import { useTranslation } from "./i18n";
import { settingsStore } from "./utils/store";
import { checkForUpdate } from "./utils/updater";

const DEFAULT_SETTINGS = {
  quality: 90,
  resize: { mode: "none", size: null },
  outputFormat: "original",
};

function App() {
  const { t } = useTranslation();
  const [showMenu, setShowMenu] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [startupUpdate, setStartupUpdate] = useState(null);
  const [settings, setSettings] = useState(DEFAULT_SETTINGS);

  const { appState, progress, toast, dismissToast, processFiles } =
    useOptimizer(settings);

  // Settings can only be changed while no batch is running
  const busy = appState !== APP_STATE.IDLE && appState !== APP_STATE.DRAGGING;

  // Auto-check for updates on startup (if enabled in settings)
  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const store = await settingsStore;
        const autoCheckEnabled = await store.get("autoCheckUpdates");

        // Default to true if the key doesn't exist yet
        if (cancelled || autoCheckEnabled === false) return;

        const update = await checkForUpdate();
        if (!cancelled && update) {
          setStartupUpdate(update);
        }
      } catch (error) {
        // Silently fail -- startup check is non-critical
        console.error("Auto update check failed:", error);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  // Handle click on dropzone to open file picker
  const handleDropzoneClick = async () => {
    if (busy) {
      return;
    }

    try {
      const selected = await open({
        multiple: true,
        filters: [
          {
            name: t("app.fileFilter"),
            extensions: [...SUPPORTED_EXTENSIONS],
          },
        ],
      });

      if (selected) {
        await processFiles(Array.isArray(selected) ? selected : [selected]);
      }
    } catch (error) {
      console.error("Error opening file picker:", error);
    }
  };

  // Determine if we should show the progress bar
  const showProgressBar = [
    APP_STATE.FADE_IN,
    APP_STATE.PROCESSING,
    APP_STATE.COMPLETED,
    APP_STATE.FADE_OUT,
  ].includes(appState);

  // Determine if we should show the dropzone message
  // Always show dropzone in IDLE, DRAGGING, and during FADE_OUT for smooth transitions
  const showDropzone = [
    APP_STATE.IDLE,
    APP_STATE.DRAGGING,
    APP_STATE.FADE_OUT,
  ].includes(appState);

  // CSS classes based on state
  const getProgressClasses = () => {
    if (appState === APP_STATE.FADE_IN) return "fade-in";
    if (appState === APP_STATE.FADE_OUT) return "fade-out";
    return "";
  };

  const dropzoneClasses = [
    "dropzone",
    appState === APP_STATE.DRAGGING && "dropzone--dragging",
    showProgressBar && "dropzone--processing",
    appState === APP_STATE.FADE_OUT && "dropzone--fading",
    appState === APP_STATE.FADE_IN && "dropzone--fading-in",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div className="container">
      <TitleBar
        onSettingsToggle={() => setShowSettings((show) => !show)}
        hasUpdate={startupUpdate !== null}
      />
      <div className="app-content">
        <div className="workspace">
          <div className="dropzone-area">
            {/* biome-ignore lint/a11y/useSemanticElements: Dropzone needs to be a div for drag-and-drop styling */}
            <div
              className={dropzoneClasses}
              onClick={handleDropzoneClick}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  handleDropzoneClick();
                }
              }}
              role="button"
              tabIndex={0}
              aria-label={t("app.dropzone.aria")}
            >
              <div className="dropzone__content">
                {showProgressBar && (
                  <div className={`progress-container ${getProgressClasses()}`}>
                    <ProgressBar
                      completed={progress.completed}
                      total={progress.total}
                      failed={progress.failed}
                      savedBytes={progress.savedBytes}
                      originalBytes={progress.originalBytes}
                      startTime={progress.startTime}
                      durationMs={progress.durationMs}
                    />
                  </div>
                )}

                {showDropzone && (
                  <div
                    className={`dropzone__message ${appState === APP_STATE.FADE_OUT ? "fade-in-delayed" : ""}`}
                  >
                    <img src={plusIcon} alt="" className="dropzone__icon" />
                    <h2>{t("app.dropzone.title")}</h2>
                    <p>{t("app.dropzone.subtitle")}</p>
                  </div>
                )}
              </div>
            </div>

            <button
              type="button"
              className="options-button"
              onClick={() => setShowMenu((show) => !show)}
              disabled={busy}
            >
              <img src={optionsIcon} alt="" />
              <span className="options-button__label">{t("app.options")}</span>
            </button>

            <FloatingMenu
              settings={settings}
              onSettingsChange={setSettings}
              show={showMenu}
              onClose={() => setShowMenu(false)}
            />
          </div>

          <SettingsPanel
            show={showSettings}
            onClose={() => setShowSettings(false)}
            startupUpdate={startupUpdate}
          />
        </div>
      </div>

      {toast && (
        <Toast
          key={toast.key}
          message={toast.message}
          type={toast.type}
          onClose={dismissToast}
        />
      )}
    </div>
  );
}

export default App;
