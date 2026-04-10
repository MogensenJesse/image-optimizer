import { useTranslation } from "../i18n";

/**
 * ProgressBar component for displaying image optimization progress
 *
 * @param {Object} props
 * @param {number} props.completedTasks - Number of completed tasks
 * @param {number} props.totalTasks - Total number of tasks
 * @param {number} props.progressPercentage - The calculated progress percentage (0-100)
 * @param {number} props.savedSize - Size saved in bytes (optional)
 * @param {number} props.savedPercentage - Percentage of size saved (optional)
 * @param {number} props.processingTime - Time elapsed in seconds since processing started
 */
function ProgressBar({
  completedTasks,
  totalTasks: _totalTasks,
  progressPercentage,
  savedSize = 0,
  savedPercentage = 0,
  processingTime = 0,
}) {
  const { t } = useTranslation();
  const displayPercentage = Math.round(progressPercentage);
  const isComplete = displayPercentage >= 100;

  // Calculate the stroke-dasharray and stroke-dashoffset for the semi-circle
  const radius = 100;
  const circumference = radius * Math.PI;
  const dashOffset = circumference - (progressPercentage / 100) * circumference;

  // Gradient ID for the SVG
  const gradientId = "progressGradient";

  const formatFileSize = (bytes) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
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
        aria-label={`Progress: ${displayPercentage}%`}
      >
        <title>Progress indicator: {displayPercentage}% complete</title>
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
            size: formatFileSize(savedSize),
            percent: savedPercentage,
          })}
        </p>

        <p className="progress-circle__percentage-label">
          {t("progress.summary", {
            count: completedTasks,
            time: formatTime(processingTime),
          })}
        </p>
      </div>
    </div>
  );
}

export default ProgressBar;
