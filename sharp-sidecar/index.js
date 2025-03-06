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
            // Send error progress (keep for error reporting)
            sendProgressMessage(createErrorMessage(task.input, err, { fileName }));
            error(`Worker ${workerId} task failed: ${fileName}`, err);
            
            results.push({
              path: task.output,
              original_size: 0,
              optimized_size: 0,
              saved_bytes: 0,
              compression_ratio: "0.00",
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
        error(`Worker ${workerId} error:`, err);
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
    error('Error parsing settings:', err);
    error('Raw settings string:', settingsArg);
    process.exit(1);
  }

  try {
    switch (command) {
      case 'optimize':
        if (!inputPath || !outputPath) {
          error('Input and output paths are required');
          process.exit(1);
        }
        try {
          const result = await optimizeImage(inputPath, outputPath, settings);
          // Ensure result is written to stdout
          process.stdout.write(JSON.stringify(result));
        } catch (err) {
          error(err);
          process.exit(1);
        }
        break;

      case 'optimize-batch':
        if (!inputPath) {
          error('Batch JSON is required');
          process.exit(1);
        }
        try {
          await optimizeBatch(inputPath);
        } catch (err) {
          error(err);
          process.exit(1);
        }
        break;

      default:
        error(`unknown command ${command}`);
        process.exit(1);
    }
  } catch (err) {
    error('Command execution failed:', err);
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