use crate::presentation::dto::offline::{
    SaveOfflineActionRequest, SaveOfflineActionResponse,
    GetOfflineActionsRequest, SyncOfflineActionsRequest, SyncOfflineActionsResponse,
    CacheStatusResponse, AddToSyncQueueRequest, UpdateCacheMetadataRequest,
    OptimisticUpdateRequest, OfflineAction, UpdateSyncStatusRequest
};
use crate::state::AppState;
use tauri::State;

/// オフラインアクションを保存
#[tauri::command]
pub async fn save_offline_action(
    state: State<'_, AppState>,
    request: SaveOfflineActionRequest,
) -> Result<SaveOfflineActionResponse, String> {
    state
        .offline_handler
        .save_offline_action(request)
        .await
        .map_err(|e| e.to_string())
}

/// オフラインアクションを取得
#[tauri::command]
pub async fn get_offline_actions(
    state: State<'_, AppState>,
    request: GetOfflineActionsRequest,
) -> Result<Vec<OfflineAction>, String> {
    state
        .offline_handler
        .get_offline_actions(request)
        .await
        .map_err(|e| e.to_string())
}

/// オフラインアクションを同期
#[tauri::command]
pub async fn sync_offline_actions(
    state: State<'_, AppState>,
    request: SyncOfflineActionsRequest,
) -> Result<SyncOfflineActionsResponse, String> {
    state
        .offline_handler
        .sync_offline_actions(request)
        .await
        .map_err(|e| e.to_string())
}

/// キャッシュステータスを取得
#[tauri::command]
pub async fn get_cache_status(
    state: State<'_, AppState>,
) -> Result<CacheStatusResponse, String> {
    state
        .offline_handler
        .get_cache_status()
        .await
        .map_err(|e| e.to_string())
}

/// 同期キューに追加
#[tauri::command]
pub async fn add_to_sync_queue(
    state: State<'_, AppState>,
    request: AddToSyncQueueRequest,
) -> Result<i64, String> {
    state
        .offline_handler
        .add_to_sync_queue(request)
        .await
        .map_err(|e| e.to_string())
}

/// キャッシュメタデータを更新
#[tauri::command]
pub async fn update_cache_metadata(
    state: State<'_, AppState>,
    request: UpdateCacheMetadataRequest,
) -> Result<String, String> {
    state
        .offline_handler
        .update_cache_metadata(request)
        .await
        .map(|response| serde_json::to_string(&response).unwrap())
        .map_err(|e| e.to_string())
}

/// 楽観的更新を保存
#[tauri::command]
pub async fn save_optimistic_update(
    state: State<'_, AppState>,
    entity_type: String,
    entity_id: String,
    original_data: Option<String>,
    updated_data: String,
) -> Result<String, String> {
    let request = OptimisticUpdateRequest {
        entity_type,
        entity_id,
        original_data,
        updated_data,
    };
    
    state
        .offline_handler
        .save_optimistic_update(request)
        .await
        .map_err(|e| e.to_string())
}

/// 楽観的更新を確定
#[tauri::command]
pub async fn confirm_optimistic_update(
    state: State<'_, AppState>,
    update_id: String,
) -> Result<String, String> {
    state
        .offline_handler
        .confirm_optimistic_update(update_id)
        .await
        .map(|response| serde_json::to_string(&response).unwrap())
        .map_err(|e| e.to_string())
}

/// 楽観的更新をロールバック
#[tauri::command]
pub async fn rollback_optimistic_update(
    state: State<'_, AppState>,
    update_id: String,
) -> Result<Option<String>, String> {
    state
        .offline_handler
        .rollback_optimistic_update(update_id)
        .await
        .map_err(|e| e.to_string())
}

/// 期限切れキャッシュをクリーンアップ
#[tauri::command]
pub async fn cleanup_expired_cache(
    state: State<'_, AppState>,
) -> Result<i32, String> {
    state
        .offline_handler
        .cleanup_expired_cache()
        .await
        .map_err(|e| e.to_string())
}

/// 同期ステータスを更新
#[tauri::command]
pub async fn update_sync_status(
    state: State<'_, AppState>,
    entity_type: String,
    entity_id: String,
    sync_status: String,
    conflict_data: Option<String>,
) -> Result<String, String> {
    let request = UpdateSyncStatusRequest {
        entity_type,
        entity_id,
        sync_status,
        conflict_data,
    };
    
    state
        .offline_handler
        .update_sync_status(request)
        .await
        .map(|response| serde_json::to_string(&response).unwrap())
        .map_err(|e| e.to_string())
}