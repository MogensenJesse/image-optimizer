import React from "react";

/**
 * ProgressBar component for displaying image optimization progress
 *
 * @param {Object} props
 * @param {number} props.completedTasks - Number of completed tasks
 * @param {number} props.totalTasks - Total number of tasks
 * @param {number} props.progressPercentage - The calculated progress percentage (0-100)
 * @param {string} props.status - Current processing status ('idle', 'processing', 'complete', etc.)
 * @param {number} props.savedSize - Size saved in MB (optional)
 * @param {number} props.savedPercentage - Percentage of size saved (optional)
 */
function ProgressBar({
  completedTasks,
  totalTasks,
  progressPercentage,
  status,
  savedSize = 78.2, // Default values for demonstration
  savedPercentage = 73,
}) {
  // Calculate saved percentage for display
  const displayPercentage = Math.round(progressPercentage);
  const isComplete = displayPercentage >= 100;

  // Calculate the stroke-dasharray and stroke-dashoffset for the semi-circle
  const radius = 120;
  const circumference = radius * Math.PI;
  const dashOffset = circumference - (progressPercentage / 100) * circumference;

  // Gradient ID for the SVG
  const gradientId = "progressGradient";

  return (
    <div className="progress-circle">
      <svg className="progress-circle__svg" viewBox="0 0 250 150">
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
        <h2 className={`progress-circle__percentage-value ${isComplete ? 'complete' : ''}`}>
          {isComplete ? "Optimization complete" : `${displayPercentage}%`}
        </h2>
        <p className="progress-circle__percentage-label">
          {savedSize} MB / {savedPercentage}% saved
        </p>
      </div>
    </div>
  );
}

export default ProgressBar;
