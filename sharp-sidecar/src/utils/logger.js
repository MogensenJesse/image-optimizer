/**
 * Logging utilities for consistent error and debug messages
 * @module utils/logger
 */

/**
 * Log an error message to stderr
 * @param {...any} args - Arguments to log
 */
function error(...args) {
  console.error(...args);
}

/**
 * Log a debug message with optional object formatting
 * @param {string} message - The message to log
 * @param {Object} [data] - Optional data to stringify
 */
function debug(message, data = null) {
  if (data) {
    console.log(message, JSON.stringify(data, null, 2));
  } else {
    console.log(message);
  }
}

/**
 * Log progress information
 * @param {string} stage - The current processing stage
 * @param {string} message - The progress message
 * @param {Object} [data] - Optional data to include
 */
function progress(stage, message, data = null) {
  const logMessage = `[${stage}] ${message}`;
  if (data) {
    console.log(logMessage, JSON.stringify(data, null, 2));
  } else {
    console.log(logMessage);
  }
}

module.exports = {
  error,
  debug,
  progress,
};
