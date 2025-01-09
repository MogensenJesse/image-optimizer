use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;
use sysinfo::*;
use crate::worker_pool::ProcessingProgress;

#[derive(Debug, Clone)]
pub struct DebouncerConfig {
    pub min_interval: Duration,
    pub max_interval: Duration,
    pub channel_capacity: usize,
    pub max_merge_count: usize,
    pub adaptive_timing: bool,
    pub cpu_threshold: f32,
    pub slowdown_factor: f32,
}

impl Default for DebouncerConfig {
    fn default() -> Self {
        Self {
            min_interval: Duration::from_millis(100),
            max_interval: Duration::from_millis(1000),
            channel_capacity: 100,
            max_merge_count: 50,
            adaptive_timing: true,
            cpu_threshold: 80.0,
            slowdown_factor: 1.5,
        }
    }
}

pub struct ProgressDebouncer {
    config: DebouncerConfig,
    last_update: Arc<Mutex<Instant>>,
    pending_updates: Sender<ProcessingProgress>,
    update_receiver: Receiver<ProcessingProgress>,
    shutdown: Arc<AtomicBool>,
    sys: Arc<Mutex<System>>,
    worker_healthy: Arc<AtomicBool>,
    last_worker_health_check: Arc<Mutex<Instant>>,
}

impl ProgressDebouncer {
    pub fn new(config: Option<DebouncerConfig>) -> Self {
        let config = config.unwrap_or_default();
        let (tx, rx) = bounded(config.channel_capacity);
        
        Self {
            config,
            last_update: Arc::new(Mutex::new(Instant::now())),
            pending_updates: tx,
            update_receiver: rx,
            shutdown: Arc::new(AtomicBool::new(false)),
            sys: Arc::new(Mutex::new(System::new_all())),
            worker_healthy: Arc::new(AtomicBool::new(true)),
            last_worker_health_check: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Checks if the worker is healthy by monitoring update intervals and channel state.
    /// Returns false if the worker appears to be stuck (no updates for >30s with non-empty channel).
    /// This can be called externally to monitor worker health and trigger recovery if needed.
    pub fn check_worker_health(&self) -> bool {
        let now = Instant::now();
        let mut last_check = self.last_worker_health_check.lock();
        
        // Only check health every 5 seconds
        if now.duration_since(*last_check) < Duration::from_secs(5) {
            return self.worker_healthy.load(Ordering::Relaxed);
        }

        *last_check = now;
        let last_update = *self.last_update.lock();
        
        // If no updates for over 30 seconds and channel not empty, worker might be stuck
        if now.duration_since(last_update) > Duration::from_secs(30) && !self.pending_updates.is_empty() {
            tracing::error!("Worker appears to be stuck, marking as unhealthy");
            self.worker_healthy.store(false, Ordering::Relaxed);
            return false;
        }

        self.worker_healthy.store(true, Ordering::Relaxed);
        true
    }

    pub fn restart_worker<F>(&self, emit_fn: F) 
    where 
        F: Fn(ProcessingProgress) + Send + 'static 
    {
        tracing::warn!("Attempting to restart progress worker");
        self.shutdown.store(true, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(100));
        self.shutdown.store(false, Ordering::Relaxed);
        self.start(emit_fn);
        self.worker_healthy.store(true, Ordering::Relaxed);
        *self.last_worker_health_check.lock() = Instant::now();
    }

    pub fn queue_update(&self, progress: ProcessingProgress) -> Result<(), String> {
        let mut retry_count = 0;
        let max_retries = 3;

        while retry_count < max_retries {
            match self.pending_updates.try_send(progress.clone()) {
                Ok(_) => return Ok(()),
                Err(crossbeam_channel::TrySendError::Full(_)) => {
                    // Channel is full, wait briefly and retry
                    std::thread::sleep(Duration::from_millis(50));
                    retry_count += 1;
                    tracing::warn!("Progress channel full, retry {}/{}", retry_count, max_retries);
                }
                Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                    return Err("Progress channel disconnected".to_string());
                }
            }
        }
        
        Err("Failed to queue progress update after retries".to_string())
    }

    fn merge_updates(base: ProcessingProgress, update: ProcessingProgress) -> ProcessingProgress {
        // Keep the most recent file and timing information
        let elapsed_time = update.elapsed_time;
        let current_file = update.current_file;
        
        // Use the latest counts
        let processed_files = update.processed_files;
        let total_files = update.total_files;
        let bytes_processed = update.bytes_processed;
        let active_workers = update.active_workers;
        
        // Calculate cumulative bytes saved
        let bytes_saved = base.bytes_saved + update.bytes_saved;
        
        // Calculate throughput based on latest state
        let throughput_files_per_sec = if elapsed_time > 0.0 {
            processed_files as f64 / elapsed_time
        } else {
            0.0
        };
        
        let throughput_mb_per_sec = if elapsed_time > 0.0 {
            (bytes_processed as f64 / 1_048_576.0) / elapsed_time
        } else {
            0.0
        };
        
        // Calculate ETA based on current throughput
        let remaining_files = total_files.saturating_sub(processed_files);
        let estimated_time_remaining = if throughput_files_per_sec > 0.0 {
            remaining_files as f64 / throughput_files_per_sec
        } else {
            0.0
        };

        ProcessingProgress {
            total_files,
            processed_files,
            current_file,
            elapsed_time,
            bytes_processed,
            bytes_saved,
            estimated_time_remaining,
            active_workers,
            throughput_files_per_sec,
            throughput_mb_per_sec,
        }
    }

    pub fn start<F>(&self, emit_fn: F) 
    where 
        F: Fn(ProcessingProgress) + Send + 'static 
    {
        let receiver = self.update_receiver.clone();
        let last_update = self.last_update.clone();
        let shutdown = self.shutdown.clone();
        let config = self.config.clone();
        let sys = self.sys.clone();
        let worker_healthy = self.worker_healthy.clone();
        let last_worker_health_check = self.last_worker_health_check.clone();

        tokio::spawn(async move {
            while !shutdown.load(Ordering::Relaxed) {
                let interval = {
                    let mut sys = sys.lock();
                    sys.refresh_all();
                    
                    if !config.adaptive_timing {
                        config.min_interval
                    } else {
                        let cpu_usage = sys.cpus().iter()
                            .map(|cpu| cpu.cpu_usage())
                            .sum::<f32>() / sys.cpus().len() as f32;

                        if cpu_usage > config.cpu_threshold {
                            let load_factor = 1.0 + ((cpu_usage - config.cpu_threshold) / 100.0) * config.slowdown_factor;
                            let base_ms = config.min_interval.as_millis() as f32;
                            let adjusted_ms = base_ms * load_factor;
                            Duration::from_millis(adjusted_ms.min(config.max_interval.as_millis() as f32) as u64)
                        } else {
                            config.min_interval
                        }
                    }
                };
                
                match receiver.recv_timeout(interval) {
                    Ok(mut latest_update) => {
                        let mut merge_count = 1;
                        
                        while let Ok(update) = receiver.try_recv() {
                            latest_update = Self::merge_updates(latest_update, update);
                            merge_count += 1;
                            if merge_count >= config.max_merge_count {
                                break;
                            }
                        }

                        let now = Instant::now();
                        let mut last = last_update.lock();
                        if now.duration_since(*last) >= interval {
                            emit_fn(latest_update);
                            *last = now;
                            worker_healthy.store(true, Ordering::Relaxed);
                            *last_worker_health_check.lock() = now;
                        }
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                        let now = Instant::now();
                        let last_health_check = *last_worker_health_check.lock();
                        let last_update_time = *last_update.lock();
                        
                        if now.duration_since(last_health_check) >= Duration::from_secs(5) 
                            && now.duration_since(last_update_time) >= Duration::from_secs(30) {
                            worker_healthy.store(false, Ordering::Relaxed);
                            tracing::warn!("Worker health check failed, attempting recovery");
                            break;
                        }
                        continue;
                    },
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                        tracing::error!("Progress channel disconnected");
                        break;
                    }
                }
            }
        });
    }
}

impl Drop for ProgressDebouncer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
} 