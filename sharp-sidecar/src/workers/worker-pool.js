const { Worker } = require('worker_threads');
const os = require('os');
const path = require('path');

/**
 * Manages a pool of Sharp worker threads for parallel image processing
 */
class SharpWorkerPool {
  /**
   * Creates a new worker pool
   * @param {number} maxWorkers - Maximum number of workers to create (defaults to CPU count)
   */
  constructor(maxWorkers = os.cpus().length) {
    this.maxWorkers = maxWorkers;
    this.workers = new Array(this.maxWorkers).fill(null).map(() => 
      new Worker(path.join(__dirname, '../../index.js'), { 
        workerData: { isWorker: true }
      })
    );
    this.taskQueue = [];
    this.activeWorkers = 0;
  }

  /**
   * Process a batch of image optimization tasks
   * @param {Array} batchData - Array of image processing tasks
   * @returns {Promise<Array>} Array of processed results
   */
  async processBatch(batchData) {
    return new Promise((resolve, reject) => {
      console.error(`Starting batch processing with ${this.maxWorkers} workers`);
      const batchSize = batchData.length;
      const chunkSize = Math.ceil(batchSize / this.maxWorkers);
      const results = new Array(batchSize);
      let completedTasks = 0;

      const chunks = [];
      for (let i = 0; i < batchData.length; i += chunkSize) {
        chunks.push(batchData.slice(i, i + chunkSize));
      }

      console.error(`Split batch into ${chunks.length} chunks of size ${chunkSize}`);

      chunks.forEach((chunk, workerIndex) => {
        const worker = this.workers[workerIndex];
        console.error(`Assigning ${chunk.length} tasks to worker ${workerIndex}`);
        
        worker.postMessage({
          type: 'process',
          tasks: chunk
        });

        worker.on('message', (workerResults) => {
          console.error(`Received results from worker ${workerIndex}: ${workerResults.length} items`);
          
          // Store results in correct order
          workerResults.forEach((result, index) => {
            const globalIndex = workerIndex * chunkSize + index;
            if (globalIndex < batchSize) {
              results[globalIndex] = result;
            }
          });

          completedTasks += workerResults.length;
          console.error(`Completed ${completedTasks}/${batchSize} tasks`);
          
          if (completedTasks >= batchSize) {
            // Filter out any undefined results
            const finalResults = results.filter(r => r !== undefined);
            console.error(`Batch processing complete. Final results: ${finalResults.length} items`);
            resolve(finalResults);
          }
        });

        worker.on('error', (error) => {
          console.error(`Worker ${workerIndex} error:`, error);
          reject(error);
        });

        worker.on('exit', (code) => {
          if (code !== 0) {
            console.error(`Worker ${workerIndex} exited with code ${code}`);
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
    console.error('Terminating worker pool');
    return Promise.all(this.workers.map(worker => worker.terminate()));
  }
}

module.exports = SharpWorkerPool; 