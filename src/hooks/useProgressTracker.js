import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";

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
    status: "idle",
    lastUpdated: Date.now(),
    savedSize: 0, // Size saved in MB
    savedPercentage: 0, // Percentage of size saved
    currentFile: null, // Currently processing file
    lastOptimizedFile: null, // Last optimized file with metrics
    startTime: null, // Start time of the processing
    processingTime: 0, // Total processing time in seconds
  });

  // Ref to track cumulative progress across batches
  const batchProgressRef = useRef({
    totalImages: 0, // Total number of dropped images
    processedImages: 0, // Number of images processed across all batches
    currentBatchId: null, // Identifier for the current batch being processed
    lastCompletedInBatch: 0, // How many were completed in the current batch
    lastStatus: null, // Status of the last update
    batchCount: 0, // How many batches we've seen
    knownTotalImages: null, // Initial count from the drag-drop event

    // Statistics tracking (based on actual results)
    totalSavedBytes: 0, // Total bytes saved
    totalOriginalSize: 0, // Total original size in bytes
    lastOptimizedFile: null, // Last optimized file with metrics

    // Keep track of the last 5 optimized files for UI display
    recentOptimizations: [],

    // Timing information
    startTime: null, // When processing started
    processingTime: 0, // Processing time in seconds
  });

  // Update processingRef when isProcessing changes
  useEffect(() => {
    processingRef.current = isProcessing;
  }, [isProcessing]);

  // Effect to update processing time while processing is active
  useEffect(() => {
    let timer = null;

    if (processingRef.current && batchProgressRef.current.startTime) {
      // Update processing time every 100ms for more precision
      timer = setInterval(() => {
        const currentTime = Date.now();
        const elapsedSeconds =
          (currentTime - batchProgressRef.current.startTime) / 1000;

        // Update both the ref and the state in one go
        batchProgressRef.current.processingTime = elapsedSeconds;

        // Use functional update to ensure we're using the latest state
        setProgress((prevProgress) => ({
          ...prevProgress,
          processingTime: elapsedSeconds,
        }));
      }, 100); // Update more frequently for smoother time display
    }

    return () => {
      if (timer) {
        clearInterval(timer);
      }
    };
  }, [isProcessing]);

  useEffect(() => {
    // Listen for progress updates from backend
    const unsubscribeProgress = listen(
      "image_optimization_progress",
      (event) => {
        // Check if this is a detailed update with file-specific metrics
        const isDetailedUpdate =
          event.payload.metadata && event.payload.metadata.detailedUpdate;

        // Use processingRef instead of isProcessing to avoid state timing issues
        if (processingRef.current) {
          const currentBatch = batchProgressRef.current;

          if (isDetailedUpdate) {
            // This is a detailed update with file-specific metrics
            const { fileName } = event.payload.metadata;
            const result = event.payload.result;

            if (result) {
              // Update the statistics with actual data
              currentBatch.totalSavedBytes += result.saved_bytes;
              currentBatch.totalOriginalSize += result.original_size;

              // Create an optimization record
              const optimizationRecord = {
                fileName,
                originalSize: result.original_size,
                optimizedSize: result.optimized_size,
                savedBytes: result.saved_bytes,
                compressionRatio: parseFloat(result.compression_ratio),
                timestamp: Date.now(),
              };

              // Update the last optimized file
              currentBatch.lastOptimizedFile = optimizationRecord;

              // Add to recent optimizations (keep only the last 5)
              currentBatch.recentOptimizations.unshift(optimizationRecord);
              if (currentBatch.recentOptimizations.length > 5) {
                currentBatch.recentOptimizations.pop();
              }

              // Calculate overall saved percentage
              const savedPercentage =
                currentBatch.totalOriginalSize > 0
                  ? Math.round(
                      (currentBatch.totalSavedBytes /
                        currentBatch.totalOriginalSize) *
                        100,
                    )
                  : 0;

              // Calculate saved size in MB
              const savedSizeMB = currentBatch.totalSavedBytes / (1024 * 1024);

              // Update the progress state with the latest metrics
              setProgress((prevProgress) => ({
                ...prevProgress,
                savedSize: parseFloat(savedSizeMB.toFixed(1)),
                savedPercentage,
                lastOptimizedFile: optimizationRecord,
                processingTime: currentBatch.processingTime,
              }));
            }
          } else {
            // This is a regular progress update
            const { completedTasks, totalTasks, status, metadata } =
              event.payload;

            // Check if the metadata contains a formatted message with saved bytes and compression ratio
            if (metadata && metadata.formattedMessage) {
              const formattedMessage = metadata.formattedMessage;

              // Try to extract saved bytes and compression ratio from the message
              // Example format: "IMG_20240406_180359.jpg optimized (630.78 KB saved / 65.86% compression) - Progress: 25% (1/4)"
              const savedBytesMatch = formattedMessage.match(
                /\(([\d.]+) (KB|MB|GB) saved/i,
              );
              const compressionRatioMatch = formattedMessage.match(
                /\/ ([\d.]+)% compression/i,
              );

              if (savedBytesMatch && compressionRatioMatch) {
                const savedAmount = parseFloat(savedBytesMatch[1]);
                const savedUnit = savedBytesMatch[2].toUpperCase();
                const compressionRatio = parseFloat(compressionRatioMatch[1]);

                // Convert saved amount to bytes based on the unit
                let savedBytes = 0;
                if (savedUnit === "KB") {
                  savedBytes = savedAmount * 1024;
                } else if (savedUnit === "MB") {
                  savedBytes = savedAmount * 1024 * 1024;
                } else if (savedUnit === "GB") {
                  savedBytes = savedAmount * 1024 * 1024 * 1024;
                }

                // Update our statistics with the extracted data
                currentBatch.totalSavedBytes += savedBytes;

                // We don't have original size from the message, but we can estimate it
                // using the compression ratio: originalSize = savedBytes / (compressionRatio / 100)
                const estimatedOriginalSize =
                  savedBytes / (compressionRatio / 100);
                currentBatch.totalOriginalSize += estimatedOriginalSize;

                // Calculate updated metrics
                const savedSizeMB =
                  currentBatch.totalSavedBytes / (1024 * 1024);
                const savedPercentage =
                  currentBatch.totalOriginalSize > 0
                    ? Math.round(
                        (currentBatch.totalSavedBytes /
                          currentBatch.totalOriginalSize) *
                          100,
                      )
                    : 0;

                // Create an optimization record for the current file
                if (metadata.fileName) {
                  const optimizationRecord = {
                    fileName: metadata.fileName,
                    savedBytes,
                    compressionRatio,
                    timestamp: Date.now(),
                  };

                  // Update the last optimized file
                  currentBatch.lastOptimizedFile = optimizationRecord;
                }
              }
            }

            // Detect a new batch in any of these cases:
            // 1. Previous status was 'complete' and now we're processing again
            // 2. completedTasks suddenly dropped to a low number after being high
            // 3. totalTasks changed from previous batch
            const previouslyComplete =
              currentBatch.lastStatus === "complete" && status === "processing";
            const taskCountReset =
              currentBatch.lastCompletedInBatch > completedTasks &&
              completedTasks <= 20;
            const taskSizeChanged =
              currentBatch.currentBatchId !== null &&
              currentBatch.currentBatchId !== `${totalTasks}`;

            if (
              !currentBatch.currentBatchId ||
              previouslyComplete ||
              taskCountReset ||
              taskSizeChanged
            ) {
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
            const newlyCompleted = Math.max(
              0,
              completedTasks - currentBatch.lastCompletedInBatch,
            );
            currentBatch.lastCompletedInBatch = completedTasks;

            // Update processed count by adding new completions
            currentBatch.processedImages += newlyCompleted;

            // The total to use for percentage calculation is either the known total from drop event
            // or the running total we've calculated from batches
            const totalForCalculation =
              currentBatch.knownTotalImages || currentBatch.totalImages;

            // Ensure we don't exceed the total
            currentBatch.processedImages = Math.min(
              currentBatch.processedImages,
              totalForCalculation,
            );

            // Calculate percentage based on overall progress - use the KNOWN total from drop event if available
            const overallPercentage =
              totalForCalculation > 0
                ? Math.floor(
                    (currentBatch.processedImages / totalForCalculation) * 100,
                  )
                : 0;

            // Calculate saved size in MB
            const savedSizeMB = currentBatch.totalSavedBytes / (1024 * 1024);

            // Calculate overall saved percentage
            const savedPercentage =
              currentBatch.totalOriginalSize > 0
                ? Math.round(
                    (currentBatch.totalSavedBytes /
                      currentBatch.totalOriginalSize) *
                      100,
                  )
                : 0;

            // Update the progress state with cumulative values
            setProgress((prevProgress) => ({
              ...prevProgress,
              completedTasks: currentBatch.processedImages,
              totalTasks: totalForCalculation,
              progressPercentage: overallPercentage,
              status: status,
              lastUpdated: Date.now(),
              // Update savedSize and savedPercentage values
              savedSize: parseFloat(savedSizeMB.toFixed(1)),
              savedPercentage,
              lastOptimizedFile: currentBatch.lastOptimizedFile,
              processingTime: currentBatch.processingTime,
            }));
          }
        }

        // If this is the final completion event with total duration from backend
        if (
          event.payload.status === "complete" &&
          event.payload.metadata &&
          event.payload.metadata.totalDuration
        ) {
          // Use the backend's reported duration for accuracy
          const backendDuration = parseFloat(
            event.payload.metadata.totalDuration,
          );

          batchProgressRef.current.processingTime = backendDuration;
          setProgress((prevProgress) => ({
            ...prevProgress,
            processingTime: backendDuration,
          }));
        }
      },
    );

    return () => {
      unsubscribeProgress.then((fn) => fn());
    };
  }, []);

  /**
   * Initialize progress tracking with the number of files dropped
   * @param {number} fileCount - Number of files dropped
   */
  const initProgress = (fileCount) => {
    const startTime = Date.now();

    // Reset progress tracking when starting a new optimization
    setProgress({
      completedTasks: 0,
      totalTasks: fileCount,
      progressPercentage: 0,
      status: "idle",
      lastUpdated: Date.now(),
      savedSize: 0,
      savedPercentage: 0,
      currentFile: null,
      lastOptimizedFile: null,
      startTime,
      processingTime: 0,
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

      // Reset statistics tracking
      totalSavedBytes: 0,
      totalOriginalSize: 0,
      lastOptimizedFile: null,
      recentOptimizations: [],

      // Set start time for processing
      startTime,
      processingTime: 0,
    };
  };

  return {
    progress,
    initProgress,
    processingRef,
  };
}

export default useProgressTracker;
