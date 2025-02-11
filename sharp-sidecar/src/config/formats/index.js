/**
 * Format configuration interface
 * @module config/formats
 */

const defaults = require('./defaults');
const { getLosslessSettings } = require('./lossless');

/**
 * Get format settings based on quality requirements
 * @param {string} format - The image format
 * @param {Object} quality - Quality settings
 * @returns {Object} Format-specific settings
 */
function getFormatSettings(format, quality = { global: 90 }) {
  if (!defaults[format]) {
    throw new Error(`Unsupported format: ${format}`);
  }

  if (quality?.global === 100 || quality?.[format] === 100) {
    const losslessSettings = getLosslessSettings(format);
    if (losslessSettings) {
      return losslessSettings;
    }
  }

  const formatOptions = { ...defaults[format] };
  
  // Override quality settings if specified
  if (quality) {
    if (quality[format] !== null && quality[format] !== undefined) {
      formatOptions.quality = quality[format];
    } else if (quality.global !== null && quality.global !== undefined) {
      formatOptions.quality = quality.global;
    }
  }

  return formatOptions;
}

/**
 * Check if a format is supported
 * @param {string} format - The image format to check
 * @returns {boolean} Whether the format is supported
 */
function isFormatSupported(format) {
  return format in defaults;
}

module.exports = {
  getFormatSettings,
  isFormatSupported,
  defaults,
  getLosslessSettings
}; 