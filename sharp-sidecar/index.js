const sharp = require('sharp');
const fs = require('fs');
const optimizationDefaults = require('./optimizationDefaults');

const command = process.argv[2];
const inputPath = process.argv[3];
const outputPath = process.argv[4];
const settingsArg = process.argv[5];

// Debug logging
console.error('Command arguments:');
console.error('Command:', command);
console.error('Input:', inputPath);
console.error('Output:', outputPath);
console.error('Raw settings:', settingsArg);

let settings;
try {
  settings = settingsArg ? JSON.parse(settingsArg) : {
    quality: { global: 90 },
    resize: { mode: 'none', maintainAspect: true },
    outputFormat: 'original'
  };
  console.error('Parsed settings:', JSON.stringify(settings, null, 2));
} catch (err) {
  console.error('Error parsing settings:', err);
  console.error('Raw settings string:', settingsArg);
  process.exit(1);
}

const getLosslessSettings = (format) => {
  switch (format) {
    case 'jpeg':
      // JPEG doesn't support true lossless, so we use highest quality
      return {
        quality: 100,
        mozjpeg: true,
        chromaSubsampling: '4:4:4',
        optimiseCoding: true
      };
    
    case 'png':
      return {
        compressionLevel: 9,    // Maximum compression
        palette: false,         // Disable palette mode for true color
        quality: 100,
        effort: 10,            // Maximum effort
        adaptiveFiltering: true,
      };
    
    case 'webp':
      return {
        lossless: true,        // Enable true lossless mode
        quality: 100,
        effort: 6,             // Maximum compression effort
        nearLossless: false    // Disable near-lossless mode
      };
    
    case 'avif':
      return {
        lossless: true,        // Enable true lossless mode
        quality: 100,
        effort: 9,             // Maximum compression effort
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

async function optimizeImage(input, output, settings) {
  try {
    console.error('Starting optimization with settings:', JSON.stringify(settings, null, 2));
    const inputStats = fs.statSync(input);
    const inputSize = inputStats.size;

    let image = sharp(input);
    const metadata = await image.metadata();
    const inputFormat = metadata.format;
    console.error('Input format:', inputFormat, 'dimensions:', metadata.width, 'x', metadata.height);

    if (!optimizationDefaults[inputFormat]) {
      throw new Error(`Unsupported input format: ${inputFormat}`);
    }

    // Determine output format
    const outputFormat = settings?.outputFormat === 'original' ? inputFormat : settings.outputFormat;
    console.error('Converting to format:', outputFormat);

    if (!optimizationDefaults[outputFormat]) {
      throw new Error(`Unsupported output format: ${outputFormat}`);
    }

    // Apply resize if needed
    if (settings?.resize?.mode !== 'none' && settings?.resize?.size) {
      const size = parseInt(settings.resize.size);
      console.error('Applying resize:', settings.resize.mode, 'to size:', size);

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
      console.error('New dimensions after resize:', resizedMetadata.width, 'x', resizedMetadata.height);
    }

    // Get format options
    let formatOptions;
    if (settings?.quality?.global === 100) {
      formatOptions = getLosslessSettings(outputFormat);
      console.error('Using lossless settings for', outputFormat);
    } else {
      formatOptions = { ...optimizationDefaults[outputFormat] };
      
      // Override quality settings
      if (settings?.quality) {
        if (settings.quality[outputFormat] !== null) {
          formatOptions.quality = settings.quality[outputFormat];
        } else if (settings.quality.global !== null) {
          formatOptions.quality = settings.quality.global;
        }
      }
    }

    console.error('Using format options:', formatOptions);

    // Ensure output path has correct extension
    let outputPath = output;
    if (outputFormat !== inputFormat) {
      const ext = `.${outputFormat}`;
      if (!outputPath.toLowerCase().endsWith(ext)) {
        outputPath = outputPath.replace(/\.[^/.]+$/, ext);
      }
    }
    console.error('Writing to:', outputPath);

    // Convert and save
    await image.toFormat(outputFormat, formatOptions).toFile(outputPath);
    
    const outputStats = fs.statSync(outputPath);
    const outputSize = outputStats.size;
    const savedBytes = inputSize - outputSize;
    const compressionRatio = ((savedBytes / inputSize) * 100).toFixed(2);

    const result = {
      path: outputPath,
      originalSize: inputSize,
      optimizedSize: outputSize,
      savedBytes: savedBytes,
      compressionRatio: compressionRatio,
      format: outputFormat
    };
    
    console.log(JSON.stringify(result));
    return result;
  } catch (err) {
    console.error('Error in optimizeImage:', err);
    throw err;
  }
}

switch (command) {
  case 'optimize':
    if (!inputPath || !outputPath) {
      console.error('Input and output paths are required');
      process.exit(1);
    }
    optimizeImage(inputPath, outputPath, settings);
    break;

  default:
    console.error(`unknown command ${command}`);
    process.exit(1);
}