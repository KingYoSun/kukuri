use crate::application::services::SyncService;
use crate::presentation::dto::ApiResponse;
use crate::shared::AppError;
use serde_json::Value;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn start_sync(
    sync_service: State<'_, Arc<SyncService>>,
) -> Result<ApiResponse<()>, AppError> {
    let result = sync_service.start_sync().await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn stop_sync(
    sync_service: State<'_, Arc<SyncService>>,
) -> Result<ApiResponse<()>, AppError> {
    let result = sync_service.stop_sync().await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn get_sync_status(
    sync_service: State<'_, Arc<SyncService>>,
) -> Result<ApiResponse<Value>, AppError> {
    let status = sync_service.get_status().await;
    let value = serde_json::to_value(status).map_err(AppError::from)?;
    Ok(ApiResponse::success(value))
}

#[tauri::command]
pub async fn reset_sync(
    sync_service: State<'_, Arc<SyncService>>,
) -> Result<ApiResponse<()>, AppError> {
    let result = sync_service.reset_sync().await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn schedule_sync(
    interval_secs: u64,
    sync_service: State<'_, Arc<SyncService>>,
) -> Result<ApiResponse<()>, AppError> {
    sync_service.schedule_sync(interval_secs).await;
    Ok(ApiResponse::success(()))
}
