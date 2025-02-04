use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tauri_plugin_shell::{ShellExt, process::Command};
use crate::utils::{OptimizerError, OptimizerResult};
use crate::benchmarking::metrics::{Duration, ProcessPoolMetrics};
use tracing::debug;
use num_cpus;

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
        // Use 75% of CPUs (max 24) to maintain ~1.33:1 worker-to-process ratio with 2x worker count
        ((cpu_count * 3) / 4).max(2).min(24)
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
        let start = std::time::Instant::now();
        
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

    pub async fn warmup(&self) -> OptimizerResult<()> {
        debug!("Starting process pool warmup with {} processes", self.max_size);
        let warmup_count = self.max_size;
        let mut handles = Vec::with_capacity(warmup_count);
        
        // Spawn warmup processes
        for i in 0..warmup_count {
            let handle = tokio::spawn({
                let pool = self.clone();
                async move {
                    debug!("Warming up process {}/{}", i + 1, warmup_count);
                    let cmd = pool.acquire().await?;
                    // Run a minimal operation to ensure process is ready
                    cmd.output()
                        .await
                        .map_err(|e| OptimizerError::sidecar(format!("Process warmup command failed: {}", e)))?;
                    pool.release().await;
                    Ok::<_, OptimizerError>(())
                }
            });
            handles.push(handle);
        }
        
        // Wait for all warmup processes
        futures::future::try_join_all(handles)
            .await
            .map_err(|e| OptimizerError::sidecar(format!("Process warmup failed: {}", e)))?
            .into_iter()
            .collect::<OptimizerResult<Vec<_>>>()?;

        debug!("Process pool warmup completed successfully");
        Ok(())
    }
} 