// src/hooks/useProgressTracker.js
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";

const IDLE = {
  completed: 0,
  total: 0,
  failed: 0,
  savedBytes: 0,
  originalBytes: 0,
  startTime: null,
  durationMs: null,
};

/**
 * Tracks progress of an image optimization batch.
 *
 * The backend emits `image_optimization_progress` events carrying running
 * totals (`completed`, `total`, `failed`, `savedBytes`, `originalBytes`), so
 * the hook just stores the latest one. `finish` stores the final summary the
 * `optimize_images` command resolves with, which also carries `durationMs`.
 *
 * @returns {{
 *   progress: Object,
 *   start: (total: number) => void,
 *   finish: (summary: Object) => void,
 *   reset: () => void,
 *   processingRef: React.MutableRefObject<boolean>,
 * }}
 */
function useProgressTracker() {
  const processingRef = useRef(false);
  const [progress, setProgress] = useState(IDLE);

  // Single event listener — registered once on mount
  useEffect(() => {
    const unsubscribe = listen("image_optimization_progress", (event) => {
      if (!processingRef.current) return;
      setProgress((prev) => ({ ...prev, ...event.payload }));
    });

    return () => {
      unsubscribe.then((fn) => fn());
    };
  }, []);

  /** Reset all tracking state for a batch of `total` files. */
  const start = useCallback((total) => {
    setProgress({ ...IDLE, total, startTime: Date.now() });
  }, []);

  /** Store the final summary returned by the backend. */
  const finish = useCallback((summary) => {
    setProgress((prev) => ({ ...prev, ...summary }));
  }, []);

  const reset = useCallback(() => setProgress(IDLE), []);

  return { progress, start, finish, reset, processingRef };
}

export default useProgressTracker;
