use crate::application::services::SyncServiceTrait;
use crate::presentation::dto::ApiResponse;
use crate::shared::AppError;
use crate::state::AppState;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub async fn start_sync(state: State<'_, AppState>) -> Result<ApiResponse<()>, AppError> {
    let result = state.sync_service.start_sync().await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn stop_sync(state: State<'_, AppState>) -> Result<ApiResponse<()>, AppError> {
    let result = state.sync_service.stop_sync().await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn get_sync_status(state: State<'_, AppState>) -> Result<ApiResponse<Value>, AppError> {
    let status = state.sync_service.get_status().await;
    let value = serde_json::to_value(status).map_err(AppError::from)?;
    Ok(ApiResponse::success(value))
}

#[tauri::command]
pub async fn reset_sync(state: State<'_, AppState>) -> Result<ApiResponse<()>, AppError> {
    let result = state.sync_service.reset_sync().await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn schedule_sync(
    interval_secs: u64,
    state: State<'_, AppState>,
) -> Result<ApiResponse<()>, AppError> {
    state.sync_service.schedule_sync(interval_secs).await;
    Ok(ApiResponse::success(()))
}
