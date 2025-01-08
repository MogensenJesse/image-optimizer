use crate::commands::image::{ImageSettings, OptimizationResult};
use crossbeam_channel::{bounded, Receiver, Sender};
use tauri_plugin_shell::ShellExt;
use tauri::Emitter;
use std::sync::{Arc, atomic::{AtomicUsize, AtomicU64, Ordering}};
use tokio::sync::Mutex;
use std::time::{Instant, SystemTime};
use sysinfo::*;
use std::collections::VecDeque;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageTask {
    pub input_path: String,
    pub output_path: String,
    pub settings: ImageSettings,
    pub priority: u8,
}

#[derive(Debug, Clone)]
pub struct ProgressSnapshot {
    pub timestamp: SystemTime,
    pub processed_files: usize,
    pub total_files: usize,
    pub bytes_processed: u64,
    pub active_workers: usize,
    pub last_event_time: SystemTime,
}

pub struct ProgressState {
    processed_files: AtomicUsize,
    bytes_processed: AtomicU64,
    start_time: Instant,
    last_active: Arc<Mutex<Instant>>,
    total_files: AtomicUsize,
}

pub struct WorkerPool {
    workers: Vec<Worker>,
    task_sender: Sender<ImageTask>,
    result_receiver: Receiver<OptimizationResult>,
    active_tasks: Arc<Mutex<usize>>,
    metrics: Arc<Mutex<Vec<WorkerMetrics>>>,
    sys: Arc<Mutex<System>>,
    progress_state: Arc<Mutex<ProgressState>>,
    last_progress_update: Arc<Mutex<Instant>>,
    progress_history: Arc<Mutex<VecDeque<ProgressSnapshot>>>,
    app: Option<tauri::AppHandle>,
}

struct Worker {
    #[allow(dead_code)]
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
    throughput_files_per_sec: f64,
    throughput_mb_per_sec: f64,
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
        tracing::info!("Initializing WorkerPool with {} workers", size);
        let mut sys = System::new_all();
        sys.refresh_all();
        
        // Get memory in GB and calculate buffer size
        let total_memory_gb = sys.total_memory() / (1024 * 1024);
        println!("Detected system memory: {}GB", total_memory_gb);
        
        // More aggressive buffer sizing based on memory
        let buffer_size = if total_memory_gb == 0 || total_memory_gb > 1024 {
            println!("Warning: Memory detection unreliable, using conservative settings");
            size * 3
        } else {
            // Calculate buffer size based on memory and CPU cores
            let base_size = if total_memory_gb < 8 {
                size * 3
            } else if total_memory_gb < 16 {
                size * 4
            } else {
                size * 6
            };

            // Adjust based on CPU frequency and core count
            let cpus = sys.cpus();
            let avg_freq = cpus.iter()
                .map(|cpu| cpu.frequency())
                .sum::<u64>() / cpus.len() as u64;
            
            if avg_freq > 3000 { // > 3GHz
                base_size + (size * 2)
            } else {
                base_size + size
            }
        };
        println!("Using buffer size: {} (based on memory and CPU)", buffer_size);

        let (task_sender, task_receiver) = bounded::<ImageTask>(buffer_size);
        let (result_sender, result_receiver) = bounded::<OptimizationResult>(buffer_size);
        let active_tasks = Arc::new(Mutex::new(0));
        let metrics = Arc::new(Mutex::new(Vec::with_capacity(size)));
        let sys = Arc::new(Mutex::new(sys));

        let mut workers = Vec::with_capacity(size);
        
        let progress_state = Arc::new(Mutex::new(ProgressState {
            processed_files: AtomicUsize::new(0),
            bytes_processed: AtomicU64::new(0),
            start_time: Instant::now(),
            last_active: Arc::new(Mutex::new(Instant::now())),
            total_files: AtomicUsize::new(0),
        }));
        
        let last_progress_update = Arc::new(Mutex::new(Instant::now()));
        let progress_history = Arc::new(Mutex::new(VecDeque::with_capacity(100)));

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
                            
                            // Update progress before sending result
                            let mut active = active_tasks.lock().await;
                            *active -= 1;
                            
                            // Update metrics first
                            {
                                let mut metrics = metrics.lock().await;
                                if let Some(metric) = metrics.get_mut(id) {
                                    metric.task_count = task_count;
                                    metric.avg_processing_time = total_time / task_count as f64;
                                    println!("Worker {} metrics - Tasks: {}, Avg Time: {:.2}s (Last: {:.2}s)", 
                                        id, task_count, metric.avg_processing_time, processing_time);
                                }
                            }
                            
                            // Send result only after progress is updated
                            let _ = result_tx.send(result);
                        }
                        Ok(Err(e)) => {
                            eprintln!("Worker {} error processing image: {}", id, e);
                            let mut active = active_tasks.lock().await;
                            *active -= 1;
                        }
                        Err(_) => {
                            eprintln!("Worker {} image processing timed out", id);
                            let mut active = active_tasks.lock().await;
                            *active -= 1;
                        }
                    }

                    println!("Worker {} finished processing. Active tasks: {}", id, *active_tasks.lock().await);

                    // Adaptive cooldown based on system state
                    let cooldown = {
                        let mut sys = sys.lock().await;
                        sys.refresh_cpu_usage();
                        let cpu_usage = sys.cpus()[id].cpu_usage() as f64;
                        
                        // Get current active tasks
                        let active = *active_tasks.lock().await;
                        
                        // Calculate base cooldown from processing time
                        let base_cooldown: u64 = if processing_time > 2.5 {
                            20
                        } else if processing_time > 2.0 {
                            15
                        } else if processing_time > 1.5 {
                            10
                        } else {
                            5
                        };
                        
                        // Adjust based on CPU and active tasks
                        if cpu_usage > 90.0 {
                            base_cooldown + 10 // Heavy CPU load
                        } else if cpu_usage > 70.0 && active >= 3 {
                            base_cooldown + 5 // Moderate load with many tasks
                        } else if cpu_usage < 30.0 && active <= 2 {
                            base_cooldown.saturating_sub(3) // Light load, speed up
                        } else {
                            base_cooldown
                        }
                    };
                    
                    if cooldown > 0 {
                        tokio::time::sleep(std::time::Duration::from_millis(cooldown)).await;
                    }
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
            progress_state,
            last_progress_update,
            progress_history,
            app: Some(app),
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
        let span = tracing::info_span!("batch_processing", total_tasks = tasks.len());
        let _enter = span.enter();

        tracing::info!("Starting batch processing of {} tasks", tasks.len());
        let start_time = Instant::now();
        let total_tasks = tasks.len();
        
        // Initialize progress state
        self.set_total_files(total_tasks).await?;
        
        let mut results = Vec::with_capacity(total_tasks);
        let mut failed_tasks = Vec::new();
        
        // Queue tasks with backpressure
        tracing::debug!("Queueing tasks with backpressure...");
        for (task_idx, task) in tasks.iter().enumerate() {
            let file_name = task.input_path.split(['/', '\\']).last()
                .unwrap_or(&task.input_path).to_string();
            
            tracing::debug!(
                task_index = task_idx + 1,
                total = total_tasks,
                file = %file_name,
                "Queueing task"
            );

            // Wait for space in the channel if it's full
            while self.task_sender.is_full() {
                tracing::debug!("Channel full, waiting for space...");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                
                // Try to collect any available results while waiting
                match self.result_receiver.try_recv() {
                    Ok(result) => {
                        self.update_progress(result.path.clone(), result.original_size).await?;
                        tracing::debug!(
                            saved_bytes = result.saved_bytes,
                            "Collected result while waiting"
                        );
                        results.push(result);

                        // Validate progress after each result
                        let processed_files = self.progress_state.lock().await.processed_files.load(Ordering::SeqCst);
                        if processed_files != results.len() {
                            tracing::warn!(
                                "Progress count mismatch during collection: atomic={}, results={}",
                                processed_files,
                                results.len()
                            );
                        }
                    }
                    Err(crossbeam_channel::TryRecvError::Empty) => (),
                    Err(e) => tracing::error!("Error receiving result while waiting: {}", e),
                }
            }

            match self.task_sender.send(task.clone()) {
                Ok(_) => tracing::debug!("Successfully queued: {}", file_name),
                Err(e) => {
                    tracing::error!("Failed to queue task: {}", e);
                    failed_tasks.push((task.clone(), format!("Queue error: {}", e)));
                    continue;
                }
            }

            // Get current progress state for callback
            let state = self.progress_state.lock().await;
            let processed = state.processed_files.load(Ordering::SeqCst);
            let bytes_processed = state.bytes_processed.load(Ordering::SeqCst);
            
            let progress = ProcessingProgress {
                total_files: total_tasks,
                processed_files: processed,
                current_file: file_name.clone(),
                elapsed_time: start_time.elapsed().as_secs_f64(),
                bytes_processed,
                bytes_saved: 0, // This will be updated when we get the result
                estimated_time_remaining: if processed > 0 {
                    (start_time.elapsed().as_secs_f64() / processed as f64) * (total_tasks - processed) as f64
                } else {
                    0.0
                },
                active_workers: *self.active_tasks.lock().await,
                throughput_files_per_sec: processed as f64 / start_time.elapsed().as_secs_f64(),
                throughput_mb_per_sec: (bytes_processed as f64 / 1_048_576.0) / start_time.elapsed().as_secs_f64(),
            };
            progress_callback(progress);
        }

        // Collect remaining results
        tracing::info!("All tasks queued, collecting remaining results...");
        let state = self.progress_state.lock().await;
        let processed = state.processed_files.load(Ordering::SeqCst);
        let mut remaining_tasks = total_tasks - processed - failed_tasks.len();
        drop(state);
        
        tracing::debug!("Remaining tasks to collect: {}", remaining_tasks);
        
        while remaining_tasks > 0 {
            tracing::debug!(
                processed_plus_one = processed + 1,
                total = total_tasks,
                remaining = remaining_tasks,
                "Waiting for result"
            );

            match tokio::time::timeout(
                std::time::Duration::from_secs(60),
                async { self.result_receiver.recv().map_err(|e| e.to_string()) }
            ).await {
                Ok(Ok(result)) => {
                    self.update_progress(result.path.clone(), result.original_size).await?;
                    remaining_tasks -= 1;
                    
                    let state = self.progress_state.lock().await;
                    let processed = state.processed_files.load(Ordering::SeqCst);
                    let bytes_processed = state.bytes_processed.load(Ordering::SeqCst);
                    
                    let progress = ProcessingProgress {
                        total_files: total_tasks,
                        processed_files: processed,
                        current_file: result.path.clone(),
                        elapsed_time: start_time.elapsed().as_secs_f64(),
                        bytes_processed,
                        bytes_saved: result.saved_bytes,
                        estimated_time_remaining: if processed > 0 {
                            (start_time.elapsed().as_secs_f64() / processed as f64) * (total_tasks - processed) as f64
                        } else {
                            0.0
                        },
                        active_workers: *self.active_tasks.lock().await,
                        throughput_files_per_sec: processed as f64 / start_time.elapsed().as_secs_f64(),
                        throughput_mb_per_sec: (bytes_processed as f64 / 1_048_576.0) / start_time.elapsed().as_secs_f64(),
                    };
                    progress_callback(progress);
                    results.push(result);

                    // Validate progress after each result
                    if processed != results.len() {
                        tracing::warn!(
                            "Progress count mismatch: atomic={}, results={}",
                            processed,
                            results.len()
                        );
                    }
                },
                Ok(Err(e)) => {
                    tracing::error!("Error receiving result: {}", e);
                    remaining_tasks -= 1;
                }
                Err(_) => {
                    tracing::error!(
                        processed_plus_one = processed + 1,
                        total = total_tasks,
                        "Timeout waiting for result"
                    );
                    remaining_tasks -= 1;
                }
            }
        }

        // Final validation
        let final_state = self.progress_state.lock().await;
        let total = final_state.total_files.load(Ordering::SeqCst);
        let processed = final_state.processed_files.load(Ordering::SeqCst);

        tracing::info!(
            "Batch complete - Processed: {}/{}, Results: {}, Failed: {}, Total time: {:.2}s",
            processed,
            total,
            results.len(),
            failed_tasks.len(),
            start_time.elapsed().as_secs_f64()
        );

        // Verify completion
        if processed != total {
            tracing::error!(
                "Completion mismatch: processed={}, total={}, results={}",
                processed,
                total,
                results.len()
            );
            // Correct the count if needed
            final_state.processed_files.store(results.len(), Ordering::SeqCst);
        }
        
        if !failed_tasks.is_empty() {
            tracing::warn!("Failed tasks:");
            for (task, error) in &failed_tasks {
                tracing::warn!(task = %task.input_path, error = %error);
            }
        }

        Ok(results)
    }

    pub async fn get_metrics(&self) -> Vec<WorkerMetrics> {
        self.metrics.lock().await.clone()
    }

    pub async fn update_progress(&self, file: String, bytes: u64) -> Result<(), String> {
        let span = tracing::debug_span!("update_progress", file = %file, bytes = %bytes);
        let _enter = span.enter();

        // Take a snapshot of the current state atomically
        let state = self.progress_state.lock().await;
        let current_processed = state.processed_files.fetch_add(1, Ordering::SeqCst);
        let current_bytes = state.bytes_processed.fetch_add(bytes, Ordering::SeqCst);
        let total_files = state.total_files.load(Ordering::SeqCst);
        *state.last_active.lock().await = Instant::now();
        
        // Create snapshot after atomic updates
        let snapshot = ProgressSnapshot {
            timestamp: SystemTime::now(),
            processed_files: current_processed + 1,
            total_files,
            bytes_processed: current_bytes + bytes,
            active_workers: *self.active_tasks.lock().await,
            last_event_time: SystemTime::now(),
        };

        // Validate progress
        if snapshot.processed_files > total_files {
            tracing::error!(
                "Progress count exceeded total: processed={}, total={}",
                snapshot.processed_files,
                total_files
            );
            // Correct the count
            state.processed_files.store(total_files, Ordering::SeqCst);
        }

        tracing::info!(
            "Progress snapshot: {}/{} files ({:.1}%), {} bytes, {} workers",
            snapshot.processed_files,
            snapshot.total_files,
            (snapshot.processed_files as f64 / snapshot.total_files as f64) * 100.0,
            snapshot.bytes_processed,
            snapshot.active_workers
        );

        // Store snapshot with proper synchronization
        let mut history = self.progress_history.lock().await;
        if history.len() >= 100 {
            history.pop_front();
        }
        history.push_back(snapshot);

        Ok(())
    }

    pub async fn set_total_files(&self, total: usize) -> Result<(), String> {
        let state = self.progress_state.lock().await;
        state.total_files.store(total, Ordering::SeqCst);
        Ok(())
    }

    pub async fn resume_processing(&self) -> Result<(), String> {
        tracing::info!("Attempting to resume processing");
        
        // Check for stuck workers
        let active_tasks = *self.active_tasks.lock().await;
        if active_tasks > 0 {
            tracing::warn!("Found {} stuck workers", active_tasks);
            
            // Reset worker states
            *self.active_tasks.lock().await = 0;
            
            // Clear any stuck tasks in the channel
            while let Ok(_) = self.result_receiver.try_recv() {
                tracing::debug!("Cleared stuck task from result channel");
            }
            
            // Update progress state
            if let Ok(mut progress_state) = self.progress_state.try_lock() {
                progress_state.last_active = Arc::new(Mutex::new(Instant::now()));
            }
            
            // Emit recovery event
            if let Some(app) = self.app.as_ref() {
                let _ = app.emit("processing_resumed", ());
            }
        }
        
        Ok(())
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
            progress_state: self.progress_state.clone(),
            last_progress_update: self.last_progress_update.clone(),
            progress_history: self.progress_history.clone(),
            app: self.app.clone(),
        }
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        for worker in self.workers.drain(..) {
            worker.handle.abort();
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

#[tauri::command]
pub async fn resume_processing(state: tauri::State<'_, Arc<Mutex<Option<WorkerPool>>>>) -> Result<(), String> {
    if let Some(pool) = state.lock().await.as_ref() {
        pool.resume_processing().await?;
        Ok(())
    } else {
        Err("Worker pool not initialized".to_string())
    }
} 