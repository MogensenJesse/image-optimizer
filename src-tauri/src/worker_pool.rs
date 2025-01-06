use crate::commands::image::{ImageSettings, OptimizationResult};
use crossbeam_channel::{bounded, Receiver, Sender};
use tauri_plugin_shell::ShellExt;
use std::{sync::Arc, time::Instant};
use tokio::sync::Mutex;
use sysinfo::System;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageTask {
    pub input_path: String,
    pub output_path: String,
    pub settings: ImageSettings,
    pub priority: u8,
}

pub struct WorkerPool {
    #[allow(dead_code)]
    workers: Vec<Worker>,
    task_sender: Sender<ImageTask>,
    result_receiver: Receiver<OptimizationResult>,
    active_tasks: Arc<Mutex<usize>>,
    metrics: Arc<Mutex<Vec<WorkerMetrics>>>,
    sys: Arc<Mutex<System>>,
}

#[allow(dead_code)]
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
        let mut sys = System::new_all();
        sys.refresh_all();
        
        let total_memory_gb = sys.total_memory() / (1024 * 1024);
        
        let buffer_size = if total_memory_gb == 0 || total_memory_gb > 1024 {
            size * 2
        } else {
            let base_size = if total_memory_gb < 8 {
                size * 2
            } else if total_memory_gb < 16 {
                size * 3
            } else {
                size * 4
            };

            let cpus = sys.cpus();
            let avg_freq = cpus.iter()
                .map(|cpu| cpu.frequency())
                .sum::<u64>() / cpus.len() as u64;
            
            if avg_freq > 3000 {
                base_size + size
            } else {
                base_size
            }
        };

        let (task_sender, task_receiver) = bounded::<ImageTask>(buffer_size);
        let (result_sender, result_receiver) = bounded::<OptimizationResult>(buffer_size);
        let active_tasks = Arc::new(Mutex::new(0));
        let metrics = Arc::new(Mutex::new(Vec::with_capacity(size)));
        let sys = Arc::new(Mutex::new(sys));

        let mut workers = Vec::with_capacity(size);
        
        for id in 0..size {
            let task_rx = task_receiver.clone();
            let result_tx = result_sender.clone();
            let app = app.clone();
            let active_tasks = Arc::clone(&active_tasks);
            let metrics = Arc::clone(&metrics);
            let sys = Arc::clone(&sys);

            let handle = tokio::spawn(async move {
                let mut task_count = 0;
                let mut total_time = 0.0;
                let mut consecutive_long_tasks = 0;

                while let Ok(task) = task_rx.recv() {
                    let start_time = std::time::Instant::now();
                    
                    {
                        let mut sys = sys.lock().await;
                        sys.refresh_cpu_usage();
                        let cpu_usage = sys.cpus()[id].cpu_usage() as f64;
                        
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

                        if cpu_usage > 90.0 && consecutive_long_tasks > 2 {
                            drop(sys);
                            drop(metrics);
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                            consecutive_long_tasks = 0;
                        }
                    }

                    let mut active = active_tasks.lock().await;
                    *active += 1;
                    drop(active);

                    let result = tokio::time::timeout(
                        std::time::Duration::from_secs(30),
                        process_image(&app, task)
                    ).await;

                    let processing_time = start_time.elapsed().as_secs_f64();
                    
                    if processing_time > 2.5 {
                        consecutive_long_tasks += 1;
                    } else {
                        consecutive_long_tasks = 0;
                    }

                    task_count += 1;
                    total_time += processing_time;

                    match result {
                        Ok(Ok(result)) => {
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

                    {
                        let mut metrics = metrics.lock().await;
                        if let Some(metric) = metrics.get_mut(id) {
                            metric.task_count = task_count;
                            metric.avg_processing_time = total_time / task_count as f64;
                        }
                    }

                    let cooldown = if processing_time > 2.5 {
                        20
                    } else if processing_time > 2.0 {
                        15
                    } else {
                        10
                    };
                    tokio::time::sleep(std::time::Duration::from_millis(cooldown)).await;
                }
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
        let start_time = Instant::now();
        let total_tasks = tasks.len();
        let mut results = Vec::with_capacity(total_tasks);
        let mut processed = 0;
        let mut bytes_processed = 0;
        let mut bytes_saved = 0;
        let mut failed_tasks = Vec::new();
        
        for task in tasks {
            while self.task_sender.is_full() {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                
                match self.result_receiver.try_recv() {
                    Ok(result) => {
                        processed += 1;
                        bytes_processed += result.original_size;
                        bytes_saved += result.saved_bytes;
                        results.push(result);
                    }
                    Err(crossbeam_channel::TryRecvError::Empty) => (),
                    Err(e) => eprintln!("Error receiving result while waiting: {}", e),
                }
            }

            if let Err(e) = self.task_sender.send(task.clone()) {
                eprintln!("Failed to queue task: {}", e);
                failed_tasks.push((task, format!("Queue error: {}", e)));
                continue;
            }

            let progress = ProcessingProgress {
                total_files: total_tasks,
                processed_files: processed,
                current_file: task.input_path,
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

        let mut remaining_tasks = total_tasks - processed - failed_tasks.len();
        
        while remaining_tasks > 0 {
            match tokio::time::timeout(
                std::time::Duration::from_secs(60),
                async { self.result_receiver.recv().map_err(|e| e.to_string()) }
            ).await {
                Ok(Ok(result)) => {
                    processed += 1;
                    remaining_tasks -= 1;
                    bytes_processed += result.original_size;
                    bytes_saved += result.saved_bytes;
                    
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
                    eprintln!("Timeout waiting for result");
                    remaining_tasks -= 1;
                }
            }
        }

        if !failed_tasks.is_empty() {
            eprintln!("Failed tasks: {}", failed_tasks.len());
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
    let settings_json = match serde_json::to_string(&task.settings) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Failed to serialize settings: {}", e);
            return Err(e.to_string());
        }
    };

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
        match serde_json::from_str::<OptimizationResult>(&stdout) {
            Ok(result) => Ok(result),
            Err(e) => {
                eprintln!("Failed to parse optimization result: {}", e);
                eprintln!("Raw output: {}", stdout);
                Err(e.to_string())
            }
        }
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        eprintln!("Sharp-sidecar failed: {}", error);
        Err(error.to_string())
    }
} 