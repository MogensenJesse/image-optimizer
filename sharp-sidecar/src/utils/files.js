/**
 * File handling utilities
 * @module utils/files
 */

const fs = require("fs");
const path = require("path");

/**
 * Ensure the output path has the correct extension for the format
 * @param {string} outputPath - The original output path
 * @param {string} inputFormat - The input file format
 * @param {string} outputFormat - The desired output format
 * @returns {string} The corrected output path
 */
function ensureCorrectExtension(outputPath, inputFormat, outputFormat) {
  if (outputFormat === inputFormat) {
    return outputPath;
  }

  const ext = `.${outputFormat}`;
  if (!outputPath.toLowerCase().endsWith(ext)) {
    return outputPath.replace(/\.[^/.]+$/, ext);
  }
  return outputPath;
}

/**
 * Get file size information
 * @param {string} filePath - Path to the file
 * @returns {Object} Object containing file size in bytes
 */
function getFileSize(filePath) {
  const stats = fs.statSync(filePath);
  return stats.size;
}

/**
 * Calculate compression statistics
 * @param {number} inputSize - Original file size in bytes
 * @param {number} outputSize - Optimized file size in bytes
 * @returns {Object} Object containing compression statistics
 */
function getCompressionStats(inputSize, outputSize) {
  const savedBytes = inputSize - outputSize;
  const compressionRatio = ((savedBytes / inputSize) * 100).toFixed(2);

  return {
    original_size: inputSize,
    optimized_size: outputSize,
    saved_bytes: savedBytes,
    compression_ratio: compressionRatio,
  };
}

/**
 * Create a standardized result object
 * @param {string} outputPath - Path to the output file
 * @param {Object} stats - Compression statistics
 * @param {string} format - Output format
 * @param {boolean} [success=true] - Whether the operation was successful
 * @param {string|null} [error=null] - Error message if operation failed
 * @returns {Object} Standardized result object
 */
function createResultObject(
  outputPath,
  stats,
  format,
  success = true,
  error = null,
) {
  return {
    path: outputPath,
    ...stats,
    format,
    success,
    error,
  };
}

module.exports = {
  ensureCorrectExtension,
  getFileSize,
  getCompressionStats,
  createResultObject,
};
