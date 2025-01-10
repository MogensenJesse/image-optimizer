import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event';
import { dirname, join } from '@tauri-apps/api/path';
import { mkdir } from "@tauri-apps/plugin-fs";
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
          
          // Create all required directories first
          await Promise.all(paths.map(async (path) => {
            const parentDir = await dirname(path);
            const optimizedPath = await join(parentDir, 'optimized');
            await mkdir(optimizedPath, { recursive: true });
          }));

          // Create batch tasks
          const tasks = await Promise.all(paths.map(async (path) => {
            const parentDir = await dirname(path);
            const fileName = path.split('\\').pop();
            const optimizedPath = await join(parentDir, 'optimized', fileName);
            return [path, optimizedPath, settings];
          }));

          // Process batch
          const results = await invoke('optimize_images', { tasks });
          setOptimizationResults(results);

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
  }, [settings]);

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

      {optimizationResults.length > 0 && (
        <div className="results">
          <h2>Results</h2>
          <div className="results__grid">
            {optimizationResults.map((result, index) => (
              <div key={index} className="result-card">
                <div className="result-card__path">{result.original_path}</div>
                <div className="result-card__stats">
                  <div>Original: {formatSize(result.original_size || 0)}</div>
                  <div>Optimized: {formatSize(result.optimized_size || 0)}</div>
                  <div>Saved: {formatSize(result.saved_bytes || 0)}</div>
                  <div>Ratio: {(result.compression_ratio || 0).toFixed(1)}%</div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </>
  );
}

export default App;