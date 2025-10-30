const { parentPort } = require("node:worker_threads");

/**
 * Progress message handling utilities
 * @module utils/progress
 */

/**
 * Progress message types
 * @enum {string}
 */
const ProgressType = {
  START: "start",
  PROGRESS: "progress",
  COMPLETE: "complete",
  ERROR: "error",
};

/**
 * Creates a progress message object
 * @param {string} type - Progress type from ProgressType enum
 * @param {Object} data - Progress data
 * @returns {Object} Formatted progress message
 */
function createProgressMessage(type, data) {
  return {
    type: "progress",
    progressType: type,
    ...data,
  };
}

/**
 * Creates a start progress message
 * @param {string} taskId - Task identifier (usually input path)
 * @param {Object} metadata - Additional metadata
 * @returns {Object} Start progress message
 */
function createStartMessage(taskId, metadata = {}) {
  return createProgressMessage(ProgressType.START, {
    taskId,
    workerId: metadata.workerId,
    fileName: metadata.fileName || require("node:path").basename(taskId),
  });
}

/**
 * Creates a completion progress message
 * @param {string} taskId - Task identifier (usually input path)
 * @param {Object} result - Task result data
 * @returns {Object} Completion progress message
 */
function createCompleteMessage(taskId, result) {
  const fileName = result.fileName || require("node:path").basename(taskId);

  return createProgressMessage(ProgressType.COMPLETE, {
    taskId,
    fileName,
    workerId: result.workerId,
    result,
    optimizationMessage:
      result.optimizationMessage ||
      `${fileName} optimized (${result.compression_ratio}% reduction)`,
  });
}

/**
 * Creates an error progress message
 * @param {string} taskId - Task identifier (usually input path)
 * @param {Error|string} error - Error object or message
 * @param {Object} metadata - Additional metadata
 * @returns {Object} Error progress message
 */
function createErrorMessage(taskId, error, metadata = {}) {
  return createProgressMessage(ProgressType.ERROR, {
    taskId,
    fileName: metadata.fileName || require("node:path").basename(taskId),
    error: error instanceof Error ? error.message : error,
  });
}

/**
 * Creates a unified progress update message
 * @param {number} completedTasks - Number of completed tasks
 * @param {number} totalTasks - Total number of tasks
 * @param {string} status - Current status (processing, complete, error)
 * @param {Object} metadata - Additional metadata to include
 * @returns {Object} Unified progress update message
 */
function createProgressUpdate(
  completedTasks,
  totalTasks,
  status,
  metadata = {},
) {
  const progressPercentage = Math.floor((completedTasks / totalTasks) * 100);

  return {
    type: "progress_update",
    completedTasks,
    totalTasks,
    progressPercentage,
    status,
    metadata,
  };
}

/**
 * Creates a detailed progress update message with optimization metrics for a specific file
 * @param {string} fileName - Name of the processed file
 * @param {string} taskId - Full path of the processed file
 * @param {Object} optimizationResult - Result of the optimization containing metrics
 * @param {Object} metrics - Overall batch progress metrics
 * @returns {Object} Detailed progress update with file-specific optimization metrics
 */
function createDetailedProgressUpdate(
  fileName,
  taskId,
  optimizationResult,
  metrics,
) {
  return {
    type: "detailed_progress",
    fileName,
    taskId,
    optimizationMetrics: {
      originalSize: optimizationResult.original_size,
      optimizedSize: optimizationResult.optimized_size,
      savedBytes: optimizationResult.saved_bytes,
      compressionRatio: optimizationResult.compression_ratio,
      format: optimizationResult.format || "unknown",
    },
    batchMetrics: {
      completedTasks: metrics.completedTasks,
      totalTasks: metrics.totalTasks,
      progressPercentage: Math.floor(
        (metrics.completedTasks / metrics.totalTasks) * 100,
      ),
    },
    formattedMessage: `${fileName} optimized (${formatBytes(optimizationResult.saved_bytes)} saved / ${optimizationResult.compression_ratio}% compression) - Progress: ${Math.floor((metrics.completedTasks / metrics.totalTasks) * 100)}% (${metrics.completedTasks}/${metrics.totalTasks})`,
  };
}

/**
 * Format bytes to human readable format (KB, MB, etc.)
 * @param {number} bytes - Number of bytes
 * @param {number} [decimals=2] - Number of decimal places
 * @returns {string} Formatted size string
 */
function formatBytes(bytes, decimals = 2) {
  if (bytes === 0) return "0 Bytes";

  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ["Bytes", "KB", "MB", "GB", "TB"];

  const i = Math.floor(Math.log(bytes) / Math.log(k));

  return `${parseFloat((bytes / k ** i).toFixed(dm))} ${sizes[i]}`;
}

/**
 * Sends a progress message through the worker thread
 * @param {Object} message - Progress message to send
 */
function sendProgressMessage(message) {
  // Ensure we're in a worker thread context
  if (require("node:worker_threads").isMainThread) {
    console.log(JSON.stringify(message));
  } else if (parentPort) {
    parentPort.postMessage(message);
  } else {
    // Fallback to console.log if neither condition is met
    console.log(JSON.stringify(message));
  }
}

module.exports = {
  ProgressType,
  createStartMessage,
  createCompleteMessage,
  createErrorMessage,
  sendProgressMessage,
  createProgressUpdate,
  createDetailedProgressUpdate,
  formatBytes,
};
