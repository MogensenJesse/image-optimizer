import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event';
import { dirname, join } from '@tauri-apps/api/path';
import { mkdir } from "@tauri-apps/plugin-fs";
import FloatingMenu from "./components/FloatingMenu";
import ProgressBar from "./components/ProgressBar";
import useProgressTracker from "./hooks/useProgressTracker";

function App() {
  const [isProcessing, setIsProcessing] = useState(false);
  const [isDragging, setIsDragging] = useState(false);
  
  // Use our custom hook for progress tracking
  const { progress, initProgress, processingRef } = useProgressTracker(isProcessing);
  
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

  const handleSettingsChange = (newSettings) => {
    setSettings(newSettings);
  };

  useEffect(() => {
    // Drag-drop event listener 
    const unsubscribeDrop = listen("tauri://drag-drop", async (event) => {
      if (processingRef.current) return;
      processingRef.current = true;

      setIsDragging(false);
      
      // Get the number of files dropped
      const droppedFiles = event.payload.paths;
      const fileCount = droppedFiles ? droppedFiles.length : 0;
      
      // Initialize progress tracking with the number of files dropped
      initProgress(fileCount);

      if (droppedFiles && droppedFiles.length > 0) {
        try {
          // Set processing state immediately before any async operations
          setIsProcessing(true);
          
          // Small delay to ensure state updates before processing begins
          await new Promise(resolve => setTimeout(resolve, 100));
          
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

          // Process batch
          await invoke('optimize_images', { tasks });

        } catch (error) {
          console.error('Error processing images:', error);
        } finally {
          setIsProcessing(false);
          processingRef.current = false;
        }
      }
    });

    // Drag enter handler
    const unsubscribeEnter = listen("tauri://drag-enter", () => {
      setIsDragging(true);
    });

    // Drag leave handler
    const unsubscribeLeave = listen("tauri://drag-leave", () => {
      setIsDragging(false);
    });

    return () => {
      unsubscribeDrop.then(fn => fn());
      unsubscribeEnter.then(fn => fn());
      unsubscribeLeave.then(fn => fn());
    };
  }, [settings, initProgress]);

  return (
    <>
      <div 
        className={`dropzone ${isDragging ? 'dropzone--dragging' : ''} 
          ${isProcessing ? 'dropzone--processing' : ''}`}
      >
        <div className="dropzone__content">
          {isProcessing ? (
            <div className="processing-info">
              <h2 className="processing-info__title">Processing...</h2>
              
              <ProgressBar
                completedTasks={progress.completedTasks}
                totalTasks={progress.totalTasks}
                progressPercentage={progress.progressPercentage}
                status={progress.status}
              />
            </div>
          ) : (
            <div className="dropzone__message">
              <h2>Drop images here</h2>
              <p>Supported formats: JPEG, PNG, WebP, AVIF</p>
            </div>
          )}
        </div>
      </div>

      <FloatingMenu 
        settings={settings}
        onSettingsChange={handleSettingsChange}
        disabled={isProcessing}
      />
    </>
  );
}

export default App;