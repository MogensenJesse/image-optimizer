const sharp = require('sharp');
const optimizationDefaults = require('./optimizationDefaults');

const command = process.argv[2];
const inputPath = process.argv[3];
const outputPath = process.argv[4];

async function optimizeImage(input, output) {
  try {
    const image = sharp(input);
    const metadata = await image.metadata();
    const format = metadata.format;

    if (!optimizationDefaults[format]) {
      throw new Error(`Unsupported format: ${format}`);
    }

    await image[format](optimizationDefaults[format])
      .toFile(output);
    
    console.log(output);
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