use tauri::State;
use crate::core::AppState;
use crate::utils::OptimizerResult;

#[tauri::command]
pub async fn get_active_tasks(
    state: State<'_, AppState>
) -> OptimizerResult<usize> {
    let pool = state.worker_pool.lock().await;
    if let Some(pool) = pool.as_ref() {
        Ok(pool.get_active_workers().await)
    } else {
        Ok(0)
    }
} 