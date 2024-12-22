import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Command } from "@tauri-apps/plugin-shell";
import { listen } from '@tauri-apps/api/event';
import { dirname, join } from '@tauri-apps/api/path';
import { mkdir } from "@tauri-apps/plugin-fs";
import "./App.css";

function App() {
  const [isProcessing, setIsProcessing] = useState(false);
  const [isDragging, setIsDragging] = useState(false);
  const [optimizationStats, setOptimizationStats] = useState({
    totalFiles: 0,
    processedFiles: 0,
    elapsedTime: 0
  });

  useEffect(() => {
    const unlisten = listen("tauri://drag-enter", () => {
      setIsDragging(true);
    });

    const unlistenCancel = listen("tauri://drag-leave", () => {
      setIsDragging(false);
    });

    const unlistenDrop = listen("tauri://drag-drop", async (event) => {
      setIsDragging(false);
      const paths = event.payload.paths;

      if (paths && paths.length > 0) {
        try {
          setIsProcessing(true);
          const startTime = performance.now();
          let processedFiles = 0;

          // Create all required directories first
          await Promise.all(paths.map(async (path) => {
            const parentDir = await dirname(path);
            const optimizedPath = await join(parentDir, 'optimized');
            await mkdir(optimizedPath, { recursive: true });
          }));

          // Process images in parallel with Promise.all
          await Promise.all(paths.map(async (path) => {
            try {
              const parentDir = await dirname(path);
              const optimizedPath = await join(parentDir, 'optimized', path.split('\\').pop());
              await invoke('optimize_image', { inputPath: path, outputPath: optimizedPath });
              processedFiles++;
              
              setOptimizationStats(prev => ({
                totalFiles: paths.length,
                processedFiles,
                elapsedTime: ((performance.now() - startTime) / 1000).toFixed(2)
              }));
            } catch (error) {
              console.error('Error processing image:', error);
            }
          }));

        } catch (error) {
          console.error('Error processing images:', error);
        } finally {
          setIsProcessing(false);
        }
      }
    });

    return () => {
      unlisten;
      unlistenCancel;
      unlistenDrop;
    };
  }, []);

  return (
    <div 
      className={`dropzone ${isDragging ? 'dragging' : ''} ${isProcessing ? 'processing' : ''}`}
    >
      {isProcessing ? (
        <div className="processing-info">
          <h2>Processing Images...</h2>
          <p>Processed {optimizationStats.processedFiles} of {optimizationStats.totalFiles} files</p>
          <p>Time elapsed: {optimizationStats.elapsedTime}s</p>
        </div>
      ) : (
        <div className="dropzone-content">
          <h2>Drop images here</h2>
          <p>Images will be optimized and saved in an 'optimized' folder</p>
        </div>
      )}
    </div>
  );
}

export default App;
