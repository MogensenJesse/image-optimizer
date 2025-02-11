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
 * @returns {Promise<Array>} Array of processing results
 */
async function optimizeBatch(batchJson) {
  if (!isMainThread) return;
  
  try {
    debug('Starting batch optimization');
    const batch = JSON.parse(batchJson);
    progress('Batch', `Processing ${batch.length} images`);
    
    const pool = new SharpWorkerPool();
    try {
      const results = await pool.processBatch(batch);
      if (!results || results.length === 0) {
        error('No results returned from worker pool');
        process.exit(1);
      }
      progress('Complete', `Successfully processed ${results.length} images`);
      // Ensure results are written to stdout
      process.stdout.write(JSON.stringify(results));
      return results;
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