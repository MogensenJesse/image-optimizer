const sharp = require('sharp');
const fs = require('fs');
const optimizationDefaults = require('./optimizationDefaults');

const command = process.argv[2];
const inputPath = process.argv[3];
const outputPath = process.argv[4];

async function optimizeImage(input, output) {
  try {
    // Get input file size
    const inputStats = fs.statSync(input);
    const inputSize = inputStats.size;

    const image = sharp(input);
    const metadata = await image.metadata();
    const format = metadata.format;

    if (!optimizationDefaults[format]) {
      throw new Error(`Unsupported format: ${format}`);
    }

    await image[format](optimizationDefaults[format])
      .toFile(output);
    
    // Get output file size and calculate stats
    const outputStats = fs.statSync(output);
    const outputSize = outputStats.size;
    const savedBytes = inputSize - outputSize;
    const compressionRatio = ((savedBytes / inputSize) * 100).toFixed(2);

    // Return stats as JSON
    const result = {
      path: output,
      originalSize: inputSize,
      optimizedSize: outputSize,
      savedBytes: savedBytes,
      compressionRatio: compressionRatio,
      format: format
    };

    console.log(JSON.stringify(result));
    process.exit(0);
  } catch (err) {
    console.error(err);
    process.exit(1);
  }
}

switch (command) {
  case 'optimize':
    if (!inputPath || !outputPath) {
      console.error('Input and output paths are required');
      process.exit(1);
    }
    optimizeImage(inputPath, outputPath);
    break;

  default:
    console.error(`unknown command ${command}`);
    process.exit(1);
}