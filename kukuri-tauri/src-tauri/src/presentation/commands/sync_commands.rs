use crate::application::services::SyncService;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn start_sync(sync_service: State<'_, Arc<SyncService>>) -> Result<(), String> {
    sync_service.start_sync().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_sync(sync_service: State<'_, Arc<SyncService>>) -> Result<(), String> {
    sync_service.stop_sync().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_sync_status(
    sync_service: State<'_, Arc<SyncService>>,
) -> Result<serde_json::Value, String> {
    let status = sync_service.get_status().await;
    serde_json::to_value(status).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reset_sync(sync_service: State<'_, Arc<SyncService>>) -> Result<(), String> {
    sync_service.reset_sync().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn schedule_sync(
    interval_secs: u64,
    sync_service: State<'_, Arc<SyncService>>,
) -> Result<(), String> {
    sync_service.schedule_sync(interval_secs).await;
    Ok(())
}
