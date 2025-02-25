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
  const [progress, setProgress] = useState({
    completedTasks: 0,
    totalTasks: 0,
    progressPercentage: 0,
    status: 'idle',
    lastUpdated: Date.now()
  });
  
  // Add refs to track cumulative progress across batches
  const batchProgressRef = useRef({
    totalImages: 0,         // This will be set to the total number of dropped images
    processedImages: 0,     // This will track how many images we've processed across all batches
    currentBatchId: null,   // Identifier for the current batch being processed
    lastCompletedInBatch: 0, // How many were completed in the current batch
    lastStatus: null,       // Status of the last update
    batchCount: 0,          // How many batches we've seen
    knownTotalImages: null  // Initial count from the drag-drop event (critical for smooth progress)
  });
  
  // Debug processing state changes
  useEffect(() => {
    console.log('isProcessing changed:', isProcessing);
  }, [isProcessing]);
  
  // Debug progress state changes
  useEffect(() => {
    console.log('Progress state updated:', progress);
  }, [progress]);
  
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
    // Listen for progress updates from backend
    const unsubscribeProgress = listen('image_optimization_progress', (event) => {
      // Add debug logging
      console.log('Progress event received:', event.payload);
      
      // Update cumulative progress
      const { completedTasks, totalTasks, status } = event.payload;
      
      // Use processingRef instead of isProcessing to avoid state timing issues
      if (processingRef.current) {
        console.log('Processing is active, updating progress');
        const currentBatch = batchProgressRef.current;
        
        // Detect a new batch in any of these cases:
        // 1. Previous status was 'complete' and now we're processing again
        // 2. completedTasks suddenly dropped to a low number after being high
        // 3. totalTasks changed from previous batch
        const previouslyComplete = currentBatch.lastStatus === 'complete' && status === 'processing';
        const taskCountReset = currentBatch.lastCompletedInBatch > completedTasks && completedTasks <= 20;
        const taskSizeChanged = currentBatch.currentBatchId !== null && currentBatch.currentBatchId !== `${totalTasks}`;
        
        if (!currentBatch.currentBatchId || previouslyComplete || taskCountReset || taskSizeChanged) {
          // This is a new batch - increment batch count and update the batch ID
          currentBatch.batchCount++;
          console.log(`New batch #${currentBatch.batchCount} detected:`, totalTasks);
          
          currentBatch.currentBatchId = `${totalTasks}`;
          
          // ONLY update totalImages if we didn't already know how many files were dropped
          // This ensures we maintain the original total from the drop event
          if (!currentBatch.knownTotalImages) {
            // If this is the first batch, set the total
            if (currentBatch.batchCount === 1) {
              currentBatch.totalImages = totalTasks;
            } else {
              // For additional batches, add to the running total if we don't know the final count
              currentBatch.totalImages += totalTasks;
            }
          }
          
          currentBatch.lastCompletedInBatch = 0;
          console.log(`Batch #${currentBatch.batchCount}: ${totalTasks} images of total ${currentBatch.totalImages || currentBatch.knownTotalImages}`);
        }
        
        // Update the last status
        currentBatch.lastStatus = status;
        
        // Calculate how many new tasks were completed in this update
        const newlyCompleted = Math.max(0, completedTasks - currentBatch.lastCompletedInBatch);
        currentBatch.lastCompletedInBatch = completedTasks;
        
        // Update processed count by adding new completions
        currentBatch.processedImages += newlyCompleted;
        
        // The total to use for percentage calculation is either the known total from drop event
        // or the running total we've calculated from batches
        const totalForCalculation = currentBatch.knownTotalImages || currentBatch.totalImages;
        
        // Ensure we don't exceed the total
        currentBatch.processedImages = Math.min(currentBatch.processedImages, totalForCalculation);
        
        // Calculate percentage based on overall progress - use the KNOWN total from drop event if available
        const overallPercentage = totalForCalculation > 0 
          ? Math.floor((currentBatch.processedImages / totalForCalculation) * 100)
          : 0;
        
        console.log('Updating progress:', {
          completedTasks: currentBatch.processedImages,
          totalTasks: totalForCalculation,
          progressPercentage: overallPercentage,
          newlyCompleted,
          batchId: currentBatch.currentBatchId,
          batchCount: currentBatch.batchCount
        });
        
        // Update the progress state with cumulative values
        setProgress({
          completedTasks: currentBatch.processedImages,
          totalTasks: totalForCalculation,
          progressPercentage: overallPercentage,
          status: status,
          lastUpdated: Date.now()
        });
      } else {
        console.log('Not processing, ignoring progress event');
      }
    });

    const unsubscribeDrop = listen("tauri://drag-drop", async (event) => {
      if (processingRef.current) return;
      processingRef.current = true;

      setIsDragging(false);
      setOptimizationResults([]);
      
      // Get the number of files dropped
      const droppedFiles = event.payload.paths;
      const fileCount = droppedFiles ? droppedFiles.length : 0;
      console.log(`User dropped ${fileCount} files - this is our baseline for progress calculation`);
      
      // Reset progress tracking when starting a new optimization
      setProgress({
        completedTasks: 0,
        totalTasks: fileCount, // Set the initial total to the number of files dropped
        progressPercentage: 0,
        status: 'idle',
        lastUpdated: Date.now()
      });
      
      // Reset batch progress ref with additional tracking properties
      batchProgressRef.current = {
        totalImages: 0,            // This will be calculated from batches if needed
        processedImages: 0,        // Start with 0 processed
        currentBatchId: null,      // No current batch yet
        lastCompletedInBatch: 0,   // No completed tasks yet
        lastStatus: null,          // No status yet
        batchCount: 0,             // No batches yet
        knownTotalImages: fileCount // CRITICAL: Store the actual count from the drop event
      };

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
      unsubscribeProgress.then(fn => fn());
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
              
              <div className="progress-info">
                <p>
                  {progress.totalTasks > 0 
                    ? `${progress.completedTasks} of ${progress.totalTasks} images optimized` 
                    : 'Preparing images...'}
                </p>
                
                <div className="progress-bar" style={{ 
                  height: '16px', 
                  background: 'rgba(50, 50, 50, 0.3)',
                  borderRadius: '8px',
                  margin: '15px 0',
                  overflow: 'hidden'
                }}>
                  <div 
                    className="progress-bar__fill" 
                    style={{ 
                      width: `${progress.progressPercentage}%`,
                      height: '100%',
                      background: '#4caf50',
                      // Slow down the transition to make updates more visible
                      transition: 'width 0.5s ease-in-out' 
                    }}
                  ></div>
                </div>
                
                <p style={{ fontWeight: 'bold' }}>{progress.progressPercentage}% complete</p>
              </div>
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