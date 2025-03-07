const fs = require('fs');
const path = require('path');
const { optimizeBatch } = require('../src/processing/batch');
const os = require('os');

// Configure logging
const LOG_FILE = path.join(__dirname, 'sidecar-output.log');

// Redirect console.log to capture the messages sent to stdout
const originalConsoleLog = console.log;
const originalConsoleError = console.error;

// Setup logging to file
const logStream = fs.createWriteStream(LOG_FILE, { flags: 'w' });

// Capture all output to log file
const captureOutput = (message) => {
  const timestamp = new Date().toISOString();
  const formattedMessage = `[${timestamp}] ${typeof message === 'string' ? message : JSON.stringify(message)}`;
  
  // Write to log file
  logStream.write(formattedMessage + '\n');
  
  // Still write to original console
  return message;
};

// Override console methods
console.log = function() {
  const message = captureOutput(arguments[0]);
  originalConsoleLog.apply(console, arguments);
};

console.error = function() {
  const message = captureOutput(`ERROR: ${arguments[0]}`);
  originalConsoleError.apply(console, arguments);
};

// Setup test images
async function setupBatchTask() {
  const testImagesDir = path.join(__dirname, 'images');
  
  // Ensure the test images directory exists
  if (!fs.existsSync(testImagesDir)) {
    console.log(`Creating test images directory: ${testImagesDir}`);
    fs.mkdirSync(testImagesDir, { recursive: true });
  }
  
  // Get list of test images
  const imageFiles = fs.readdirSync(testImagesDir)
    .filter(file => /\.(jpg|jpeg|png|webp)$/i.test(file))
    .map(file => path.join(testImagesDir, file));
  
  if (imageFiles.length === 0) {
    console.error('No test images found. Please add some test images to the images directory.');
    process.exit(1);
  }
  
  // Create batch task - format as expected by the batch processor
  const tasks = imageFiles.map(imagePath => {
    const outputPath = path.join(path.dirname(imagePath), 'optimized', path.basename(imagePath));
    return {
      input: imagePath,
      output: outputPath,
      settings: {
        quality: { global: 80 },
        resize: { mode: 'none' },
        outputFormat: 'original'
      }
    };
  });
  
  console.log(`Created batch task with ${tasks.length} images`);
  return tasks;
}

// Run the test
async function runTest() {
  console.log('Starting sidecar output capture test');
  console.log(`Timestamp: ${new Date().toISOString()}`);
  console.log(`Node.js version: ${process.version}`);
  console.log(`Platform: ${process.platform}-${os.arch()}`);
  
  try {
    // Setup batch task
    const tasks = await setupBatchTask();
    
    // Process batch - convert tasks to JSON string as expected by optimizeBatch
    console.log('Processing batch...');
    const tasksJson = JSON.stringify(tasks);
    await optimizeBatch(tasksJson);
    console.log('Batch processing complete.');
    
    // Close log stream
    logStream.end();
    
    return 0;
  } catch (error) {
    console.error(`Error in test: ${error.message}`);
    console.error(error.stack);
    
    // Close log stream
    logStream.end();
    
    return 1;
  }
}

// Run the test
runTest()
  .then(exitCode => {
    process.exit(exitCode);
  })
  .catch(error => {
    console.error(`Unhandled error: ${error.message}`);
    process.exit(1);
  }); 