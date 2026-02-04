const fs = require("node:fs");
const path = require("node:path");
const os = require("node:os");

// Store original console methods BEFORE any module loading (for error reporting)
const _originalStderr = process.stderr.write.bind(process.stderr);
const _realConsoleError = (...args) => {
  _originalStderr(`${args.join(" ")}\n`);
};

// Early error handler to catch module loading issues
process.on("uncaughtException", (err) => {
  _realConsoleError("Uncaught exception:", err.message);
  _realConsoleError(err.stack);
  process.exit(1);
});

process.on("unhandledRejection", (reason, promise) => {
  _realConsoleError("Unhandled rejection at:", promise, "reason:", reason);
  process.exit(1);
});

// Load Sharp and clear cache to release file handles from previous runs
const sharp = require("sharp");
// Clear Sharp's cache to release any file handles from previous runs
// This helps avoid Windows file locking issues on repeated test runs
sharp.cache(false);
sharp.cache({ memory: 50, files: 10, items: 100 });

// Load batch processor
const { optimizeBatch } = require("../src/processing/batch");

// Configure logging
const LOG_FILE = path.join(__dirname, "sidecar-output.log");

// Redirect console.log to capture the messages sent to stdout
const _originalConsoleLog = console.log;
const _originalConsoleError = console.error;

// Setup logging to file
const logStream = fs.createWriteStream(LOG_FILE, { flags: "w" });

// Capture all output to log file
const captureOutput = (message) => {
  const timestamp = new Date().toISOString();
  let formattedMessage;

  if (typeof message === "string") {
    // Trim the message to remove leading/trailing whitespace
    message = message.trim();
    // Skip empty messages
    if (!message) return message;
    formattedMessage = `[${timestamp}] ${message}`;
  } else {
    formattedMessage = `[${timestamp}] ${JSON.stringify(message)}`;
  }

  // Write to log file (only if there's actual content)
  if (formattedMessage.length > timestamp.length + 3) {
    logStream.write(`${formattedMessage}\n`);
  }

  return message;
};

// Capture process.stdout.write calls
const originalStdoutWrite = process.stdout.write;
process.stdout.write = (...args) => {
  const chunk = args[0];
  try {
    // If it's a Buffer, convert to string
    if (Buffer.isBuffer(chunk)) {
      const stringChunk = chunk.toString("utf8");
      // Split by newlines and process each line separately
      stringChunk.split("\n").forEach((line) => {
        if (line.trim()) captureOutput(line);
      });
    }
    // Try to parse as JSON if it's a string
    else if (typeof chunk === "string" && chunk.trim().startsWith("{")) {
      const message = JSON.parse(chunk);
      captureOutput(message);
    } else if (typeof chunk === "string") {
      // Split by newlines and process each line separately
      chunk.split("\n").forEach((line) => {
        if (line.trim()) captureOutput(line);
      });
    } else {
      captureOutput(chunk);
    }
  } catch (_e) {
    // If it's not valid JSON, just capture as is
    captureOutput(chunk);
  }
  return originalStdoutWrite.apply(process.stdout, args);
};

// Override console methods
console.log = (...args) => {
  // Properly handle multiple arguments
  if (args.length === 1) {
    // Handle a single argument
    const arg = args[0];
    if (typeof arg === "string") {
      // Split string by newlines and process each line
      arg.split("\n").forEach((line) => {
        if (line.trim()) captureOutput(line);
      });
    } else {
      captureOutput(arg);
    }
  } else {
    // Combine multiple arguments
    const combinedArgs = args
      .map((arg) => (typeof arg === "object" ? JSON.stringify(arg) : arg))
      .join(" ");

    // Split by newlines and process each line
    combinedArgs.split("\n").forEach((line) => {
      if (line.trim()) captureOutput(line);
    });
  }
  // Don't call original console.log to avoid double logging
  // _originalConsoleLog.apply(console, args);
};

console.error = (...args) => {
  // Properly handle multiple arguments
  if (args.length === 1) {
    // Handle a single argument
    const arg = args[0];
    if (typeof arg === "string") {
      // Split string by newlines and process each line
      arg.split("\n").forEach((line) => {
        if (line.trim()) captureOutput(`ERROR: ${line}`);
      });
    } else {
      captureOutput(`ERROR: ${arg}`);
    }
  } else {
    // Combine multiple arguments
    const combinedArgs = args
      .map((arg) => (typeof arg === "object" ? JSON.stringify(arg) : arg))
      .join(" ");

    // Split by newlines and process each line
    combinedArgs.split("\n").forEach((line) => {
      if (line.trim()) captureOutput(`ERROR: ${line}`);
    });
  }
  // Don't call original console.error to avoid double logging
  // _originalConsoleError.apply(console, args);
};

// Setup test images
async function setupBatchTask() {
  const testImagesDir = path.join(__dirname, "images");

  // Ensure the test images directory exists
  if (!fs.existsSync(testImagesDir)) {
    console.log(`Creating test images directory: ${testImagesDir}`);
    fs.mkdirSync(testImagesDir, { recursive: true });
  }

  // Ensure the optimized directory exists and clean up any previous output
  const optimizedDir = path.join(testImagesDir, "optimized");
  if (fs.existsSync(optimizedDir)) {
    // Clean up previous output files to avoid Windows file locking issues
    const existingFiles = fs.readdirSync(optimizedDir);
    for (const file of existingFiles) {
      try {
        fs.unlinkSync(path.join(optimizedDir, file));
      } catch (err) {
        // If file is locked, wait a bit and retry
        if (err.code === "EBUSY" || err.code === "EPERM") {
          console.log(`File locked, waiting to retry: ${file}`);
          await new Promise((resolve) => setTimeout(resolve, 100));
          try {
            fs.unlinkSync(path.join(optimizedDir, file));
          } catch {
            console.log(`Could not delete ${file}, will overwrite`);
          }
        }
      }
    }
  } else {
    console.log(`Creating optimized output directory: ${optimizedDir}`);
    fs.mkdirSync(optimizedDir, { recursive: true });
  }

  // Get list of test images
  const imageFiles = fs
    .readdirSync(testImagesDir)
    .filter((file) => /\.(jpg|jpeg|png|webp)$/i.test(file))
    .map((file) => path.join(testImagesDir, file));

  if (imageFiles.length === 0) {
    console.error(
      "No test images found. Please add some test images to the images directory.",
    );
    process.exit(1);
  }

  // Create batch task - format as expected by the batch processor
  const tasks = imageFiles.map((imagePath) => {
    const outputPath = path.join(
      path.dirname(imagePath),
      "optimized",
      path.basename(imagePath),
    );
    return {
      input: imagePath,
      output: outputPath,
      settings: {
        quality: { global: 80 },
        resize: { mode: "none" },
        outputFormat: "original",
      },
    };
  });

  console.log(`Created batch task with ${tasks.length} images`);
  return tasks;
}

// Run the test
async function runTest() {
  console.log("Starting sidecar output capture test");
  console.log(`Timestamp: ${new Date().toISOString()}`);
  console.log(`Node.js version: ${process.version}`);
  console.log(`Platform: ${process.platform}-${os.arch()}`);

  try {
    // Setup batch task
    const tasks = await setupBatchTask();

    // Process batch - convert tasks to JSON string as expected by optimizeBatch
    console.log("Processing batch...");
    const tasksJson = JSON.stringify(tasks);
    await optimizeBatch(tasksJson);
    console.log("Batch processing complete.");

    // Close log stream
    logStream.end();

    return 0;
  } catch (error) {
    // Log to both real stderr and captured log
    _realConsoleError("Error in test:", error.message);
    _realConsoleError(error.stack);
    console.error(`Error in test: ${error.message}`);
    console.error(error.stack);

    // Close log stream
    logStream.end();

    return 1;
  }
}

// Run the test
runTest()
  .then((exitCode) => {
    process.exit(exitCode);
  })
  .catch((error) => {
    _realConsoleError("Unhandled error:", error.message);
    _realConsoleError(error.stack);
    process.exit(1);
  });
