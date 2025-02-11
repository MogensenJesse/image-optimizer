const { isMainThread, parentPort, workerData } = require('worker_threads');
const { optimizeImage } = require('./src/processing/optimizer');
const { optimizeBatch } = require('./src/processing/batch');
const { error, debug, progress, createResultObject } = require('./src/utils');

// Worker thread code
if (!isMainThread && workerData?.isWorker) {
  debug('Worker thread started');
  parentPort.on('message', async ({ type, tasks }) => {
    if (type === 'process') {
      try {
        progress('Worker', `Received ${tasks.length} tasks`);
        const results = [];
        for (const task of tasks) {
          try {
            debug(`Processing task: ${task.input}`);
            const result = await optimizeImage(task.input, task.output, task.settings);
            results.push(result);
            progress('Task', `Completed: ${task.input}`);
          } catch (error) {
            error(`Task failed: ${task.input}`, error);
            results.push(createResultObject(
              task.output,
              { original_size: 0, optimized_size: 0, saved_bytes: 0, compression_ratio: "0.00" },
              null,
              false,
              error.message
            ));
          }
        }
        progress('Worker', `Completed ${results.length} tasks`);
        parentPort.postMessage(results);
      } catch (err) {
        error('Worker error:', err);
        parentPort.postMessage([]);
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