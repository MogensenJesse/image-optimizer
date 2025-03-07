const sharp = require('sharp');
const { getFormatSettings, isFormatSupported, getLosslessSettings } = require('../config/formats');
const {
  debug,
  progress,
  error,
  getFileSize,
  getCompressionStats,
  createResultObject,
  ensureCorrectExtension
} = require('../utils');
const { formatBytes } = require('../utils/progress');

/**
 * Optimize a single image with the given settings
 * @param {string} input - Input file path
 * @param {string} output - Output file path
 * @param {Object} settings - Optimization settings
 * @param {string|null} settings.format - Output format (null = same as input)
 * @param {Object} settings.quality - Quality settings (global or per-format)
 * @param {Object} settings.resize - Resize settings (width, height, fit)
 * @param {boolean} settings.strip - Strip metadata
 * @returns {Promise<Object>} Optimization result
 * @throws {Error} If optimization fails
 */
async function optimizeImage(input, output, settings) {
  try {
    debug('Starting optimization with settings:', settings);
    
    // Validate inputs
    if (!input) {
      const errorMessage = 'Input file path is required';
      error(errorMessage);
      throw new Error(errorMessage);
    }
    
    if (!output) {
      const errorMessage = 'Output file path is required';
      error(errorMessage);
      throw new Error(errorMessage);
    }
    
    const inputSize = getFileSize(input);
    if (inputSize === 0) {
      const errorMessage = `Input file is empty or cannot be read: ${input}`;
      error(errorMessage);
      throw new Error(errorMessage);
    }
    
    const fileName = require('path').basename(input);

    let image;
    try {
      image = sharp(input);
    } catch (err) {
      const errorMessage = `Failed to load image ${fileName}: ${err.message}`;
      error(errorMessage, err);
      throw new Error(errorMessage);
    }
    
    let metadata;
    try {
      metadata = await image.metadata();
    } catch (err) {
      const errorMessage = `Failed to read metadata for ${fileName}: ${err.message}`;
      error(errorMessage, err);
      throw new Error(errorMessage);
    }
    
    const inputFormat = metadata.format;
    progress('Input', `${fileName}: Format: ${inputFormat}, dimensions: ${metadata.width}x${metadata.height}`);

    if (!isFormatSupported(inputFormat)) {
      const errorMessage = `Unsupported input format: ${inputFormat}`;
      error(errorMessage);
      throw new Error(errorMessage);
    }

    // Determine output format
    const outputFormat = settings?.outputFormat === 'original' ? inputFormat : settings.outputFormat;
    debug(`Converting ${fileName} to format: ${outputFormat}`);

    if (!isFormatSupported(outputFormat)) {
      const errorMessage = `Unsupported output format: ${outputFormat}`;
      error(errorMessage);
      throw new Error(errorMessage);
    }

    // Apply resize if needed
    if (settings?.resize?.mode !== 'none' && settings?.resize?.size) {
      const size = parseInt(settings.resize.size);
      progress('Resize', `Mode: ${settings.resize.mode}, size: ${size}`);

      try {
        switch (settings.resize.mode) {
          case 'width':
            image = image.resize(size, null, { 
              withoutEnlargement: true,
              fit: 'inside'
            });
            break;
          case 'height':
            image = image.resize(null, size, { 
              withoutEnlargement: true,
              fit: 'inside'
            });
            break;
          case 'longest':
            if (metadata.width >= metadata.height) {
              image = image.resize(size, null, { 
                withoutEnlargement: true,
                fit: 'inside'
              });
            } else {
              image = image.resize(null, size, { 
                withoutEnlargement: true,
                fit: 'inside'
              });
            }
            break;
          case 'shortest':
            if (metadata.width <= metadata.height) {
              image = image.resize(size, null, { 
                withoutEnlargement: true,
                fit: 'inside'
              });
            } else {
              image = image.resize(null, size, { 
                withoutEnlargement: true,
                fit: 'inside'
              });
            }
            break;
          default:
            const errorMessage = `Unknown resize mode: ${settings.resize.mode}`;
            error(errorMessage);
            throw new Error(errorMessage);
        }

        const resizedMetadata = await image.metadata();
        progress('Resize', `New dimensions: ${resizedMetadata.width}x${resizedMetadata.height}`);
      } catch (err) {
        const errorMessage = `Error resizing image ${fileName}: ${err.message}`;
        error(errorMessage, err);
        throw new Error(errorMessage);
      }
    }

    // Get format options
    let formatOptions;
    try {
      formatOptions = getFormatSettings(outputFormat, settings?.quality);
      debug('Using format options:', formatOptions);
    } catch (err) {
      const errorMessage = `Error getting format settings for ${outputFormat}: ${err.message}`;
      error(errorMessage, err);
      throw new Error(errorMessage);
    }

    // Ensure output path has correct extension
    const outputPath = ensureCorrectExtension(output, inputFormat, outputFormat);
    debug(`Writing ${fileName} to: ${outputPath}`);

    // Convert and save
    try {
      await image.toFormat(outputFormat, formatOptions).toFile(outputPath);
    } catch (err) {
      const errorMessage = `Error saving image ${fileName} to ${outputPath}: ${err.message}`;
      error(errorMessage, err);
      throw new Error(errorMessage);
    }
    
    let outputSize;
    try {
      outputSize = getFileSize(outputPath);
    } catch (err) {
      const errorMessage = `Error getting file size for ${outputPath}: ${err.message}`;
      error(errorMessage, err);
      throw new Error(errorMessage);
    }
    
    const stats = getCompressionStats(inputSize, outputSize);
    progress('Complete', `${fileName}: Saved ${formatBytes(stats.saved_bytes)} (${stats.compression_ratio}% reduction)`);

    return createResultObject(outputPath, stats, outputFormat);
  } catch (err) {
    // If error doesn't have a specific message already set, add context
    if (err.message && !err.message.includes('Error in optimizeImage:')) {
      error('Error in optimizeImage:', err);
    }
    throw err;
  }
}

module.exports = {
  optimizeImage
}; 