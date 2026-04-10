// src/hooks/useProgressTracker.js
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";

const TIMER_INTERVAL_MS = 100;

/**
 * Tracks progress of image optimization.
 *
 * The backend emits `image_optimization_progress` events with overall
 * (not per-chunk) `completedTasks` / `totalTasks` counts, so this hook
 * simply forwards those values into React state.
 *
 * @returns {{ progress: Object, initProgress: Function, processingRef: React.MutableRefObject<boolean> }}
 */
function useProgressTracker() {
  const processingRef = useRef(false);
  const [progress, setProgress] = useState({
    completedTasks: 0,
    totalTasks: 0,
    progressPercentage: 0,
    status: "idle",
    savedSize: 0,
    savedPercentage: 0,
    processingTime: 0,
  });

  const statsRef = useRef({
    totalSavedBytes: 0,
    totalOriginalSize: 0,
    startTime: null,
  });

  const timerRef = useRef(null);

  // Elapsed-time ticker — runs from initProgress until the final event
  // arrives, independent of the FADE_IN / PROCESSING state machine.
  const startTimer = () => {
    stopTimer();
    timerRef.current = setInterval(() => {
      const { startTime } = statsRef.current;
      if (!startTime) return;
      const elapsed = (Date.now() - startTime) / 1000;
      setProgress((prev) => ({ ...prev, processingTime: elapsed }));
    }, TIMER_INTERVAL_MS);
  };

  const stopTimer = () => {
    if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }
  };

  // Clean up timer on unmount
  useEffect(() => stopTimer, []);

  // Single event listener — registered once on mount
  useEffect(() => {
    const unsubscribe = listen("image_optimization_progress", (event) => {
      if (!processingRef.current) return;

      const { completedTasks, totalTasks, progressPercentage, status, metadata } =
        event.payload;

      const stats = statsRef.current;

      // Accumulate per-image size stats when present
      if (metadata?.savedBytes != null && metadata?.originalSize != null) {
        stats.totalSavedBytes += Number(metadata.savedBytes);
        stats.totalOriginalSize += Number(metadata.originalSize);
      }

      const savedSizeMB = stats.totalSavedBytes / (1024 * 1024);
      const savedPercentage =
        stats.totalOriginalSize > 0
          ? Math.round((stats.totalSavedBytes / stats.totalOriginalSize) * 100)
          : 0;

      // Use the backend's wall-clock duration on the final event
      let processingTime = stats.startTime
        ? (Date.now() - stats.startTime) / 1000
        : 0;

      if (status === "complete" && metadata?.totalDuration) {
        processingTime = parseFloat(metadata.totalDuration);
        stopTimer();
      }

      setProgress({
        completedTasks,
        totalTasks,
        progressPercentage,
        status,
        savedSize: parseFloat(savedSizeMB.toFixed(1)),
        savedPercentage,
        processingTime,
      });
    });

    return () => {
      unsubscribe.then((fn) => fn());
    };
  }, []);

  /**
   * Reset all tracking state and start the elapsed-time timer.
   * Called by App before invoking the backend command.
   *
   * @param {number} fileCount - Number of files to process
   */
  const initProgress = (fileCount) => {
    statsRef.current = {
      totalSavedBytes: 0,
      totalOriginalSize: 0,
      startTime: Date.now(),
    };

    setProgress({
      completedTasks: 0,
      totalTasks: fileCount,
      progressPercentage: 0,
      status: "idle",
      savedSize: 0,
      savedPercentage: 0,
      processingTime: 0,
    });

    startTimer();
  };

  return { progress, initProgress, processingRef };
}

export default useProgressTracker;
