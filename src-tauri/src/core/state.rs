use std::sync::Arc;
use tokio::sync::Mutex;
use crate::worker::WorkerPool;

lazy_static::lazy_static! {
    pub(crate) static ref WORKER_POOL: Arc<Mutex<Option<WorkerPool>>> = Arc::new(Mutex::new(None));
}

#[derive(Clone)]
pub struct AppState {
    pub(crate) worker_pool: Arc<Mutex<Option<WorkerPool>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            worker_pool: Arc::clone(&WORKER_POOL),
        }
    }

    pub async fn get_or_init_worker_pool(&self, app: tauri::AppHandle) -> Arc<WorkerPool> {
        let mut pool = self.worker_pool.lock().await;
        if pool.is_none() {
            *pool = Some(WorkerPool::new(app));
        }
        Arc::new(pool.as_ref().unwrap().clone())
    }
} 