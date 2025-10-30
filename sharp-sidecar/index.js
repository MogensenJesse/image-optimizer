// Handle Sharp native modules path resolution
const fs = require("node:fs");
const path = require("node:path");

// Check if we're running as a packaged executable
const isPackaged =
  !process.argv[0].endsWith("node") && !process.argv[0].endsWith("nodejs");

if (isPackaged) {
  console.log(
    "Running as packaged executable, setting up Sharp native modules paths",
  );

  // Get the executable directory
  const execDir = path.dirname(process.execPath);

  // Set environment variables for Sharp to find native modules
  process.env.SHARP_DIST_DIR = path.join(execDir, "sharp");

  // Log for debugging
  console.log(`Set SHARP_DIST_DIR to: ${process.env.SHARP_DIST_DIR}`);

  // Verify directories exist
  if (fs.existsSync(path.join(execDir, "sharp", "build", "Release"))) {
    console.log("Found Sharp native modules directory");
  } else {
    console.error(
      "ERROR: Sharp native modules directory not found at:",
      path.join(execDir, "sharp", "build", "Release"),
    );
    console.log("Available directories:", fs.readdirSync(execDir));
  }
}

// Now load worker_threads and continue with the rest of the code
const { isMainThread, parentPort, workerData } = require("node:worker_threads");
const { optimizeImage } = require("./src/processing/optimizer");
const { optimizeBatch } = require("./src/processing/batch");
const { error, debug } = require("./src/utils");
const {
  createStartMessage,
  createCompleteMessage,
  createErrorMessage,
  sendProgressMessage,
  formatBytes,
} = require("./src/utils/progress");

// Worker thread code
if (!isMainThread && workerData?.isWorker) {
  const workerId = workerData.workerId;
  debug(`Worker thread ${workerId} started`);

  parentPort.on("message", async ({ type, tasks }) => {
    if (type === "process") {
      try {
        debug(`Worker ${workerId} received ${tasks.length} tasks`);
        const results = [];

        for (const task of tasks) {
          try {
            const fileName = path.basename(task.input);
            // Send start progress (keep for detailed logging in benchmark mode)
            sendProgressMessage(
              createStartMessage(task.input, {
                workerId,
                fileName,
              }),
            );

            debug(`Worker ${workerId} processing: ${fileName}`);
            const result = await optimizeImage(
              task.input,
              task.output,
              task.settings,
            );

            // Create a more detailed result with formatted values for display
            const enhancedResult = {
              ...result,
              workerId,
              fileName,
              formattedOriginalSize: formatBytes(result.original_size),
              formattedOptimizedSize: formatBytes(result.optimized_size),
              formattedSavedBytes: formatBytes(result.saved_bytes),
              optimizationMessage: `${fileName} optimized: ${formatBytes(result.original_size)} → ${formatBytes(result.optimized_size)} (${result.compression_ratio}% reduction)`,
            };

            // Send completion progress with enhanced result
            sendProgressMessage(
              createCompleteMessage(task.input, enhancedResult),
            );

            results.push(result);
          } catch (err) {
            const fileName = path.basename(task.input);
            const errorMessage = `Error processing ${fileName}: ${err.message}`;
            // Send error progress (keep for error reporting)
            sendProgressMessage(
              createErrorMessage(task.input, err, { fileName }),
            );
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
              fileName,
            });
          }
        }

        debug(`Worker ${workerId} completed ${results.length} tasks`);
        // Send final results
        parentPort.postMessage({
          type: "results",
          workerId,
          results,
        });
      } catch (err) {
        const errorMessage = `Worker ${workerId} error: ${err.message}`;
        error(errorMessage, err);
        parentPort.postMessage({
          type: "error",
          workerId,
          error: err.message,
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
    debug("Command arguments:", {
      command,
      input: inputPath,
      output: outputPath,
      settings: settingsArg,
    });
  }

  let settings;
  try {
    settings = settingsArg
      ? JSON.parse(settingsArg)
      : {
          quality: { global: 90 },
          resize: { mode: "none", maintainAspect: true },
          outputFormat: "original",
        };
    debug("Parsed settings:", settings);
  } catch (err) {
    const errorMessage = `Failed to parse settings: ${err.message}`;
    error(errorMessage, err);
    error("Raw settings string:", settingsArg);
    throw new Error(errorMessage);
  }

  try {
    switch (command) {
      case "optimize":
        if (!inputPath || !outputPath) {
          const errorMessage = "Input and output paths are required";
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

      case "optimize-batch":
        if (!inputPath) {
          const errorMessage = "Batch JSON is required";
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

      case "optimize-batch-mmap":
        if (!inputPath) {
          const errorMessage = "Memory-mapped file path is required";
          error(errorMessage);
          throw new Error(errorMessage);
        }

        try {
          // Read from the memory-mapped file
          const fs = require("node:fs");
          debug(`Reading batch data from memory-mapped file: ${inputPath}`);
          const fileData = fs.readFileSync(inputPath, "utf8");

          debug(
            `Successfully read ${fileData.length} bytes from memory-mapped file`,
          );

          // Process the batch
          await optimizeBatch(fileData);

          // Clean up the file - only if processing was successful
          try {
            fs.unlinkSync(inputPath);
            debug(`Successfully removed temporary file: ${inputPath}`);
          } catch (unlinkErr) {
            // Just log the error, don't fail the whole process
            error(
              `Warning: Failed to remove temporary file: ${unlinkErr.message}`,
            );
          }
        } catch (err) {
          const errorMessage = `Error reading from memory-mapped file: ${err.message}`;
          error(errorMessage, err);
          throw err;
        }
        break;

      default: {
        const errorMessage = `Unknown command: ${command}`;
        error(errorMessage);
        throw new Error(errorMessage);
      }
    }
  } catch (_err) {
    // Exit with error code 1 for command-line usage
    process.exit(1);
  }
}

// Only run main in the main thread
if (isMainThread) {
  main().catch((err) => {
    error("Fatal error:", err);
    process.exit(1);
  });
}
