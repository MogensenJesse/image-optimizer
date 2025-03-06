import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event';
import { dirname, join } from '@tauri-apps/api/path';
import { mkdir } from "@tauri-apps/plugin-fs";
import FloatingMenu from "./components/FloatingMenu";
import ProgressBar from "./components/ProgressBar";
import TitleBar from "./components/TitleBar";
import useProgressTracker from "./hooks/useProgressTracker";
import plusIcon from "./assets/icons/plus.svg";
import optionsIcon from "./assets/icons/options.svg";

// Define app states as constants
const APP_STATE = {
  IDLE: 'idle',
  DRAGGING: 'dragging',
  FADE_IN: 'fadeIn',
  PROCESSING: 'processing',
  COMPLETED: 'completed',
  FADE_OUT: 'fadeOut',
};

function App() {
  // Main application state
  const [appState, setAppState] = useState(APP_STATE.IDLE);
  const [showMenu, setShowMenu] = useState(false);
  
  // Use our custom hook for progress tracking
  const { progress, initProgress, processingRef } = useProgressTracker(appState === APP_STATE.PROCESSING);
  
  const [settings, setSettings] = useState({
    quality: {
      global: 90,
      jpeg: null,
      png: null,
      webp: null,
      avif: null
    },
    resize: {
      width: null,
      height: null,
      maintainAspect: true,
      mode: 'none',
      size: null
    },
    outputFormat: 'original'
  });

  // Effect to handle PROCESSING to COMPLETED transition
  useEffect(() => {
    // Only run this effect when processing is complete
    if (appState === APP_STATE.PROCESSING && progress.progressPercentage === 100 && progress.completedTasks > 0) {
      setAppState(APP_STATE.COMPLETED);
    }
  }, [appState, progress.progressPercentage, progress.completedTasks]);
  
  // Effect to handle COMPLETED to FADE_OUT transition
  useEffect(() => {
    let completionTimer = null;
    
    if (appState === APP_STATE.COMPLETED) {
      completionTimer = setTimeout(() => {
        setAppState(APP_STATE.FADE_OUT);
      }, 1500);
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

  useEffect(() => {
    // Drag-drop event listener 
    const unsubscribeDrop = listen("tauri://drag-drop", async (event) => {
      if (processingRef.current) {
        return;
      }
      
      // Get the number of files dropped
      const droppedFiles = event.payload.paths;
      const fileCount = droppedFiles ? droppedFiles.length : 0;
      
      if (!droppedFiles || droppedFiles.length === 0) {
        return;
      }
      
      initProgress(fileCount);
      
      // Set processing flag AFTER initializing progress
      processingRef.current = true;

      try {
        // Start the fade in animation
        setAppState(APP_STATE.FADE_IN);
        
        // Wait for animation to start
        await new Promise(resolve => setTimeout(resolve, 50));
        
        // After a short delay, start processing
        await new Promise(resolve => setTimeout(resolve, 500));
        setAppState(APP_STATE.PROCESSING);
        
        // Create all required directories first
        await Promise.all(droppedFiles.map(async (path) => {
          const parentDir = await dirname(path);
          const optimizedPath = await join(parentDir, 'optimized');
          await mkdir(optimizedPath, { recursive: true });
        }));

        // Create batch tasks
        const tasks = await Promise.all(droppedFiles.map(async (path) => {
          const parentDir = await dirname(path);
          const fileName = path.split('\\').pop();
          const optimizedPath = await join(parentDir, 'optimized', fileName);
          return [path, optimizedPath, settings];
        }));

        // Process batch - this will trigger progress events
        await invoke('optimize_images', { tasks });
        
        // If we reach here, the invoke call completed but our state transitions
        // will be handled by the useEffect hooks that watch for progress changes
      } catch (error) {
        console.error('Error processing images:', error);
        // Reset to idle state in case of error
        setAppState(APP_STATE.IDLE);
        processingRef.current = false;
      }
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
      unsubscribeDrop.then(fn => fn());
      unsubscribeEnter.then(fn => fn());
      unsubscribeLeave.then(fn => fn());
    };
  }, [settings, initProgress, appState]);

  // Determine if we should show the progress bar
  const showProgressBar = [
    APP_STATE.FADE_IN, 
    APP_STATE.PROCESSING, 
    APP_STATE.COMPLETED, 
    APP_STATE.FADE_OUT
  ].includes(appState);
  
  // Determine if we should show the dropzone message
  // Always show dropzone in IDLE, DRAGGING, and during FADE_OUT for smooth transitions
  const showDropzone = [
    APP_STATE.IDLE,
    APP_STATE.DRAGGING,
    APP_STATE.FADE_OUT
  ].includes(appState);
  
  // CSS classes based on state
  const getProgressClasses = () => {
    if (appState === APP_STATE.FADE_IN) return 'fade-in';
    if (appState === APP_STATE.FADE_OUT) return 'fade-out';
    return '';
  };

  return (
    <div className="container">
      <TitleBar />
      <div className="app-content">
        <div 
          className={`dropzone 
            ${appState === APP_STATE.DRAGGING ? 'dropzone--dragging' : ''} 
            ${showProgressBar ? 'dropzone--processing' : ''}
            ${appState === APP_STATE.FADE_OUT ? 'dropzone--fading' : ''}
            ${appState === APP_STATE.FADE_IN ? 'dropzone--fading-in' : ''}`}
        >
          <div className="dropzone__content">
            {showProgressBar && (
              <div className={`progress-container ${getProgressClasses()}`}>
                <ProgressBar
                  completedTasks={progress.completedTasks}
                  totalTasks={progress.totalTasks}
                  progressPercentage={progress.progressPercentage}
                  status={progress.status}
                  savedSize={progress.savedSize}
                  savedPercentage={progress.savedPercentage}
                  lastOptimizedFile={progress.lastOptimizedFile}
                />
              </div>
            )}
            
            {showDropzone && (
              <div className={`dropzone__message ${appState === APP_STATE.FADE_OUT ? 'fade-in-delayed' : ''}`}>
                <img src={plusIcon} alt="Drop here" className="dropzone__icon" />
                <h2>Drop images here</h2>
                <p>Optimized images will be saved in their source folder</p>
              </div>
            )}
          </div>
        </div>

        <button 
          className="options-button" 
          onClick={toggleMenu} 
          disabled={appState !== APP_STATE.IDLE && appState !== APP_STATE.DRAGGING}
        >
          <img src={optionsIcon} alt="Options" />
        </button>

        <FloatingMenu 
          settings={settings}
          onSettingsChange={handleSettingsChange}
          disabled={appState !== APP_STATE.IDLE && appState !== APP_STATE.DRAGGING}
          show={showMenu}
          onClose={() => setShowMenu(false)}
        />
      </div>
    </div>
  );
}

export default App;