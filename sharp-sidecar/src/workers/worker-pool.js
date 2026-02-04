const { Worker } = require("node:worker_threads");
const os = require("node:os");
const path = require("node:path");
const { debug, error } = require("../utils");
const {
  createProgressUpdate,
  createDetailedProgressUpdate,
  formatBytes,
} = require("../utils/progress");

// Optimize UV_THREADPOOL_SIZE for systems with > 4 cores
// This affects libuv's thread pool used by Sharp for async I/O
// Default is 4, but should match or exceed CPU count for optimal parallelism
const cpuCount = os.cpus().length;
if (!process.env.UV_THREADPOOL_SIZE && cpuCount > 4) {
  // Note: This must be set before any async I/O operations
  // Since this file is loaded early, we set it here
  process.env.UV_THREADPOOL_SIZE = String(Math.min(cpuCount, 128));
  debug(`Set UV_THREADPOOL_SIZE to ${process.env.UV_THREADPOOL_SIZE}`);
}

/**
 * Manages a pool of Sharp worker threads for parallel image processing.
 *
 * Note: This pool is designed for single-use per batch. Event listeners are
 * attached in processBatch() and the pool should be terminated after use
 * to clean up workers and listeners. Create a new pool for each batch.
 */
class SharpWorkerPool {
  /**
   * Creates a new worker pool
   * @param {number} taskCount - Number of tasks to process (used to limit worker count)
   */
  constructor(taskCount = os.cpus().length) {
    // Don't create more workers than tasks or CPU cores
    this.maxWorkers = Math.min(taskCount, os.cpus().length);
    this.workers = [];
    this.metrics = {
      startTime: 0,
      completedTasks: 0,
      totalTasks: 0,
      queueLength: 0,
      worker_count: this.maxWorkers,
      active_workers: 0,
      tasks_per_worker: [],
    };
    this.initializeWorkers();
  }

  /**
   * Initialize workers
   */
  initializeWorkers() {
    this.workers = new Array(this.maxWorkers).fill(null).map(
      (_, index) =>
        new Worker(path.join(__dirname, "../../index.js"), {
          workerData: {
            isWorker: true,
            workerId: index,
          },
        }),
    );
  }

  /**
   * Get current worker pool metrics
   * @returns {Object} Current metrics state
   */
  getMetrics() {
    const endTime = Date.now();
    const _duration = this.metrics.startTime
      ? (endTime - this.metrics.startTime) / 1000
      : 0;

    return {
      worker_count: this.metrics.worker_count,
      tasks_per_worker: this.metrics.tasks_per_worker,
    };
  }

  /**
   * Send a progress update
   * Always sends updates without throttling
   */
  sendProgressUpdate() {
    // Ensure completedTasks doesn't exceed totalTasks for progress reporting
    const reportedCompletedTasks = Math.min(
      this.metrics.completedTasks,
      this.metrics.totalTasks,
    );
    const _currentPercentage = Math.floor(
      (reportedCompletedTasks / this.metrics.totalTasks) * 100,
    );

    // Always send update without throttling
    // Use the new unified format
    const progressUpdate = createProgressUpdate(
      reportedCompletedTasks, // Use the capped value
      this.metrics.totalTasks,
      "processing",
    );

    console.log(JSON.stringify(progressUpdate));
  }

  /**
   * Process a batch of image optimization tasks
   * @param {Array} batchData - Array of image processing tasks
   * @returns {Promise<Object>} Object containing results and metrics
   */
  async processBatch(batchData) {
    return new Promise((resolve, reject) => {
      debug(`Starting batch processing with ${this.maxWorkers} workers`);
      const batchSize = batchData.length;
      const chunkSize = Math.ceil(batchSize / this.maxWorkers);
      const results = new Array(batchSize);

      // Initialize metrics for this batch
      this.metrics.startTime = Date.now();
      this.metrics.completedTasks = 0;
      this.metrics.totalTasks = batchSize;
      this.metrics.queueLength = batchSize;

      const chunks = [];
      for (let i = 0; i < batchData.length; i += chunkSize) {
        chunks.push(batchData.slice(i, i + chunkSize));
      }

      // Update tasks per worker metrics
      this.metrics.tasks_per_worker = chunks.map((chunk) => chunk.length);
      this.metrics.active_workers = chunks.length;

      debug(`Split batch into ${chunks.length} chunks of size ${chunkSize}`);

      // Keep track of finished workers to ensure we wait for all
      let finishedWorkers = 0;
      const totalWorkersWithTasks = chunks.length;

      chunks.forEach((chunk, workerIndex) => {
        const worker = this.workers[workerIndex];
        debug(`Assigning ${chunk.length} tasks to worker ${workerIndex}`);

        worker.postMessage({
          type: "process",
          tasks: chunk,
        });

        worker.on("message", (message) => {
          if (message.type === "progress") {
            // Calculate safe metrics for reporting
            const safeCompletedTasks = Math.min(
              this.metrics.completedTasks,
              this.metrics.totalTasks,
            );
            const safeQueueLength = Math.max(
              0,
              this.metrics.totalTasks - safeCompletedTasks,
            );

            // Only forward progress messages to stdout if they are "start" messages
            // We skip the "complete" messages since the backend doesn't need these individual messages
            if (message.progressType === "start") {
              console.log(
                JSON.stringify({
                  type: "progress",
                  progressType: message.progressType,
                  taskId: message.taskId,
                  workerId: workerIndex,
                  result: message.result,
                  error: message.error,
                  metrics: {
                    completedTasks: safeCompletedTasks,
                    totalTasks: this.metrics.totalTasks,
                    queueLength: safeQueueLength,
                  },
                }),
              );
            }

            // If this is a completion message, update metrics and send a progress update
            // but don't emit the individual completion message
            if (message.progressType === "complete") {
              // Store the result in memory for tallying final results
              const result = message.result;

              // Update the counters
              this.metrics.completedTasks += 1;
              // Ensure queueLength doesn't go negative
              this.metrics.queueLength = Math.max(
                0,
                this.metrics.totalTasks - this.metrics.completedTasks,
              );

              // Extract the file name from the task ID
              const fileName = path.basename(message.taskId);

              // Log completion for the current task
              debug(
                `Completed ${this.metrics.completedTasks}/${this.metrics.totalTasks} tasks`,
              );

              // Create detailed progress update with optimization metrics
              if (result) {
                const progressDetailedUpdate = createDetailedProgressUpdate(
                  fileName,
                  message.taskId,
                  result,
                  {
                    completedTasks: this.metrics.completedTasks,
                    totalTasks: this.metrics.totalTasks,
                  },
                );

                // Log the formatted message for debugging
                debug(`${progressDetailedUpdate.formattedMessage}`);

                // Send the detailed update as a normal console.log message
                console.log(JSON.stringify(progressDetailedUpdate));

                // Log detailed optimization for debugging
                debug(
                  `Worker ${workerIndex} completed ${fileName}: ${formatBytes(result.original_size)} → ${formatBytes(result.optimized_size)} (${result.compression_ratio}% reduction)`,
                );
              } else {
                // Only send a standard progress update if we couldn't send a detailed one
                this.sendProgressUpdate();
              }

              // Log completion for debugging, but don't emit to stdout for the backend
              debug(
                `Worker ${workerIndex} completed ${message.taskId} - ${result ? `${result.compression_ratio}% reduction` : "no result"}`,
              );
            }
          } else if (message.type === "results") {
            debug(
              `Received results from worker ${workerIndex}: ${message.results.length} items`,
            );

            // Store results in correct order
            message.results.forEach((result, index) => {
              const globalIndex = workerIndex * chunkSize + index;
              if (globalIndex < batchSize) {
                results[globalIndex] = result;
              }
            });

            // Increment the count of workers that have finished
            finishedWorkers++;
            debug(
              `Worker ${workerIndex} finished. ${finishedWorkers}/${totalWorkersWithTasks} workers completed.`,
            );

            // Update queue length but ensure it doesn't go negative
            this.metrics.queueLength = Math.max(
              0,
              batchSize - this.metrics.completedTasks,
            );

            debug(
              `Completed ${Math.min(this.metrics.completedTasks, batchSize)}/${batchSize} tasks`,
            );

            // Only finalize when ALL workers have completed their tasks
            if (finishedWorkers >= totalWorkersWithTasks) {
              debug(
                `All ${totalWorkersWithTasks} workers have completed their tasks.`,
              );

              // Filter out any undefined results
              const finalResults = results.filter((r) => r !== undefined);
              debug(
                `Batch processing complete. Final results: ${finalResults.length} items`,
              );

              // Send final progress update with 100%
              const finalProgressUpdate = createProgressUpdate(
                this.metrics.totalTasks,
                this.metrics.totalTasks,
                "complete",
              );

              console.log(JSON.stringify(finalProgressUpdate));

              // Return both results and metrics
              const finalMetrics = this.getMetrics();
              debug(`Final metrics: ${JSON.stringify(finalMetrics)}`);

              resolve({
                results: finalResults,
                metrics: finalMetrics,
              });
            }
          } else if (message.type === "error") {
            error(`Worker ${workerIndex} error:`, message.error);

            // Send error progress update
            const errorProgressUpdate = createProgressUpdate(
              this.metrics.completedTasks,
              this.metrics.totalTasks,
              "error",
            );

            console.log(JSON.stringify(errorProgressUpdate));
          }
        });

        worker.on("error", (err) => {
          error(`Worker ${workerIndex} error:`, err);

          // Send error progress update
          const workerErrorUpdate = createProgressUpdate(
            this.metrics.completedTasks,
            this.metrics.totalTasks,
            "error",
          );

          console.log(JSON.stringify(workerErrorUpdate));

          reject(err);
        });

        worker.on("exit", (code) => {
          // Code 1 is expected when worker.terminate() is called
          // Only log unexpected exit codes (not 0 or 1)
          if (code !== 0 && code !== 1) {
            error(
              `Worker ${workerIndex} exited unexpectedly with code ${code}`,
            );
          }
        });
      });
    });
  }

  /**
   * Gracefully terminate all workers in the pool
   * @returns {Promise<void>}
   */
  terminate() {
    debug("Terminating worker pool");
    return Promise.all(this.workers.map((worker) => worker.terminate()));
  }
}

module.exports = SharpWorkerPool;
