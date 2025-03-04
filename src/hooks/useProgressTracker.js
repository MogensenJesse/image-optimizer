import { useState, useRef, useEffect } from "react";
import { listen } from '@tauri-apps/api/event';

/**
 * Custom hook to track progress of image optimization across multiple batches
 * 
 * @param {boolean} isProcessing - Whether the application is currently processing images
 * @returns {Object} Progress state and functions to control progress
 */
function useProgressTracker(isProcessing) {
  const processingRef = useRef(false);
  const [progress, setProgress] = useState({
    completedTasks: 0,
    totalTasks: 0,
    progressPercentage: 0,
    status: 'idle',
    lastUpdated: Date.now(),
    savedSize: 0,          // Size saved in MB
    savedPercentage: 0     // Percentage of size saved
  });
  
  // Ref to track cumulative progress across batches
  const batchProgressRef = useRef({
    totalImages: 0,         // Total number of dropped images
    processedImages: 0,     // Number of images processed across all batches
    currentBatchId: null,   // Identifier for the current batch being processed
    lastCompletedInBatch: 0, // How many were completed in the current batch
    lastStatus: null,       // Status of the last update
    batchCount: 0,          // How many batches we've seen
    knownTotalImages: null,  // Initial count from the drag-drop event
    
    // Statistics tracking (estimated based on current progress)
    averageCompressionRatio: 0.65, // Assume 65% average compression
    totalSavedSize: 0,      // Total saved size in MB (simulated)
    savedPercentage: 0      // Percentage saved (simulated)
  });
  
  // Update processingRef when isProcessing changes
  useEffect(() => {
    processingRef.current = isProcessing;
  }, [isProcessing]);
  
  useEffect(() => {
    // Listen for progress updates from backend
    const unsubscribeProgress = listen('image_optimization_progress', (event) => {
      // Update cumulative progress
      const { completedTasks, totalTasks, status } = event.payload;
      
      // Use processingRef instead of isProcessing to avoid state timing issues
      if (processingRef.current) {
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
        
        // Calculate estimated saved size and percentage based on progress
        // Use a simple model: as progress increases, the saved statistics increase
        // Start with low values and reach full estimated values at 100%
        const progressRatio = overallPercentage / 100;
        const estimatedTotalSizeMB = totalForCalculation * 2; // Assume average 2MB per image
        const estimatedSavedSizeMB = estimatedTotalSizeMB * currentBatch.averageCompressionRatio * progressRatio;
        const estimatedSavedPercentage = Math.round(currentBatch.averageCompressionRatio * 100 * progressRatio);
        
        // Update the progress state with cumulative values
        setProgress({
          completedTasks: currentBatch.processedImages,
          totalTasks: totalForCalculation,
          progressPercentage: overallPercentage,
          status: status,
          lastUpdated: Date.now(),
          savedSize: parseFloat(estimatedSavedSizeMB.toFixed(1)),
          savedPercentage: estimatedSavedPercentage
        });
      }
    });

    return () => {
      unsubscribeProgress.then(fn => fn());
    };
  }, []);
  
  /**
   * Initialize progress tracking with the number of files dropped
   * @param {number} fileCount - Number of files dropped
   */
  const initProgress = (fileCount) => {
    // Reset progress tracking when starting a new optimization
    setProgress({
      completedTasks: 0,
      totalTasks: fileCount,
      progressPercentage: 0,
      status: 'idle',
      lastUpdated: Date.now(),
      savedSize: 0,
      savedPercentage: 0
    });
    
    // Reset batch progress ref with additional tracking properties
    batchProgressRef.current = {
      totalImages: 0,
      processedImages: 0,
      currentBatchId: null,
      lastCompletedInBatch: 0,
      lastStatus: null,
      batchCount: 0,
      knownTotalImages: fileCount,
      
      // Initialize statistics tracking with reasonable defaults
      averageCompressionRatio: 0.65, // Assume 65% average compression
      totalSavedSize: 0,
      savedPercentage: 0
    };
  };
  
  return {
    progress,
    initProgress,
    processingRef
  };
}

export default useProgressTracker; 