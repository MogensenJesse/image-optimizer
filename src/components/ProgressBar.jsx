import React from 'react';

function ProgressBar({ progress }) {
  if (!progress) return null;
  
  const percentage = Math.round((progress.processed_files / progress.total_files) * 100);
  
  return (
    <div className="progress-container">
      <div className="progress-bar">
        <div className="progress-bar__fill" style={{ width: `${percentage}%` }} />
      </div>
      <div className="progress-stats">
        {progress.processed_files} / {progress.total_files} files
        {progress.failed_tasks.length > 0 && (
          <span className="progress-stats__failed">
            ({progress.failed_tasks.length} failed)
          </span>
        )}
      </div>
    </div>
  );
}

export default ProgressBar; 