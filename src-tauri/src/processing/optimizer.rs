use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::{Mutex, Semaphore};
use tauri_plugin_shell::{ShellExt, process::{Command, Output}};
use tauri;
use crate::core::OptimizationResult;
use crate::worker::ImageTask;
use crate::utils::{
    OptimizerError,
    OptimizerResult,
};
use crate::processing::validation::validate_task;
use crate::benchmarking::metrics::{Duration, ProcessPoolMetrics};
use serde_json;
use tracing::{debug, warn};
use sysinfo::System;
use serde::Deserialize;
use futures::future::try_join_all;
use std::time::Instant;
use num_cpus;

/// Manages a pool of Sharp sidecar processes
#[derive(Clone)]
pub struct ProcessPool {
    semaphore: Arc<Semaphore>,
    app: tauri::AppHandle,
    max_size: usize,
    active_count: Arc<Mutex<usize>>,
    metrics: Arc<Mutex<ProcessPoolMetrics>>,
}

impl ProcessPool {
    fn calculate_optimal_processes() -> usize {
        let cpu_count = num_cpus::get();
        // Use half of CPU cores, with min 2 and max 8
        (cpu_count / 2).max(2).min(16)
    }

    pub fn new(app: tauri::AppHandle) -> Self {
        let size = Self::calculate_optimal_processes();
        debug!("Creating process pool with {} processes (based on {} CPU cores)", size, num_cpus::get());
        Self::new_with_size(app, size)
    }

    pub fn new_with_size(app: tauri::AppHandle, size: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(size)),
            app,
            max_size: size,
            active_count: Arc::new(Mutex::new(0)),
            metrics: Arc::new(Mutex::new(ProcessPoolMetrics::default())),
        }
    }
    
    pub async fn acquire(&self) -> OptimizerResult<Command> {
        let start = Instant::now();
        
        let _permit = self.semaphore.acquire().await.map_err(|e| 
            OptimizerError::sidecar(format!("Pool acquisition failed: {}", e))
        )?;
        
        // Update active count and metrics
        {
            let mut count = self.active_count.lock().await;
            *count += 1;
            
            let mut metrics = self.metrics.lock().await;
            metrics.update_active_count(*count);
        }
        
        // Create the sidecar command
        let result = self.app.shell()
            .sidecar("sharp-sidecar")
            .map_err(|e| OptimizerError::sidecar(format!("Sidecar spawn failed: {}", e)));
            
        // Record spawn metrics
        {
            let mut metrics = self.metrics.lock().await;
            metrics.record_spawn(Duration::new_unchecked(start.elapsed().as_secs_f64()));
        }
        
        result
    }
    
    pub async fn release(&self) {
        let mut count = self.active_count.lock().await;
        *count = count.saturating_sub(1);
        
        let mut metrics = self.metrics.lock().await;
        metrics.update_active_count(*count);
    }
    
    pub fn get_max_size(&self) -> usize {
        self.max_size
    }
    
    pub async fn get_metrics(&self) -> ProcessPoolMetrics {
        self.metrics.lock().await.clone()
    }
}

#[derive(Debug, Deserialize)]
struct SharpResult {
    path: String,
    optimized_size: u64,
    original_size: u64,
    saved_bytes: i64,
    compression_ratio: String,
    /// The output format of the image (e.g., 'jpeg', 'png').
    /// Kept for debugging and future format conversion tracking.
    #[allow(dead_code)]
    format: Option<String>,
    success: bool,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct BatchSizeConfig {
    min_size: usize,
    max_size: usize,
    target_memory_usage: usize,    // in bytes
    target_memory_percentage: f32, // percentage of available memory to use
    tasks_per_process: usize,      // target number of tasks per process
}

impl Default for BatchSizeConfig {
    fn default() -> Self {
        Self {
            min_size: 10,
            max_size: 100,
            target_memory_usage: 1024 * 1024 * 4096, // Increased to 4GB limit
            target_memory_percentage: 0.7,     // Increased from 0.5 to 0.7 (70% of available memory)
            tasks_per_process: 20,            // Target 20 tasks per process for better utilization
        }
    }
}

#[derive(Debug, Clone)]
pub struct BatchMemoryMetrics {
    pub initial_memory: usize,
    pub avg_batch_memory: usize,
    pub peak_pressure: usize,
    pub memory_distribution: [usize; 3],
}

impl BatchMemoryMetrics {
    fn new(initial_memory: usize) -> Self {
        Self {
            initial_memory,
            avg_batch_memory: 0,
            peak_pressure: 0,
            memory_distribution: [0; 3],
        }
    }

    fn record_usage(&mut self, used_memory: usize, available_memory: usize) {
        // Update average (exponential moving average with alpha=0.2)
        if self.avg_batch_memory == 0 {
            self.avg_batch_memory = used_memory;
        } else {
            self.avg_batch_memory = (used_memory / 5) + (self.avg_batch_memory * 4 / 5);
        }
        
        // Update peak pressure (track highest memory usage)
        self.peak_pressure = self.peak_pressure.max(used_memory);
        
        // Update distribution based on percentage of initial memory used
        let usage_pct = (used_memory as f64 / self.initial_memory as f64) * 100.0;
        let index = (usage_pct / 33.33).min(2.0) as usize;
        self.memory_distribution[index] += 1;

        debug!(
            "Memory usage recorded - Used: {}MB, Available: {}MB, Usage: {:.1}%, Index: {}", 
            used_memory / (1024 * 1024),
            available_memory / (1024 * 1024),
            usage_pct,
            index
        );
    }
}

#[derive(Clone)]
pub struct ImageOptimizer {
    active_tasks: Arc<Mutex<HashSet<String>>>,
    process_pool: ProcessPool,
}

impl ImageOptimizer {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            active_tasks: Arc::new(Mutex::new(HashSet::new())),
            process_pool: ProcessPool::new(app),
        }
    }

    pub async fn process_batch(&self, tasks: Vec<ImageTask>) -> OptimizerResult<(Vec<OptimizationResult>, BatchMemoryMetrics)> {
        let batch_size = self.calculate_batch_size(&tasks);
        let pool_size = self.process_pool.get_max_size();
        
        // Adjust batch size based on pool size to optimize process utilization
        let adjusted_batch_size = (batch_size / pool_size).max(1) * pool_size;
        debug!("Calculated optimal batch size: {} (adjusted from {} for {} processes)", 
            adjusted_batch_size, batch_size, pool_size);
        
        let mut results = Vec::with_capacity(tasks.len());
        let available_mem = self.get_available_memory();
        
        // Initialize memory metrics
        let mut memory_metrics = BatchMemoryMetrics::new(available_mem);
        
        // Get initial process metrics for benchmarking
        let process_metrics = self.process_pool.get_metrics().await;
        debug!("Initial process pool metrics - Active: {}, Total Spawns: {}", 
            process_metrics.active_processes.last().unwrap_or(&0),
            process_metrics.total_spawns
        );
        
        for (chunk_index, chunk) in tasks.chunks(adjusted_batch_size).enumerate() {
            debug!(
                "Processing chunk {}/{} ({} tasks)", 
                chunk_index + 1, 
                (tasks.len() + adjusted_batch_size - 1) / adjusted_batch_size,
                chunk.len()
            );
            
            // Record pre-processing memory state
            let pre_mem = self.get_available_memory();
            
            let chunk_results = match self.process_chunk(chunk.to_vec()).await {
                Ok(res) => {
                    debug!("Chunk {} processed successfully", chunk_index + 1);
                    res
                },
                Err(e) => {
                    warn!("Chunk {} failed: {}", chunk_index + 1, e);
                    return Err(e);
                }
            };
            
            // Record post-processing memory state and calculate metrics
            let post_mem = self.get_available_memory();
            let used_memory = pre_mem.saturating_sub(post_mem);
            memory_metrics.record_usage(used_memory, pre_mem);
            
            results.extend(chunk_results);
        }
        
        // Get final process metrics for benchmarking
        let final_metrics = self.process_pool.get_metrics().await;
        debug!("Final process pool metrics - Active: {}, Total Spawns: {}", 
            final_metrics.active_processes.last().unwrap_or(&0),
            final_metrics.total_spawns
        );
        
        // Log memory metrics summary
        debug!(
            "Memory metrics - Avg: {}MB, Initial: {}MB, Peak Pressure: {}MB", 
            memory_metrics.avg_batch_memory / (1024 * 1024),
            memory_metrics.initial_memory / (1024 * 1024),
            memory_metrics.peak_pressure / (1024 * 1024)
        );
        
        Ok((results, memory_metrics))
    }

    async fn validate_tasks(&self, tasks: &[ImageTask]) -> OptimizerResult<()> {
        debug!("Starting parallel validation of {} tasks", tasks.len());
        
        let validation_tasks: Vec<_> = tasks.iter()
            .map(|task| {
                let task = task.clone();
                tokio::spawn(async move {
                    validate_task(&task).await
                })
            })
            .collect();

        // Wait for all validations to complete
        let results = try_join_all(validation_tasks).await
            .map_err(|e| OptimizerError::processing(format!("Task validation failed: {}", e)))?;

        // Check results and collect any errors
        let errors: Vec<_> = results
            .into_iter()
            .filter_map(|r| r.err())
            .collect();

        if !errors.is_empty() {
            warn!("Validation failed for {} tasks", errors.len());
            return Err(OptimizerError::processing(format!(
                "Validation failed for {} tasks: {:?}",
                errors.len(),
                errors
            )));
        }

        debug!("All {} tasks validated successfully", tasks.len());
        Ok(())
    }

    async fn process_chunk(
        &self,
        tasks: Vec<ImageTask>,
    ) -> OptimizerResult<Vec<OptimizationResult>> {
        let mut results = Vec::with_capacity(tasks.len());

        debug!("Validating {} tasks in chunk", tasks.len());
        // Validate all tasks in parallel
        self.validate_tasks(&tasks).await?;
        
        // Track active tasks
        {
            let mut active = self.active_tasks.lock().await;
            for task in &tasks {
                active.insert(task.input_path.clone());
            }
            debug!("Tracked {} tasks for processing", tasks.len());
        }

        // Process the batch using Sharp sidecar
        debug!("Processing batch with Sharp sidecar - {} tasks", tasks.len());
        let result = self.run_sharp_process_batch(&tasks).await?;
        let (sharp_result, output) = result;

        // Remove from active tasks
        {
            let mut active = self.active_tasks.lock().await;
            for task in &tasks {
                active.remove(&task.input_path);
            }
        }

        match sharp_result {
            Ok(_) => {
                debug!("Collecting results for {} tasks", tasks.len());
                // Parse Sharp output to get results
                let stdout = String::from_utf8_lossy(&output.stdout);
                let sharp_results: Vec<SharpResult> = serde_json::from_str(&stdout)
                    .map_err(|e| OptimizerError::processing(format!(
                        "Failed to parse Sharp output: {}", e
                    )))?;

                // Collect results for each task
                let mut total_original = 0;
                let mut total_optimized = 0;
                
                for (task, result) in tasks.into_iter().zip(sharp_results) {
                    total_original += result.original_size;
                    total_optimized += result.optimized_size;

                    results.push(OptimizationResult {
                        original_path: task.input_path,
                        optimized_path: result.path,
                        original_size: result.original_size,
                        optimized_size: result.optimized_size,
                        success: result.success,
                        error: result.error,
                        saved_bytes: result.saved_bytes,
                        compression_ratio: result.compression_ratio.parse().unwrap_or(0.0),
                    });
                }
                
                let total_saved_percentage = ((total_original - total_optimized) as f64 / total_original as f64 * 100.0).round();
                debug!(
                    "Batch completed - Tasks: {}, Total Original: {}, Total Optimized: {}, Average Savings: {}%",
                    results.len(),
                    total_original,
                    total_optimized,
                    total_saved_percentage
                );
                Ok(results)
            }
            Err(e) => Err(e),
        }
    }

    async fn run_sharp_process_batch(
        &self,
        tasks: &[ImageTask],
    ) -> OptimizerResult<(OptimizerResult<()>, Output)> {
        // Acquire a process from the pool
        let cmd = self.process_pool.acquire().await?;
        
        // Create batch task data
        let batch_data = tasks.iter().map(|task| {
            serde_json::json!({
                "input": task.input_path,
                "output": task.output_path,
                "settings": task.settings
            })
        }).collect::<Vec<_>>();

        debug!("Prepared {} tasks for Sharp processing", tasks.len());

        let batch_json = serde_json::to_string(&batch_data)
            .map_err(|e| OptimizerError::processing(format!("Failed to serialize batch settings: {}", e)))?;
        
        // Run the command
        let output = cmd
            .args(&["optimize-batch", &batch_json])
            .output()
            .await
            .map_err(|e| OptimizerError::sidecar(format!("Failed to run Sharp: {}", e)))?;
        
        // Release the process back to the pool
        self.process_pool.release().await;
        
        // Check for success and validate output
        if output.status.success() {
            debug!("Sharp sidecar command completed successfully");
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            if !stdout.is_empty() {
                // Parse Sharp output JSON
                let sharp_results: Vec<serde_json::Value> = serde_json::from_str(&stdout)
                    .map_err(|e| OptimizerError::processing(format!(
                        "Failed to parse Sharp output: {}", e
                    )))?;
                
                debug!("Sharp sidecar processed {} results", sharp_results.len());
                
                // Verify each result matches a task and the file exists
                for (task, result) in tasks.iter().zip(sharp_results.iter()) {
                    let output_path = result["path"].as_str().ok_or_else(|| 
                        OptimizerError::processing("Missing path in Sharp output".to_string())
                    )?;
                    
                    let success = result["success"].as_bool().unwrap_or(false);
                    if !success {
                        let error = result["error"].as_str().unwrap_or("Unknown error");
                        let error_message = format!(
                            "Sharp processing failed for {}: {}", 
                            task.input_path, error
                        );
                        warn!("{}", error_message);
                        return Ok((Err(OptimizerError::processing(error_message)), output));
                    }
                    
                    // Verify the output file exists
                    match tokio::fs::metadata(output_path).await {
                        Ok(_) => {},
                        Err(e) => {
                            let error_message = format!(
                                "Output file missing: {} (Error: {})", output_path, e
                            );
                            warn!("{}", error_message);
                            return Ok((Err(OptimizerError::processing(error_message)), output));
                        }
                    }
                }
                
                Ok((Ok(()), output))
            } else {
                let error_message = "Empty output from Sharp sidecar".to_string();
                warn!("{}", error_message);
                Ok((Err(OptimizerError::processing(error_message)), output))
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let error_message = format!("Sharp process failed: {}", stderr);
            warn!("{}", error_message);
            Ok((Err(OptimizerError::sidecar(error_message)), output))
        }
    }

    pub async fn get_active_tasks(&self) -> Vec<String> {
        self.active_tasks.lock().await.iter().cloned().collect()
    }

    fn get_available_memory(&self) -> usize {
        let mut system = System::new();
        system.refresh_memory();
        system.available_memory() as usize
    }

    fn calculate_batch_size(&self, tasks: &[ImageTask]) -> usize {
        let config = BatchSizeConfig::default();
        let process_count = self.process_pool.get_max_size();
        
        // Calculate total and average task size
        let total_size: u64 = tasks.iter()
            .filter_map(|t| std::fs::metadata(&t.input_path).ok())
            .map(|m| m.len())
            .sum();
        
        let avg_size = if tasks.is_empty() {
            0
        } else {
            total_size / tasks.len() as u64
        };
        
        // Get available system memory and calculate target
        let available_mem = self.get_available_memory();
        let memory_target = ((available_mem as f32 * config.target_memory_percentage) as usize)
            .min(config.target_memory_usage);
        
        // Calculate batch sizes based on different criteria
        let memory_based_size = if avg_size == 0 {
            config.max_size
        } else {
            memory_target / avg_size as usize
        };
        
        // Calculate process-based size
        let process_based_size = config.tasks_per_process * process_count;
        
        // Consider total tasks - don't make batches larger than needed
        let task_based_size = tasks.len();
        
        // Take the minimum of all calculated sizes
        let calculated_size = memory_based_size
            .min(process_based_size)
            .min(task_based_size);
        
        // Clamp between min and max sizes
        let batch_size = calculated_size.clamp(config.min_size, config.max_size);
        
        debug!(
            "Batch size calculation: avg_size={}MB, memory_target={}MB, process_count={}, tasks_per_process={}, memory_based={}, process_based={}, task_based={}, final={}",
            avg_size / (1024 * 1024),
            memory_target / (1024 * 1024),
            process_count,
            config.tasks_per_process,
            memory_based_size,
            process_based_size,
            task_based_size,
            batch_size
        );
        
        batch_size
    }
} 