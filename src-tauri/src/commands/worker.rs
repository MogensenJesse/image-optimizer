use tauri::State;
use crate::core::AppState;
use crate::utils::OptimizerResult;

#[tauri::command]
pub async fn get_active_tasks(
    app: tauri::AppHandle,
    state: State<'_, AppState>
) -> OptimizerResult<Vec<String>> {
    let pool = state.get_or_init_process_pool(app).await?;
    Ok(pool.get_active_tasks().await)
} 