/**
 * Batch processing functionality
 * @module processing/batch
 */

const { isMainThread } = require("node:worker_threads");
const SharpWorkerPool = require("../workers/worker-pool");
const { error, debug, progress } = require("../utils");

/**
 * Process a batch of images
 * @param {string} batchJson - JSON string containing batch processing tasks
 * @returns {Promise<Object>} Object containing results array and metrics
 * @throws {Error} If batch processing fails
 */
async function optimizeBatch(batchJson) {
  if (!isMainThread) return;

  try {
    // Validate input
    if (!batchJson) {
      const errorMessage = "Batch JSON input is required";
      error(errorMessage);
      throw new Error(errorMessage);
    }

    debug("Starting batch optimization");

    let batch;
    try {
      batch = JSON.parse(batchJson);
    } catch (err) {
      const errorMessage = `Failed to parse batch JSON: ${err.message}`;
      error(errorMessage, err);
      throw new Error(errorMessage);
    }

    if (!Array.isArray(batch)) {
      const errorMessage = "Batch must be an array of tasks";
      error(errorMessage);
      throw new Error(errorMessage);
    }

    if (batch.length === 0) {
      const errorMessage = "Batch is empty, no tasks to process";
      error(errorMessage);
      throw new Error(errorMessage);
    }

    progress("Batch", `Processing ${batch.length} images`);

    // Create pool with worker count limited to task count (no point having more workers than tasks)
    const pool = new SharpWorkerPool(batch.length);
    try {
      let results, metrics;
      try {
        ({ results, metrics } = await pool.processBatch(batch));
      } catch (err) {
        const errorMessage = `Worker pool failed to process batch: ${err.message}`;
        error(errorMessage, err);
        throw new Error(errorMessage);
      }

      if (!results || results.length === 0) {
        const errorMessage = "No results returned from worker pool";
        error(errorMessage);
        throw new Error(errorMessage);
      }

      progress("Complete", `Successfully processed ${results.length} images`);
      debug("Worker pool metrics:", metrics);

      // Output both results and metrics
      const output = {
        results,
        metrics,
      };

      try {
        // Use console.log instead of process.stdout.write to ensure proper flushing of buffer
        // and to add a newline which can help with parsing
        console.log("BATCH_RESULT_START");
        console.log(JSON.stringify(output));
        console.log("BATCH_RESULT_END");

        // Add a small delay to ensure output is flushed
        await new Promise((resolve) => setTimeout(resolve, 50));
      } catch (err) {
        const errorMessage = `Error writing results to stdout: ${err.message}`;
        error(errorMessage, err);
        throw new Error(errorMessage);
      }

      return output;
    } finally {
      try {
        await pool.terminate();
      } catch (err) {
        // Log but don't throw - this is a cleanup error
        error(`Error terminating worker pool: ${err.message}`, err);
      }
    }
  } catch (err) {
    // If error doesn't have a specific message already set, add context
    if (err.message && !err.message.includes("batch processing")) {
      error(`Error in batch processing: ${err.message}`, err);
    }
    throw err;
  }
}

module.exports = {
  optimizeBatch,
};
