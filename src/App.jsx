// src/App.jsx
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { basename, dirname, join } from "@tauri-apps/api/path";
import { open } from "@tauri-apps/plugin-dialog";
import { mkdir } from "@tauri-apps/plugin-fs";
import { platform as getPlatform } from "@tauri-apps/plugin-os";
import { load } from "@tauri-apps/plugin-store";
import { useCallback, useEffect, useRef, useState } from "react";
import optionsIcon from "./assets/icons/options.svg";
import plusIcon from "./assets/icons/plus.svg";
import FloatingMenu from "./components/FloatingMenu";
import ProgressBar from "./components/ProgressBar";
import SettingsPanel from "./components/SettingsPanel";
import TitleBar from "./components/TitleBar";
import Toast from "./components/Toast";
import useProgressTracker from "./hooks/useProgressTracker";
import { useTranslation } from "./i18n";
import { checkForUpdate } from "./utils/updater";

const SUPPORTED_EXTENSIONS = new Set(["jpg", "jpeg", "png", "webp", "avif"]);

// Define app states as constants
const APP_STATE = {
  IDLE: "idle",
  DRAGGING: "dragging",
  FADE_IN: "fadeIn",
  PROCESSING: "processing",
  COMPLETED: "completed",
  FADE_OUT: "fadeOut",
};

function App() {
  // Main application state
  const [appState, setAppState] = useState(APP_STATE.IDLE);
  const [showMenu, setShowMenu] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [hasUpdate, setHasUpdate] = useState(false);
  const [platformName, setPlatformName] = useState(null);
  const [toast, setToast] = useState(null);
  const toastTimerRef = useRef(null);
  const { t } = useTranslation();

  // Use our custom hook for progress tracking
  const { progress, initProgress, processingRef } = useProgressTracker(
    appState === APP_STATE.PROCESSING,
  );

  const [settings, setSettings] = useState({
    quality: {
      global: 90,
      jpeg: null,
      png: null,
      webp: null,
      avif: null,
    },
    resize: {
      width: null,
      height: null,
      maintainAspect: true,
      mode: "none",
      size: null,
    },
    outputFormat: "original",
  });

  // Effect to handle PROCESSING to COMPLETED transition
  useEffect(() => {
    // Only run this effect when processing is complete
    if (
      appState === APP_STATE.PROCESSING &&
      progress.progressPercentage === 100 &&
      progress.completedTasks > 0
    ) {
      setAppState(APP_STATE.COMPLETED);
    }
  }, [appState, progress.progressPercentage, progress.completedTasks]);

  // Effect to handle COMPLETED to FADE_OUT transition
  useEffect(() => {
    let completionTimer = null;

    if (appState === APP_STATE.COMPLETED) {
      completionTimer = setTimeout(() => {
        setAppState(APP_STATE.FADE_OUT);
      }, 3000);
    }

    return () => {
      if (completionTimer) {
        clearTimeout(completionTimer);
      }
    };
  }, [appState]);

  // Effect to handle FADE_OUT to IDLE transition
  useEffect(() => {
    let fadeOutTimer = null;

    if (appState === APP_STATE.FADE_OUT) {
      fadeOutTimer = setTimeout(() => {
        setAppState(APP_STATE.IDLE);
        processingRef.current = false;
      }, 1200);
    }

    return () => {
      if (fadeOutTimer) {
        clearTimeout(fadeOutTimer);
      }
    };
  }, [appState, processingRef]);

  const handleSettingsChange = (newSettings) => {
    setSettings(newSettings);
  };

  const toggleMenu = () => {
    setShowMenu(!showMenu);
  };

  const toggleSettings = () => {
    setShowSettings((prev) => !prev);
  };

  const isLinuxPlatform = platformName === "linux";

  // Detect platform once on mount
  useEffect(() => {
    let isMounted = true;
    (async () => {
      try {
        const detectedPlatform = await getPlatform();
        if (isMounted) {
          setPlatformName(detectedPlatform);
        }
      } catch (error) {
        console.error("Failed to detect platform:", error);
      }
    })();

    return () => {
      isMounted = false;
    };
  }, []);

  // Auto-check for updates on startup (if enabled in settings)
  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const store = await load("settings.json", { autoSave: true });
        const autoCheckEnabled = await store.get("autoCheckUpdates");

        // Default to true if the key doesn't exist yet
        if (cancelled || autoCheckEnabled === false) return;

        const update = await checkForUpdate();
        if (!cancelled && update) {
          setHasUpdate(true);
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

  // Toggle Linux-specific body class for styling fallbacks
  useEffect(() => {
    const className = "platform-linux";
    if (isLinuxPlatform) {
      document.body.classList.add(className);
    } else {
      document.body.classList.remove(className);
    }

    return () => {
      document.body.classList.remove(className);
    };
  }, [isLinuxPlatform]);

  const showToast = useCallback((message, type = "warning") => {
    if (toastTimerRef.current) clearTimeout(toastTimerRef.current);
    setToast({ message, type });
    toastTimerRef.current = setTimeout(() => setToast(null), 5000);
  }, []);

  const processFiles = useCallback(
    async (filePaths) => {
      if (processingRef.current || !filePaths || filePaths.length === 0) {
        return;
      }

      const supported = [];
      const skipped = [];

      for (const p of filePaths) {
        const ext = p.split(".").pop()?.toLowerCase();
        if (ext && SUPPORTED_EXTENSIONS.has(ext)) {
          supported.push(p);
        } else {
          skipped.push(p);
        }
      }

      if (skipped.length > 0) {
        const names = skipped.map((p) => p.split(/[\\/]/).pop());
        const label =
          skipped.length === 1
            ? t("app.skippedFile", { name: names[0] })
            : t("app.skippedFiles", {
                count: skipped.length,
                names: names.join(", "),
              });
        showToast(label);
      }

      if (supported.length === 0) return;

      initProgress(supported.length);
      processingRef.current = true;

      let animationDone = false;

      try {
        setAppState(APP_STATE.FADE_IN);

        const optimizationPromise = (async () => {
          await Promise.all(
            supported.map(async (path) => {
              const parentDir = await dirname(path);
              const optimizedPath = await join(parentDir, "optimized");
              await mkdir(optimizedPath, { recursive: true });
            }),
          );

          const tasks = await Promise.all(
            supported.map(async (path) => {
              const parentDir = await dirname(path);
              const fileName = await basename(path);
              const optimizedPath = await join(
                parentDir,
                "optimized",
                fileName,
              );
              return [path, optimizedPath, settings];
            }),
          );

          return invoke("optimize_images", { tasks });
        })();

        const animationPromise = (async () => {
          await new Promise((resolve) => setTimeout(resolve, 50));
          await new Promise((resolve) => setTimeout(resolve, 150));
          animationDone = true;
          setAppState(APP_STATE.PROCESSING);
        })();

        await Promise.all([optimizationPromise, animationPromise]);
      } catch (error) {
        console.error("Error processing images:", error);
        if (animationDone) {
          setAppState(APP_STATE.IDLE);
        } else {
          // Animation hasn't fired yet; defer reset so the late
          // setAppState(PROCESSING) doesn't overwrite us.
          setTimeout(() => setAppState(APP_STATE.IDLE), 300);
        }
        processingRef.current = false;
      }
    },
    [settings, initProgress, processingRef, showToast, t],
  );

  // Handle click on dropzone to open file picker
  const handleDropzoneClick = async () => {
    if (processingRef.current) {
      return;
    }

    try {
      // Open file picker dialog
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
        // Process the selected files
        await processFiles(Array.isArray(selected) ? selected : [selected]);
      }
    } catch (error) {
      console.error("Error opening file picker:", error);
    }
  };

  useEffect(() => {
    // Drag-drop event listener
    const unsubscribeDrop = listen("tauri://drag-drop", async (event) => {
      // Get the number of files dropped
      const droppedFiles = event.payload.paths;
      await processFiles(droppedFiles);
    });

    // Drag enter handler
    const unsubscribeEnter = listen("tauri://drag-enter", () => {
      if (!processingRef.current) {
        setAppState(APP_STATE.DRAGGING);
      }
    });

    // Drag leave handler
    const unsubscribeLeave = listen("tauri://drag-leave", () => {
      if (appState === APP_STATE.DRAGGING) {
        setAppState(APP_STATE.IDLE);
      }
    });

    return () => {
      unsubscribeDrop.then((fn) => fn());
      unsubscribeEnter.then((fn) => fn());
      unsubscribeLeave.then((fn) => fn());
    };
  }, [appState, processFiles, processingRef.current]);

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

  return (
    <div className="container">
      <TitleBar onSettingsToggle={toggleSettings} hasUpdate={hasUpdate} />
      <div className="app-content">
        <div className="workspace">
          <div className="dropzone-area">
            {/* biome-ignore lint/a11y/useSemanticElements: Dropzone needs to be a div for drag-and-drop styling */}
            <div
              className={`dropzone 
                ${appState === APP_STATE.DRAGGING ? "dropzone--dragging" : ""} 
                ${showProgressBar ? "dropzone--processing" : ""}
                ${appState === APP_STATE.FADE_OUT ? "dropzone--fading" : ""}
                ${appState === APP_STATE.FADE_IN ? "dropzone--fading-in" : ""}`}
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
                      completedTasks={progress.completedTasks}
                      totalTasks={progress.totalTasks}
                      progressPercentage={progress.progressPercentage}
                      savedSize={progress.savedSize}
                      savedPercentage={progress.savedPercentage}
                      processingTime={progress.processingTime}
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
              onClick={toggleMenu}
              disabled={
                appState !== APP_STATE.IDLE && appState !== APP_STATE.DRAGGING
              }
            >
              <img src={optionsIcon} alt="" />
            </button>

            <FloatingMenu
              settings={settings}
              onSettingsChange={handleSettingsChange}
              disabled={
                appState !== APP_STATE.IDLE && appState !== APP_STATE.DRAGGING
              }
              show={showMenu}
              onClose={() => setShowMenu(false)}
            />
          </div>

          <SettingsPanel
            show={showSettings}
            onClose={() => setShowSettings(false)}
          />
        </div>
      </div>

      {toast && (
        <Toast
          message={toast.message}
          type={toast.type}
          onClose={() => setToast(null)}
        />
      )}
    </div>
  );
}

export default App;
