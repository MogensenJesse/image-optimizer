import { useEffect, useState } from "react";
import { useTranslation } from "../i18n";

const TICK_MS = 100;

/**
 * Seconds elapsed since `startTime`, live until the backend reports the final
 * `durationMs`. The ticking lives here so only this component re-renders.
 */
function useElapsedSeconds(startTime, durationMs) {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (durationMs != null) return;
    const timer = setInterval(() => setNow(Date.now()), TICK_MS);
    return () => clearInterval(timer);
  }, [durationMs]);

  if (durationMs != null) return durationMs / 1000;
  return startTime ? Math.max(0, (now - startTime) / 1000) : 0;
}

/**
 * ProgressBar component for displaying image optimization progress
 *
 * @param {Object} props
 * @param {number} props.completed - Files finished so far (failed ones included)
 * @param {number} props.total - Files in the batch
 * @param {number} props.failed - Files that could not be optimized
 * @param {number} props.savedBytes - Bytes saved so far
 * @param {number} props.originalBytes - Original size of the files counted in savedBytes
 * @param {number|null} props.startTime - Epoch ms when the batch started
 * @param {number|null} props.durationMs - Final duration, once the batch is done
 */
function ProgressBar({
  completed,
  total,
  failed = 0,
  savedBytes = 0,
  originalBytes = 0,
  startTime,
  durationMs,
}) {
  const { t } = useTranslation();
  const progressPercentage = total > 0 ? (completed / total) * 100 : 0;
  const displayPercentage = Math.round(progressPercentage);
  const progressLabel = t("progress.aria", { percent: displayPercentage });
  const isComplete = displayPercentage >= 100;
  const savedPercentage =
    originalBytes > 0 ? Math.round((savedBytes / originalBytes) * 100) : 0;
  const optimizedCount = completed - failed;
  const processingTime = useElapsedSeconds(startTime, durationMs);

  // Calculate the stroke-dasharray and stroke-dashoffset for the semi-circle
  const radius = 100;
  const circumference = radius * Math.PI;
  const dashOffset = circumference - (progressPercentage / 100) * circumference;

  // Gradient ID for the SVG
  const gradientId = "progressGradient";

  const formatFileSize = (bytes) => {
    if (bytes < 1024) return t("progress.sizeB", { value: bytes });
    if (bytes < 1024 * 1024)
      return t("progress.sizeKB", { value: (bytes / 1024).toFixed(1) });
    return t("progress.sizeMB", { value: (bytes / (1024 * 1024)).toFixed(2) });
  };

  const formatTime = (seconds) => {
    if (seconds < 60) {
      return t("progress.seconds", { value: seconds.toFixed(1) });
    }
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = (seconds % 60).toFixed(1);
    return t("progress.minutes", { min: minutes, sec: remainingSeconds });
  };

  return (
    <div className="progress-circle">
      <svg
        className="progress-circle__svg"
        viewBox="0 0 250 150"
        aria-label={progressLabel}
      >
        <title>{progressLabel}</title>
        {/* Define the gradient */}
        <defs>
          <linearGradient id={gradientId} x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stopColor="#FFD700" /> {/* Gold */}
            <stop offset="50%" stopColor="#FFA500" /> {/* Orange */}
            <stop offset="100%" stopColor="#FF4500" /> {/* Red-orange */}
          </linearGradient>
        </defs>

        {/* Background semi-circle */}
        <path
          className="progress-circle__background"
          d="M25,125 a100,100 0 0,1 200,0"
          fill="none"
          strokeWidth="2"
        />

        {/* Progress semi-circle with gradient */}
        <path
          className="progress-circle__progress"
          d="M25,125 a100,100 0 0,1 200,0"
          fill="none"
          strokeWidth="2"
          style={{
            strokeDasharray: circumference,
            strokeDashoffset: dashOffset,
            stroke: `url(#${gradientId})`,
          }}
        />
      </svg>

      {/* Percentage display in the center */}
      <div className="progress-circle__percentage">
        <h2
          className={`progress-circle__percentage-value ${isComplete ? "complete" : ""}`}
        >
          {isComplete ? t("progress.complete") : `${displayPercentage}%`}
        </h2>
        <p className="progress-circle__percentage-label">
          {t("progress.saved", {
            size: formatFileSize(savedBytes),
            percent: savedPercentage,
          })}
        </p>

        <p className="progress-circle__percentage-label">
          {t(
            optimizedCount === 1 ? "progress.summaryOne" : "progress.summary",
            {
              count: optimizedCount,
              time: formatTime(processingTime),
            },
          )}
        </p>
      </div>
    </div>
  );
}

export default ProgressBar;
