import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { basename, dirname, join } from "@tauri-apps/api/path";
import { open } from "@tauri-apps/plugin-dialog";
import { mkdir } from "@tauri-apps/plugin-fs";
import { useEffect, useState } from "react";
import optionsIcon from "./assets/icons/options.svg";
import plusIcon from "./assets/icons/plus.svg";
import FloatingMenu from "./components/FloatingMenu";
import ProgressBar from "./components/ProgressBar";
import TitleBar from "./components/TitleBar";
import useProgressTracker from "./hooks/useProgressTracker";

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
  }, [appState]);

  const handleSettingsChange = (newSettings) => {
    setSettings(newSettings);
  };

  const toggleMenu = () => {
    setShowMenu(!showMenu);
  };

  // Function to handle file processing (used for both drop and click)
  const processFiles = async (filePaths) => {
    if (processingRef.current || !filePaths || filePaths.length === 0) {
      return;
    }

    initProgress(filePaths.length);
    processingRef.current = true;

    try {
      // Start UI animations immediately
      setAppState(APP_STATE.FADE_IN);

      // Start optimization process in parallel with UI animations
      const optimizationPromise = (async () => {
        // Create all required directories first
        await Promise.all(
          filePaths.map(async (path) => {
            const parentDir = await dirname(path);
            const optimizedPath = await join(parentDir, "optimized");
            await mkdir(optimizedPath, { recursive: true });
          }),
        );

        // Create batch tasks
        const tasks = await Promise.all(
          filePaths.map(async (path) => {
            const parentDir = await dirname(path);
            const fileName = await basename(path);
            const optimizedPath = await join(parentDir, "optimized", fileName);
            return [path, optimizedPath, settings];
          }),
        );

        // Start the actual optimization
        return invoke("optimize_images", { tasks });
      })();

      // Handle UI animations independently
      const animationPromise = (async () => {
        await new Promise((resolve) => setTimeout(resolve, 50));
        await new Promise((resolve) => setTimeout(resolve, 150));
        setAppState(APP_STATE.PROCESSING);
      })();

      // Wait for both to complete, but optimization can start before animations finish
      await Promise.all([optimizationPromise, animationPromise]);
    } catch (error) {
      console.error("Error processing images:", error);
      setAppState(APP_STATE.IDLE);
      processingRef.current = false;
    }
  };

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
            name: "Images",
            extensions: ["png", "jpg", "jpeg", "webp", "avif", "gif"],
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
  }, [settings, initProgress, appState]);

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
      <TitleBar />
      <div className="app-content">
        <div
          className={`dropzone 
            ${appState === APP_STATE.DRAGGING ? "dropzone--dragging" : ""} 
            ${showProgressBar ? "dropzone--processing" : ""}
            ${appState === APP_STATE.FADE_OUT ? "dropzone--fading" : ""}
            ${appState === APP_STATE.FADE_IN ? "dropzone--fading-in" : ""}`}
          onClick={handleDropzoneClick}
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
                <img
                  src={plusIcon}
                  alt="Drop here"
                  className="dropzone__icon"
                />
                <h2>Drop images here</h2>
                <p>Optimized images will be saved in their source folder</p>
              </div>
            )}
          </div>
        </div>

        <button
          className="options-button"
          onClick={toggleMenu}
          disabled={
            appState !== APP_STATE.IDLE && appState !== APP_STATE.DRAGGING
          }
        >
          <img src={optionsIcon} alt="Options" />
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
    </div>
  );
}

export default App;
