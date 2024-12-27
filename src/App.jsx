import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event';
import { dirname, join } from '@tauri-apps/api/path';
import { mkdir } from "@tauri-apps/plugin-fs";
import "./reset.scss";
import "./App.scss";
import FloatingMenu from "./components/FloatingMenu";

function formatSize(bytes) {
  const absBytes = Math.abs(bytes);
  if (absBytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  } else {
    return `${(bytes / 1024).toFixed(2)} KB`;
  }
}

function App() {
  const [isProcessing, setIsProcessing] = useState(false);
  const [isDragging, setIsDragging] = useState(false);
  const [optimizationStats, setOptimizationStats] = useState({
    totalFiles: 0,
    processedFiles: 0,
    elapsedTime: 0
  });
  const [optimizationResults, setOptimizationResults] = useState([]);
  const processingRef = useRef(false);
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
    console.log('Settings updated:', newSettings); // Debug log
    setSettings(newSettings);
  };

  useEffect(() => {
    const unsubscribeDrop = listen("tauri://drag-drop", async (event) => {
      if (processingRef.current) return;
      processingRef.current = true;

      setIsDragging(false);
      setOptimizationResults([]);

      const paths = event.payload.paths;
      if (paths && paths.length > 0) {
        try {
          setIsProcessing(true);
          const batchStartTime = performance.now();
          
          setOptimizationStats({
            totalFiles: paths.length,
            processedFiles: 0,
            elapsedTime: 0
          });

          // Create all required directories first
          await Promise.all(paths.map(async (path) => {
            const parentDir = await dirname(path);
            const optimizedPath = await join(parentDir, 'optimized');
            await mkdir(optimizedPath, { recursive: true });
          }));

          // Process images sequentially for accurate timing
          for (const path of paths) {
            try {
              const parentDir = await dirname(path);
              const fileName = path.split('\\').pop();
              const optimizedPath = await join(parentDir, 'optimized', fileName);
              
              console.log('Processing with current settings:', settings); // Debug log
              
              const result = await invoke('optimize_image', { 
                inputPath: path, 
                outputPath: optimizedPath,
                settings: settings  // This will now use the current settings
              });

              setOptimizationResults(prev => [...prev, result]);
              setOptimizationStats(prev => ({
                ...prev,
                processedFiles: prev.processedFiles + 1,
                elapsedTime: ((performance.now() - batchStartTime) / 1000).toFixed(2)
              }));
            } catch (error) {
              console.error(`Error processing ${path}:`, error);
            }
          }

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
      console.log('Drag enter');
      setIsDragging(true);
    });

    // Drag leave handler
    const unsubscribeLeave = listen("tauri://drag-leave", () => {
      console.log('Drag leave');
      setIsDragging(false);
    });

    return () => {
      unsubscribeDrop.then(fn => fn());
      unsubscribeEnter.then(fn => fn());
      unsubscribeLeave.then(fn => fn());
    };
  }, [settings]);

  return (
    <>
      <div 
        className={`dropzone ${isDragging ? 'dropzone--dragging' : ''} ${isProcessing ? 'dropzone--processing' : ''}`}
      >
        <div className="dropzone__content">
          {isProcessing ? (
            <div className="processing-info">
              <h2 className="processing-info__title">Processing...</h2>
              <p>Processed {optimizationStats.processedFiles} of {optimizationStats.totalFiles} files</p>
              <p>Time elapsed: {optimizationStats.elapsedTime}s</p>
            </div>
          ) : optimizationResults.length > 0 ? (
            <div className="processing-info">
              <h2>Optimization Complete</h2>
              <p>{optimizationResults.length} files processed in {optimizationStats.elapsedTime}s</p>
              <p>Total size reduction: {formatSize(optimizationResults.reduce((total, result) => 
                total + result.savedBytes, 0))}</p>
              <p>Average compression: {(optimizationResults.reduce((total, result) => 
                total + parseFloat(result.compressionRatio), 0) / optimizationResults.length).toFixed(2)}%</p>
            </div>
          ) : (
            <p>Drop images here</p>
          )}
        </div>
      </div>
      <FloatingMenu 
        settings={settings}
        onSettingsChange={handleSettingsChange}
      />
    </>
  );
}

export default App;