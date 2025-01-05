use crate::commands::image::{ImageSettings, OptimizationResult};
use crossbeam_channel::{bounded, Receiver, Sender};
use tauri_plugin_shell::ShellExt;
use std::sync::Arc;
use tokio::sync::Mutex;

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
}

struct Worker {
    id: usize,
    handle: tokio::task::JoinHandle<()>,
}

impl WorkerPool {
    pub fn new(size: usize, app: tauri::AppHandle) -> Self {
        let (task_sender, task_receiver) = bounded::<ImageTask>(size * 2);
        let (result_sender, result_receiver) = bounded::<OptimizationResult>(size * 2);
        let active_tasks = Arc::new(Mutex::new(0));

        let mut workers = Vec::with_capacity(size);
        
        for id in 0..size {
            let task_rx = task_receiver.clone();
            let result_tx = result_sender.clone();
            let app = app.clone();
            let active_tasks = Arc::clone(&active_tasks);

            let handle = tokio::spawn(async move {
                while let Ok(task) = task_rx.recv() {
                    let mut active = active_tasks.lock().await;
                    *active += 1;
                    drop(active);

                    let result = process_image(&app, task).await;
                    if let Ok(result) = result {
                        let _ = result_tx.send(result);
                    }

                    let mut active = active_tasks.lock().await;
                    *active -= 1;
                }
            });

            workers.push(Worker { id, handle });
        }

        WorkerPool {
            workers,
            task_sender,
            result_receiver,
            active_tasks,
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
        tasks: Vec<ImageTask>
    ) -> Result<Vec<OptimizationResult>, String> {
        let total_tasks = tasks.len();
        let mut results = Vec::with_capacity(total_tasks);
        
        // Send all tasks to the queue
        for task in tasks {
            self.task_sender.send(task)
                .map_err(|e| format!("Failed to queue task: {}", e))?;
        }

        // Collect results
        for _ in 0..total_tasks {
            match self.result_receiver.recv() {
                Ok(result) => results.push(result),
                Err(e) => return Err(format!("Failed to receive result: {}", e)),
            }
        }

        Ok(results)
    }
}

impl Clone for WorkerPool {
    fn clone(&self) -> Self {
        WorkerPool {
            workers: Vec::new(),
            task_sender: self.task_sender.clone(),
            result_receiver: self.result_receiver.clone(),
            active_tasks: self.active_tasks.clone(),
        }
    }
}

async fn process_image(app: &tauri::AppHandle, task: ImageTask) -> Result<OptimizationResult, String> {
    let settings_json = serde_json::to_string(&task.settings)
        .map_err(|e| e.to_string())?;

    let output = app.shell()
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
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
            .map_err(|e| e.to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
} 