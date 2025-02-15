/**
 * Batch processing functionality
 * @module processing/batch
 */

const { isMainThread } = require('worker_threads');
const SharpWorkerPool = require('../workers/worker-pool');
const { error, debug, progress } = require('../utils');

/**
 * Process a batch of images
 * @param {string} batchJson - JSON string containing batch processing tasks
 * @returns {Promise<Object>} Object containing results array and metrics
 */
async function optimizeBatch(batchJson) {
  if (!isMainThread) return;
  
  try {
    debug('Starting batch optimization');
    const batch = JSON.parse(batchJson);
    progress('Batch', `Processing ${batch.length} images`);
    
    const pool = new SharpWorkerPool();
    try {
      const { results, metrics } = await pool.processBatch(batch);
      
      if (!results || results.length === 0) {
        error('No results returned from worker pool');
        process.exit(1);
      }

      progress('Complete', `Successfully processed ${results.length} images`);
      debug('Worker pool metrics:', metrics);

      // Output both results and metrics
      const output = {
        results,
        metrics
      };
      process.stdout.write(JSON.stringify(output));
      return output;
    } finally {
      await pool.terminate();
    }
  } catch (err) {
    error('Error in batch processing:', err);
    process.exit(1);
  }
}

module.exports = {
  optimizeBatch
}; 