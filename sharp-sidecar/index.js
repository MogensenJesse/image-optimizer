const sharp = require('sharp');

const command = process.argv[2];
const inputPath = process.argv[3];
const outputPath = process.argv[4];

switch (command) {
  case 'optimize':
    if (!inputPath || !outputPath) {
      console.error('Input and output paths are required');
      process.exit(1);
    }

    sharp(inputPath)
      .jpeg({ quality: 80, mozjpeg: true })
      .toFile(outputPath)
      .then(() => {
        console.log(outputPath);
        process.exit(0);
      })
      .catch(err => {
        console.error(err);
        process.exit(1);
      });
    break;

  default:
    console.error(`unknown command ${command}`);
    process.exit(1);
}