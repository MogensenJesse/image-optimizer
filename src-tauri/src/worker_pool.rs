use crate::commands::image::{ImageSettings, OptimizationResult};
use crossbeam_channel::{bounded, Receiver, Sender};
use tauri_plugin_shell::ShellExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Instant;
use sysinfo::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageTask {
    pub input_path: String,
    pub output_path: String,
    pub settings: ImageSettings,
    pub priority: u8,
}

pub struct WorkerPool {
    workers: Vec<Worker>,
    task_sender: Sender<ImageTask>,
    result_receiver: Receiver<OptimizationResult>,
    active_tasks: Arc<Mutex<usize>>,
    metrics: Arc<Mutex<Vec<WorkerMetrics>>>,
    sys: Arc<Mutex<System>>,
}

struct Worker {
    id: usize,
    handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProcessingProgress {
    total_files: usize,
    processed_files: usize,
    current_file: String,
    elapsed_time: f64,
    bytes_processed: u64,
    bytes_saved: i64,
    estimated_time_remaining: f64,
    active_workers: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkerMetrics {
    pub cpu_usage: f64,
    pub thread_id: usize,
    pub task_count: usize,
    pub avg_processing_time: f64,
}

impl WorkerPool {
    pub fn new(size: usize, app: tauri::AppHandle) -> Self {
        println!("Initializing WorkerPool with {} workers", size);
        let mut sys = System::new_all();
        sys.refresh_all();
        
        // Get memory in GB and calculate buffer size
        let total_memory_gb = sys.total_memory() / (1024 * 1024);
        println!("Detected system memory: {}GB", total_memory_gb);
        
        // More aggressive buffer sizing based on memory
        let buffer_size = if total_memory_gb == 0 || total_memory_gb > 1024 {
            println!("Warning: Memory detection unreliable, using conservative settings");
            size * 2
        } else {
            // Calculate buffer size based on memory and CPU cores
            let base_size = if total_memory_gb < 8 {
                size * 2
            } else if total_memory_gb < 16 {
                size * 3
            } else {
                size * 4
            };

            // Adjust based on CPU frequency
            let cpus = sys.cpus();
            let avg_freq = cpus.iter()
                .map(|cpu| cpu.frequency())
                .sum::<u64>() / cpus.len() as u64;
            
            if avg_freq > 3000 { // > 3GHz
                base_size + size
            } else {
                base_size
            }
        };
        println!("Using buffer size: {} (based on memory and CPU)", buffer_size);

        let (task_sender, task_receiver) = bounded::<ImageTask>(buffer_size);
        let (result_sender, result_receiver) = bounded::<OptimizationResult>(buffer_size);
        let active_tasks = Arc::new(Mutex::new(0));
        let metrics = Arc::new(Mutex::new(Vec::with_capacity(size)));
        let sys = Arc::new(Mutex::new(sys));

        let mut workers = Vec::with_capacity(size);
        
        for id in 0..size {
            println!("Spawning worker {}", id);
            let task_rx = task_receiver.clone();
            let result_tx = result_sender.clone();
            let app = app.clone();
            let active_tasks = Arc::clone(&active_tasks);
            let metrics = Arc::clone(&metrics);
            let sys = Arc::clone(&sys);

            let handle = tokio::spawn(async move {
                println!("Worker {} started", id);
                let mut task_count = 0;
                let mut total_time = 0.0;
                let mut consecutive_long_tasks = 0;

                while let Ok(task) = task_rx.recv() {
                    println!("Worker {} received task: {}", id, task.input_path);
                    let start_time = std::time::Instant::now();
                    
                    // Update CPU usage and check workload
                    {
                        let mut sys = sys.lock().await;
                        sys.refresh_cpu_usage();
                        let cpu_usage = sys.cpus()[id].cpu_usage() as f64;
                        println!("Worker {} CPU usage: {:.1}%", id, cpu_usage);
                        
                        let mut metrics = metrics.lock().await;
                        if metrics.len() <= id {
                            metrics.push(WorkerMetrics {
                                cpu_usage,
                                thread_id: id,
                                task_count: task_count,
                                avg_processing_time: 0.0,
                            });
                        } else {
                            metrics[id].cpu_usage = cpu_usage;
                        }

                        // If CPU usage is high and we have consecutive long tasks,
                        // take a short break
                        if cpu_usage > 90.0 && consecutive_long_tasks > 2 {
                            println!("Worker {} taking a brief break due to high CPU usage", id);
                            drop(sys);
                            drop(metrics);
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                            consecutive_long_tasks = 0;
                        }
                    }

                    let mut active = active_tasks.lock().await;
                    *active += 1;
                    println!("Worker {} started processing. Active tasks: {}", id, *active);
                    drop(active);

                    println!("Worker {} processing image: {}", id, task.input_path);
                    let result = tokio::time::timeout(
                        std::time::Duration::from_secs(30),
                        process_image(&app, task)
                    ).await;

                    let processing_time = start_time.elapsed().as_secs_f64();
                    
                    // Track long-running tasks
                    if processing_time > 2.5 { // Higher than average
                        consecutive_long_tasks += 1;
                    } else {
                        consecutive_long_tasks = 0;
                    }

                    task_count += 1;
                    total_time += processing_time;

                    match result {
                        Ok(Ok(result)) => {
                            println!("Worker {} successfully processed image. Size reduction: {} bytes", 
                                id, result.saved_bytes);
                            let _ = result_tx.send(result);
                        }
                        Ok(Err(e)) => {
                            eprintln!("Worker {} error processing image: {}", id, e);
                        }
                        Err(_) => {
                            eprintln!("Worker {} image processing timed out", id);
                        }
                    }

                    let mut active = active_tasks.lock().await;
                    *active -= 1;
                    println!("Worker {} finished processing. Active tasks: {}", id, *active);

                    {
                        let mut metrics = metrics.lock().await;
                        if let Some(metric) = metrics.get_mut(id) {
                            metric.task_count = task_count;
                            metric.avg_processing_time = total_time / task_count as f64;
                            println!("Worker {} metrics - Tasks: {}, Avg Time: {:.2}s (Last: {:.2}s)", 
                                id, task_count, metric.avg_processing_time, processing_time);
                        }
                    }

                    // Adaptive cooldown based on processing time
                    let cooldown = if processing_time > 2.5 {
                        20
                    } else if processing_time > 2.0 {
                        15
                    } else {
                        10
                    };
                    tokio::time::sleep(std::time::Duration::from_millis(cooldown)).await;
                }
                println!("Worker {} channel closed, shutting down", id);
            });

            workers.push(Worker { id, handle });
        }

        WorkerPool {
            workers,
            task_sender,
            result_receiver,
            active_tasks,
            metrics,
            sys,
        }
    }

    pub async fn process(&self, task: ImageTask) -> Result<OptimizationResult, String> {
        self.task_sender.send(task).map_err(|e| e.to_string())?;
        self.result_receiver.recv().map_err(|e| e.to_string())
    }

    pub async fn active_tasks(&self) -> usize {
        *self.active_tasks.lock().await
    }

    pub async fn process_batch(
        &self,
        tasks: Vec<ImageTask>,
        progress_callback: impl Fn(ProcessingProgress) + Send + 'static,
    ) -> Result<Vec<OptimizationResult>, String> {
        println!("Starting batch processing of {} tasks", tasks.len());
        let start_time = Instant::now();
        let total_tasks = tasks.len();
        let mut results = Vec::with_capacity(total_tasks);
        let mut processed = 0;
        let mut bytes_processed = 0;
        let mut bytes_saved = 0;
        let mut failed_tasks = Vec::new();
        
        // Queue tasks with backpressure
        println!("Queueing tasks with backpressure...");
        for (task_idx, task) in tasks.iter().enumerate() {
            let file_name = task.input_path.split(['/', '\\']).last()
                .unwrap_or(&task.input_path).to_string();
            
            println!("Queueing task {}/{}: {} (channel capacity: {:?})", 
                task_idx + 1, total_tasks, file_name, self.task_sender.capacity());

            // Wait for space in the channel if it's full
            while self.task_sender.is_full() {
                println!("Channel full, waiting for space...");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                
                // Try to collect any available results while waiting
                match self.result_receiver.try_recv() {
                    Ok(result) => {
                        processed += 1;
                        bytes_processed += result.original_size;
                        bytes_saved += result.saved_bytes;
                        println!("Collected result while waiting: {} (saved: {} bytes)", 
                            result.path, result.saved_bytes);
                        results.push(result);
                    }
                    Err(crossbeam_channel::TryRecvError::Empty) => (),
                    Err(e) => eprintln!("Error receiving result while waiting: {}", e),
                }
            }

            match self.task_sender.send(task.clone()) {
                Ok(_) => println!("Successfully queued: {}", file_name),
                Err(e) => {
                    eprintln!("Failed to queue task: {}", e);
                    failed_tasks.push((task.clone(), format!("Queue error: {}", e)));
                    continue;
                }
            }

            let progress = ProcessingProgress {
                total_files: total_tasks,
                processed_files: processed,
                current_file: file_name.clone(),
                elapsed_time: start_time.elapsed().as_secs_f64(),
                bytes_processed,
                bytes_saved,
                estimated_time_remaining: if processed > 0 {
                    (start_time.elapsed().as_secs_f64() / processed as f64) * (total_tasks - processed) as f64
                } else {
                    0.0
                },
                active_workers: *self.active_tasks.lock().await,
            };
            progress_callback(progress);
        }

        // Collect remaining results
        println!("All tasks queued, collecting remaining results...");
        let mut remaining_tasks = total_tasks - processed - failed_tasks.len();
        println!("Remaining tasks to collect: {}", remaining_tasks);
        
        while remaining_tasks > 0 {
            println!("Waiting for result {}/{} (remaining: {})", 
                processed + 1, total_tasks, remaining_tasks);

            match tokio::time::timeout(
                std::time::Duration::from_secs(60),
                async { self.result_receiver.recv().map_err(|e| e.to_string()) }
            ).await {
                Ok(Ok(result)) => {
                    processed += 1;
                    remaining_tasks -= 1;
                    bytes_processed += result.original_size;
                    bytes_saved += result.saved_bytes;
                    println!("Received result {}/{}: saved {} bytes (remaining: {})", 
                        processed, total_tasks, result.saved_bytes, remaining_tasks);
                    
                    let progress = ProcessingProgress {
                        total_files: total_tasks,
                        processed_files: processed,
                        current_file: result.path.clone(),
                        elapsed_time: start_time.elapsed().as_secs_f64(),
                        bytes_processed,
                        bytes_saved,
                        estimated_time_remaining: if processed > 0 {
                            (start_time.elapsed().as_secs_f64() / processed as f64) * (total_tasks - processed) as f64
                        } else {
                            0.0
                        },
                        active_workers: *self.active_tasks.lock().await,
                    };
                    progress_callback(progress);
                    results.push(result);
                },
                Ok(Err(e)) => {
                    eprintln!("Error receiving result: {}", e);
                    remaining_tasks -= 1;
                }
                Err(_) => {
                    eprintln!("Timeout waiting for result {}/{}", processed + 1, total_tasks);
                    remaining_tasks -= 1;
                }
            }
        }

        println!("Batch processing complete. Processed: {}, Failed: {}, Total time: {:.2}s", 
            processed, failed_tasks.len(), start_time.elapsed().as_secs_f64());
        
        if !failed_tasks.is_empty() {
            println!("Failed tasks:");
            for (task, error) in &failed_tasks {
                println!("- {}: {}", task.input_path, error);
            }
        }

        Ok(results)
    }

    pub async fn get_metrics(&self) -> Vec<WorkerMetrics> {
        self.metrics.lock().await.clone()
    }
}

impl Clone for WorkerPool {
    fn clone(&self) -> Self {
        WorkerPool {
            workers: Vec::new(),
            task_sender: self.task_sender.clone(),
            result_receiver: self.result_receiver.clone(),
            active_tasks: self.active_tasks.clone(),
            metrics: self.metrics.clone(),
            sys: self.sys.clone(),
        }
    }
}

async fn process_image(app: &tauri::AppHandle, task: ImageTask) -> Result<OptimizationResult, String> {
    println!("Processing image: {}", task.input_path);
    let settings_json = match serde_json::to_string(&task.settings) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Failed to serialize settings: {}", e);
            return Err(e.to_string());
        }
    };

    println!("Invoking sharp-sidecar for: {}", task.input_path);
    let output = match app.shell()
        .sidecar("sharp-sidecar")
        .map_err(|e| e.to_string())?
        .args(&[
            "optimize",
            &task.input_path,
            &task.output_path,
            &settings_json,
        ])
        .output()
        .await
    {
        Ok(output) => output,
        Err(e) => {
            eprintln!("Failed to execute sharp-sidecar: {}", e);
            return Err(e.to_string());
        }
    };

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("Successfully processed: {}", task.input_path);
        match serde_json::from_str::<OptimizationResult>(&stdout) {
            Ok(result) => {
                println!("Optimization result for {}: {} bytes saved", 
                    task.input_path, result.saved_bytes);
                Ok(result)
            }
            Err(e) => {
                eprintln!("Failed to parse optimization result: {}", e);
                eprintln!("Raw output: {}", stdout);
                Err(e.to_string())
            }
        }
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        eprintln!("Sharp-sidecar failed for {}: {}", task.input_path, error);
        Err(error.to_string())
    }
} 