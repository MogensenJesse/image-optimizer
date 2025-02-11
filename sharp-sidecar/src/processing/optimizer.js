const sharp = require('sharp');
const { getFormatSettings, isFormatSupported } = require('../config/formats');
const {
  debug,
  progress,
  error,
  getFileSize,
  getCompressionStats,
  createResultObject,
  ensureCorrectExtension
} = require('../utils');

/**
 * Get lossless settings for a specific format
 * @param {string} format - The image format (jpeg, png, webp, avif, tiff)
 * @returns {Object} Format-specific lossless settings
 */
const getLosslessSettings = (format) => {
  switch (format) {
    case 'jpeg':
      return {
        quality: 100,
        mozjpeg: true,
        chromaSubsampling: '4:4:4',
        optimiseCoding: true
      };
    case 'png':
      return {
        compressionLevel: 9,
        palette: false,
        quality: 100,
        effort: 10,
        adaptiveFiltering: true,
      };
    case 'webp':
      return {
        lossless: true,
        quality: 100,
        effort: 6,
        nearLossless: false
      };
    case 'avif':
      return {
        lossless: true,
        quality: 100,
        effort: 9,
        chromaSubsampling: '4:4:4'
      };
    case 'tiff':
      return {
        quality: 100,
        compression: 'deflate',
        predictor: 'horizontal',
        pyramid: false,
        tile: true,
        tileWidth: 256,
        tileHeight: 256,
        squash: false,
        preserveIccProfile: true
      };
    default:
      return optimizationDefaults[format];
  }
};

/**
 * Optimize a single image with the given settings
 * @param {string} input - Input file path
 * @param {string} output - Output file path
 * @param {Object} settings - Optimization settings
 * @returns {Promise<Object>} Optimization result
 */
async function optimizeImage(input, output, settings) {
  try {
    debug('Starting optimization with settings:', settings);
    const inputSize = getFileSize(input);

    let image = sharp(input);
    const metadata = await image.metadata();
    const inputFormat = metadata.format;
    progress('Input', `Format: ${inputFormat}, dimensions: ${metadata.width}x${metadata.height}`);

    if (!isFormatSupported(inputFormat)) {
      throw new Error(`Unsupported input format: ${inputFormat}`);
    }

    // Determine output format
    const outputFormat = settings?.outputFormat === 'original' ? inputFormat : settings.outputFormat;
    debug('Converting to format:', outputFormat);

    if (!isFormatSupported(outputFormat)) {
      throw new Error(`Unsupported output format: ${outputFormat}`);
    }

    // Apply resize if needed
    if (settings?.resize?.mode !== 'none' && settings?.resize?.size) {
      const size = parseInt(settings.resize.size);
      progress('Resize', `Mode: ${settings.resize.mode}, size: ${size}`);

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
      }

      const resizedMetadata = await image.metadata();
      progress('Resize', `New dimensions: ${resizedMetadata.width}x${resizedMetadata.height}`);
    }

    // Get format options
    const formatOptions = getFormatSettings(outputFormat, settings?.quality);
    debug('Using format options:', formatOptions);

    // Ensure output path has correct extension
    const outputPath = ensureCorrectExtension(output, inputFormat, outputFormat);
    debug('Writing to:', outputPath);

    // Convert and save
    await image.toFormat(outputFormat, formatOptions).toFile(outputPath);
    
    const outputSize = getFileSize(outputPath);
    const stats = getCompressionStats(inputSize, outputSize);
    progress('Complete', `Saved ${stats.saved_bytes} bytes (${stats.compression_ratio}% reduction)`);

    return createResultObject(outputPath, stats, outputFormat);
  } catch (err) {
    error('Error in optimizeImage:', err);
    throw err;
  }
}

module.exports = {
  optimizeImage,
  getLosslessSettings
}; 