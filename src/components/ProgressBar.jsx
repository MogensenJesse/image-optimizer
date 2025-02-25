import React from 'react';

/**
 * ProgressBar component for displaying image optimization progress
 * 
 * @param {Object} props
 * @param {number} props.completedTasks - Number of completed tasks
 * @param {number} props.totalTasks - Total number of tasks
 * @param {number} props.progressPercentage - The calculated progress percentage (0-100)
 * @param {string} props.status - Current processing status ('idle', 'processing', 'complete', etc.)
 */
function ProgressBar({ completedTasks, totalTasks, progressPercentage, status }) {
  return (
    <div className="progress-info">
      <p className="progress-info__text">
        {totalTasks > 0 
          ? `${completedTasks} of ${totalTasks} images optimized` 
          : 'Preparing images...'}
      </p>
      
      <div className="progress-bar">
        <div 
          className="progress-bar__fill" 
          style={{ width: `${progressPercentage}%` }}
        ></div>
      </div>
      
      <p className="progress-info__percentage">{progressPercentage}% complete</p>
    </div>
  );
}

export default ProgressBar; 