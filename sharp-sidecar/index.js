const { isMainThread, parentPort, workerData } = require('worker_threads');
const { optimizeImage } = require('./src/processing/optimizer');
const { optimizeBatch } = require('./src/processing/batch');
const { error, debug } = require('./src/utils');
const { createStartMessage, createCompleteMessage, createErrorMessage, sendProgressMessage, formatBytes } = require('./src/utils/progress');
const path = require('path');

// Worker thread code
if (!isMainThread && workerData?.isWorker) {
  const workerId = workerData.workerId;
  debug(`Worker thread ${workerId} started`);
  
  parentPort.on('message', async ({ type, tasks }) => {
    if (type === 'process') {
      try {
        debug(`Worker ${workerId} received ${tasks.length} tasks`);
        const results = [];
        
        for (const task of tasks) {
          try {
            const fileName = path.basename(task.input);
            // Send start progress (keep for detailed logging in benchmark mode)
            sendProgressMessage(createStartMessage(task.input, {
              workerId,
              fileName
            }));
            
            debug(`Worker ${workerId} processing: ${fileName}`);
            const result = await optimizeImage(task.input, task.output, task.settings);
            
            // Create a more detailed result with formatted values for display
            const enhancedResult = {
              ...result,
              workerId,
              fileName,
              formattedOriginalSize: formatBytes(result.original_size),
              formattedOptimizedSize: formatBytes(result.optimized_size),
              formattedSavedBytes: formatBytes(result.saved_bytes),
              optimizationMessage: `${fileName} optimized: ${formatBytes(result.original_size)} → ${formatBytes(result.optimized_size)} (${result.compression_ratio}% reduction)`
            };
            
            // Send completion progress with enhanced result
            sendProgressMessage(createCompleteMessage(task.input, enhancedResult));
            
            results.push(result);
          } catch (err) {
            const fileName = path.basename(task.input);
            const errorMessage = `Error processing ${fileName}: ${err.message}`;
            // Send error progress (keep for error reporting)
            sendProgressMessage(createErrorMessage(task.input, err, { fileName }));
            error(errorMessage, err);
            
            results.push({
              path: task.output,
              original_size: 0,
              optimized_size: 0,
              saved_bytes: 0,
              compression_ratio: "0.00",
              format: null,
              success: false,
              error: err.message,
              fileName
            });
          }
        }
        
        debug(`Worker ${workerId} completed ${results.length} tasks`);
        // Send final results
        parentPort.postMessage({
          type: 'results',
          workerId,
          results
        });
      } catch (err) {
        const errorMessage = `Worker ${workerId} error: ${err.message}`;
        error(errorMessage, err);
        parentPort.postMessage({
          type: 'error',
          workerId,
          error: err.message
        });
      }
    }
  });
}

// Main thread code
async function main() {
  const command = process.argv[2];
  const inputPath = process.argv[3];
  const outputPath = process.argv[4];
  const settingsArg = process.argv[5];

  // Only run this in main thread
  if (isMainThread) {
    debug('Command arguments:', {
      command,
      input: inputPath,
      output: outputPath,
      settings: settingsArg
    });
  }

  let settings;
  try {
    settings = settingsArg ? JSON.parse(settingsArg) : {
      quality: { global: 90 },
      resize: { mode: 'none', maintainAspect: true },
      outputFormat: 'original'
    };
    debug('Parsed settings:', settings);
  } catch (err) {
    const errorMessage = `Failed to parse settings: ${err.message}`;
    error(errorMessage, err);
    error('Raw settings string:', settingsArg);
    throw new Error(errorMessage);
  }

  try {
    switch (command) {
      case 'optimize':
        if (!inputPath || !outputPath) {
          const errorMessage = 'Input and output paths are required';
          error(errorMessage);
          throw new Error(errorMessage);
        }
        try {
          const result = await optimizeImage(inputPath, outputPath, settings);
          // Ensure result is written to stdout
          process.stdout.write(JSON.stringify(result));
        } catch (err) {
          const errorMessage = `Error optimizing image: ${err.message}`;
          error(errorMessage, err);
          throw err;
        }
        break;

      case 'optimize-batch':
        if (!inputPath) {
          const errorMessage = 'Batch JSON is required';
          error(errorMessage);
          throw new Error(errorMessage);
        }
        try {
          await optimizeBatch(inputPath);
        } catch (err) {
          const errorMessage = `Error in batch optimization: ${err.message}`;
          error(errorMessage, err);
          throw err;
        }
        break;

      default:
        const errorMessage = `Unknown command: ${command}`;
        error(errorMessage);
        throw new Error(errorMessage);
    }
  } catch (err) {
    // Exit with error code 1 for command-line usage
    process.exit(1);
  }
}

// Only run main in the main thread
if (isMainThread) {
  main().catch(err => {
    error('Fatal error:', err);
    process.exit(1);
  });
}