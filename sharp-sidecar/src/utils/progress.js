const { parentPort } = require('worker_threads');

/**
 * Progress message handling utilities
 * @module utils/progress
 */

/**
 * Progress message types
 * @enum {string}
 */
const ProgressType = {
  START: 'start',
  PROGRESS: 'progress',
  COMPLETE: 'complete',
  ERROR: 'error'
};

/**
 * Creates a progress message object
 * @param {string} type - Progress type from ProgressType enum
 * @param {Object} data - Progress data
 * @returns {Object} Formatted progress message
 */
function createProgressMessage(type, data) {
  return {
    type: 'progress',
    progressType: type,
    ...data
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
    workerId: metadata.workerId
  });
}

/**
 * Creates a completion progress message
 * @param {string} taskId - Task identifier (usually input path)
 * @param {Object} result - Task result data
 * @returns {Object} Completion progress message
 */
function createCompleteMessage(taskId, result) {
  return createProgressMessage(ProgressType.COMPLETE, {
    taskId,
    workerId: result.workerId,
    result
  });
}

/**
 * Creates an error progress message
 * @param {string} taskId - Task identifier (usually input path)
 * @param {Error|string} error - Error object or message
 * @returns {Object} Error progress message
 */
function createErrorMessage(taskId, error) {
  return createProgressMessage(ProgressType.ERROR, {
    taskId,
    error: error instanceof Error ? error.message : error
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
function createProgressUpdate(completedTasks, totalTasks, status, metadata = {}) {
  const progressPercentage = Math.floor((completedTasks / totalTasks) * 100);
  
  return {
    type: 'progress_update',
    completedTasks,
    totalTasks,
    progressPercentage,
    status,
    metadata
  };
}

/**
 * Sends a progress message through the worker thread
 * @param {Object} message - Progress message to send
 */
function sendProgressMessage(message) {
  // Ensure we're in a worker thread context
  if (require('worker_threads').isMainThread) {
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
  createProgressUpdate
}; 