const fs = require('fs');
const path = require('path');
const { optimizeBatch } = require('../src/processing/batch');
const os = require('os');

// Configure logging
const LOG_FILE = path.join(__dirname, 'sidecar-output.log');
const JSON_OUTPUT_FILE = path.join(__dirname, 'sidecar-output.json');

// Redirect console.log to capture the messages sent to stdout
const originalConsoleLog = console.log;
const originalConsoleError = console.error;

// Capture all output
const allOutputs = [];
const captureOutput = (message) => {
  allOutputs.push({
    timestamp: new Date().toISOString(),
    message: typeof message === 'string' ? message : JSON.stringify(message),
    parsed: typeof message === 'string' ? tryParseJson(message) : message
  });
  
  // Still write to original console
  return message;
};

// Try to parse a JSON string, return null if it fails
function tryParseJson(str) {
  try {
    return JSON.parse(str);
  } catch (e) {
    return null;
  }
}

// Override console.log to capture messages
console.log = function() {
  const message = captureOutput(arguments[0]);
  originalConsoleLog.apply(console, arguments);
  return message;
};

// Override console.error to capture error messages
console.error = function() {
  const message = captureOutput(arguments[0]);
  originalConsoleError.apply(console, arguments);
  return message;
};

// Create a temporary output directory
const outputDir = path.join(os.tmpdir(), 'sharp-sidecar-test-output');
fs.mkdirSync(outputDir, { recursive: true });

// Setup batch task for the images
async function setupBatchTask() {
  const imagesDir = path.join(__dirname, 'images');
  const files = fs.readdirSync(imagesDir).filter(file => file.endsWith('.jpg'));
  
  const batch = [];
  
  for (const file of files) {
    const inputPath = path.join(imagesDir, file);
    const outputPath = path.join(outputDir, file);
    
    batch.push({
      input: inputPath,
      output: outputPath,
      settings: {
        quality: { global: 85 },
        resize: { 
          mode: 'width', 
          size: 800,
          maintainAspect: true 
        },
        outputFormat: 'jpeg'
      }
    });
  }
  
  console.log(`Created batch task with ${batch.length} images`);
  return batch;
}

// Process batch and capture output
async function runTest() {
  try {
    console.log('Starting sidecar output capture test');
    console.log(`Timestamp: ${new Date().toISOString()}`);
    console.log(`Node.js version: ${process.version}`);
    console.log(`Platform: ${process.platform}-${process.arch}`);
    
    const batch = await setupBatchTask();
    console.log('Processing batch...');
    
    const batchJson = JSON.stringify(batch);
    const result = await optimizeBatch(batchJson);
    
    console.log('Batch processing complete.');
    
    // Write all captured output to file
    fs.writeFileSync(LOG_FILE, allOutputs.map(item => `[${item.timestamp}] ${item.message}`).join('\n'));
    fs.writeFileSync(JSON_OUTPUT_FILE, JSON.stringify(allOutputs, null, 2));
    
    console.log(`Raw output saved to: ${LOG_FILE}`);
    console.log(`JSON output saved to: ${JSON_OUTPUT_FILE}`);
    
    // Count and summarize message types
    const messageTypes = {};
    allOutputs.forEach(output => {
      if (output.parsed && output.parsed.type) {
        messageTypes[output.parsed.type] = (messageTypes[output.parsed.type] || 0) + 1;
      }
    });
    
    console.log('Message type summary:');
    Object.entries(messageTypes).forEach(([type, count]) => {
      console.log(`- ${type}: ${count} messages`);
    });
    
    // Look specifically at progress messages
    const progressMessages = allOutputs.filter(output => 
      output.parsed && output.parsed.type === 'progress');
    
    console.log(`Progress messages: ${progressMessages.length}`);
    
    // Get unique progress types
    const progressTypes = new Set();
    progressMessages.forEach(msg => {
      if (msg.parsed.progressType) {
        progressTypes.add(msg.parsed.progressType);
      }
    });
    
    console.log('Progress message types:');
    [...progressTypes].forEach(type => {
      console.log(`- ${type}`);
    });
    
    return result;
  } catch (error) {
    console.error('Error running test:', error);
    throw error;
  }
}

// Run the test
runTest()
  .then(result => {
    console.log('Test completed successfully');
    process.exit(0);
  })
  .catch(err => {
    console.error('Test failed:', err);
    process.exit(1);
  }); 