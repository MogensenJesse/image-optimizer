// src/hooks/useOptimizer.js
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  useCallback,
  useEffect,
  useEffectEvent,
  useRef,
  useState,
} from "react";
import { useTranslation } from "../i18n";
import useProgressTracker from "./useProgressTracker";

export const SUPPORTED_EXTENSIONS = new Set([
  "jpg",
  "jpeg",
  "png",
  "webp",
  "avif",
  "svg",
]);

// Phases of one batch, in order: IDLE → FADE_IN → PROCESSING → COMPLETED →
// FADE_OUT → IDLE. DRAGGING is shown while files are dragged over the window.
export const APP_STATE = {
  IDLE: "idle",
  DRAGGING: "dragging",
  FADE_IN: "fadeIn",
  PROCESSING: "processing",
  COMPLETED: "completed",
  FADE_OUT: "fadeOut",
};

const FADE_IN_MS = 200;
const COMPLETED_MS = 3000;
const FADE_OUT_MS = 1200;

const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/**
 * Runs image optimization batches and owns everything around them: the
 * phase state machine, drag-and-drop listeners, progress and toasts.
 *
 * @param {Object} settings - Quality, resize and format settings for a batch
 */
function useOptimizer(settings) {
  const { t } = useTranslation();
  const [appState, setAppState] = useState(APP_STATE.IDLE);
  const [toast, setToast] = useState(null);
  const toastKeyRef = useRef(0);
  const { progress, start, finish, reset, processingRef } =
    useProgressTracker();

  // COMPLETED → FADE_OUT → IDLE
  useEffect(() => {
    if (appState === APP_STATE.COMPLETED) {
      const timer = setTimeout(
        () => setAppState(APP_STATE.FADE_OUT),
        COMPLETED_MS,
      );
      return () => clearTimeout(timer);
    }

    if (appState === APP_STATE.FADE_OUT) {
      const timer = setTimeout(() => {
        setAppState(APP_STATE.IDLE);
        processingRef.current = false;
      }, FADE_OUT_MS);
      return () => clearTimeout(timer);
    }
  }, [appState, processingRef]);

  const showToast = useCallback((message, type = "warning") => {
    toastKeyRef.current += 1;
    setToast({ message, type, key: toastKeyRef.current });
  }, []);

  const dismissToast = useCallback(() => setToast(null), []);

  const processFiles = useCallback(
    async (filePaths) => {
      if (processingRef.current || !filePaths || filePaths.length === 0) {
        return;
      }

      const supported = [];
      const skipped = [];

      for (const p of filePaths) {
        const ext = p.split(".").pop()?.toLowerCase();
        if (ext && SUPPORTED_EXTENSIONS.has(ext)) {
          supported.push(p);
        } else {
          skipped.push(p);
        }
      }

      if (skipped.length > 0) {
        const firstName = skipped[0].split(/[\\/]/).pop();
        const others = skipped.length - 1;
        let label;
        if (others === 0) {
          label = t("app.skippedFile", { name: firstName });
        } else if (others === 1) {
          label = t("app.skippedFilesOne", { name: firstName });
        } else {
          label = t("app.skippedFilesMany", { name: firstName, count: others });
        }
        showToast(label);
      }

      if (supported.length === 0) {
        // Nothing to do (e.g. only unsupported files dropped): leave the drag state
        setAppState((s) => (s === APP_STATE.DRAGGING ? APP_STATE.IDLE : s));
        return;
      }

      start(supported.length);
      processingRef.current = true;
      setAppState(APP_STATE.FADE_IN);

      // Let the progress bar fade in before it shows live progress
      const fadeIn = delay(FADE_IN_MS).then(() =>
        setAppState((s) =>
          s === APP_STATE.FADE_IN ? APP_STATE.PROCESSING : s,
        ),
      );

      try {
        const summary = await invoke("optimize_images", {
          paths: supported,
          settings,
        });
        await fadeIn;
        finish(summary);
        setAppState(APP_STATE.COMPLETED);

        if (summary.failed > 0) {
          const [{ fileName, error }] = summary.errors;
          showToast(
            summary.failed === 1
              ? t("app.failedFilesOne", { name: fileName, error })
              : t("app.failedFiles", {
                  name: fileName,
                  count: summary.failed - 1,
                }),
            "error",
          );
        }
      } catch (error) {
        console.error("Error processing images:", error);
        await fadeIn;
        reset();
        showToast(t("app.batchFailed", { error: String(error) }), "error");
        setAppState(APP_STATE.IDLE);
        processingRef.current = false;
      }
    },
    [settings, start, finish, reset, processingRef, showToast, t],
  );

  // Always runs the latest processFiles without re-registering the listeners
  const onDrop = useEffectEvent((paths) => processFiles(paths));

  // Drag-and-drop listeners, registered once
  useEffect(() => {
    const unlisteners = [
      listen("tauri://drag-drop", (event) => onDrop(event.payload.paths)),
      listen("tauri://drag-enter", () => {
        if (!processingRef.current) {
          setAppState(APP_STATE.DRAGGING);
        }
      }),
      listen("tauri://drag-leave", () =>
        setAppState((s) => (s === APP_STATE.DRAGGING ? APP_STATE.IDLE : s)),
      ),
    ];

    return () => {
      for (const unlisten of unlisteners) {
        unlisten.then((fn) => fn());
      }
    };
  }, [processingRef]);

  return { appState, progress, toast, dismissToast, processFiles };
}

export default useOptimizer;
