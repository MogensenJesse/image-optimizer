const { Worker } = require('worker_threads');
const os = require('os');
const path = require('path');
const debug = require('debug')('sharp-sidecar:worker-pool');
const error = require('debug')('sharp-sidecar:worker-pool:error');
const { createProgressUpdate } = require('../utils/progress');

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
    this.workers = [];
    this.metrics = {
      startTime: 0,
      completedTasks: 0,
      totalTasks: 0,
      queueLength: 0,
      worker_count: maxWorkers,
      active_workers: 0,
      tasks_per_worker: []
    };
    this.lastProgressPercentage = 0;
    this.lastProgressTime = Date.now();
    this.progressThrottleMs = 500; // Minimum 500ms between updates
    this.initializeWorkers();
  }

  /**
   * Initialize workers
   */
  initializeWorkers() {
    this.workers = new Array(this.maxWorkers).fill(null).map((_, index) => 
      new Worker(path.join(__dirname, '../../index.js'), { 
        workerData: { 
          isWorker: true,
          workerId: index 
        }
      })
    );
  }

  /**
   * Get current worker pool metrics
   * @returns {Object} Current metrics state
   */
  getMetrics() {
    const endTime = Date.now();
    const duration = this.metrics.startTime ? (endTime - this.metrics.startTime) / 1000 : 0;

    return {
      worker_count: this.metrics.worker_count,
      tasks_per_worker: this.metrics.tasks_per_worker,
    };
  }

  /**
   * Send a throttled progress update
   * Only sends updates when the percentage changes significantly or after time threshold
   */
  sendProgressUpdate() {
    const currentTime = Date.now();
    const currentPercentage = Math.floor((this.metrics.completedTasks / this.metrics.totalTasks) * 100);
    const timeSinceLastUpdate = currentTime - this.lastProgressTime;
    
    // Send update if:
    // 1. Percentage has increased, AND
    // 2. Either:
    //    a. It's been at least progressThrottleMs since last update, OR
    //    b. It's a milestone percentage (0%, 25%, 50%, 75%, 100%)
    const isMilestone = currentPercentage % 25 === 0 || currentPercentage === 100;
    
    if (currentPercentage > this.lastProgressPercentage && 
        (timeSinceLastUpdate >= this.progressThrottleMs || isMilestone)) {
      
      this.lastProgressPercentage = currentPercentage;
      this.lastProgressTime = currentTime;
      
      // Use the new unified format
      const progressUpdate = createProgressUpdate(
        this.metrics.completedTasks,
        this.metrics.totalTasks,
        'processing'
      );
      
      console.log(JSON.stringify(progressUpdate));
    }
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
      this.metrics.tasks_per_worker = chunks.map(chunk => chunk.length);
      this.metrics.active_workers = chunks.length;

      debug(`Split batch into ${chunks.length} chunks of size ${chunkSize}`);

      chunks.forEach((chunk, workerIndex) => {
        const worker = this.workers[workerIndex];
        debug(`Assigning ${chunk.length} tasks to worker ${workerIndex}`);
        
        worker.postMessage({
          type: 'process',
          tasks: chunk
        });

        worker.on('message', (message) => {
          if (message.type === 'progress') {
            // Forward progress messages to stdout for Rust to parse
            console.log(JSON.stringify({
              type: 'progress',
              progressType: message.progressType,
              taskId: message.taskId,
              workerId: workerIndex,
              result: message.result,
              error: message.error,
              metrics: {
                completedTasks: this.metrics.completedTasks,
                totalTasks: this.metrics.totalTasks,
                queueLength: this.metrics.queueLength
              }
            }));
            
            // If this is a completion message, update metrics and send a throttled update
            if (message.progressType === 'complete') {
              this.metrics.completedTasks += 1;
              this.metrics.queueLength = this.metrics.totalTasks - this.metrics.completedTasks;
              this.sendProgressUpdate();
            }
          } else if (message.type === 'results') {
            debug(`Received results from worker ${workerIndex}: ${message.results.length} items`);
            
            // Store results in correct order
            message.results.forEach((result, index) => {
              const globalIndex = workerIndex * chunkSize + index;
              if (globalIndex < batchSize) {
                results[globalIndex] = result;
              }
            });

            // Update metrics
            this.metrics.completedTasks += message.results.length;
            this.metrics.queueLength = batchSize - this.metrics.completedTasks;
            
            debug(`Completed ${this.metrics.completedTasks}/${batchSize} tasks`);
            
            if (this.metrics.completedTasks >= batchSize) {
              // Filter out any undefined results
              const finalResults = results.filter(r => r !== undefined);
              debug(`Batch processing complete. Final results: ${finalResults.length} items`);
              
              // Send final progress update with 100%
              const finalProgressUpdate = createProgressUpdate(
                this.metrics.totalTasks,
                this.metrics.totalTasks,
                'complete'
              );
              
              console.log(JSON.stringify(finalProgressUpdate));
              
              // Return both results and metrics
              const finalMetrics = this.getMetrics();
              debug(`Final metrics: ${JSON.stringify(finalMetrics)}`);
              
              resolve({
                results: finalResults,
                metrics: finalMetrics
              });
            }
          } else if (message.type === 'error') {
            error(`Worker ${workerIndex} error:`, message.error);
            
            // Send error progress update
            const errorProgressUpdate = createProgressUpdate(
              this.metrics.completedTasks,
              this.metrics.totalTasks,
              'error'
            );
            
            console.log(JSON.stringify(errorProgressUpdate));
          }
        });

        worker.on('error', (err) => {
          error(`Worker ${workerIndex} error:`, err);
          
          // Send error progress update
          const workerErrorUpdate = createProgressUpdate(
            this.metrics.completedTasks,
            this.metrics.totalTasks,
            'error'
          );
          
          console.log(JSON.stringify(workerErrorUpdate));
          
          reject(err);
        });

        worker.on('exit', (code) => {
          if (code !== 0) {
            error(`Worker ${workerIndex} exited with code ${code}`);
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