use crate::presentation::dto::ApiResponse;
use crate::presentation::dto::offline::{
    AddToSyncQueueRequest, CacheStatusResponse, GetOfflineActionsRequest, OfflineAction,
    OptimisticUpdateRequest, SaveOfflineActionRequest, SaveOfflineActionResponse,
    SyncOfflineActionsRequest, SyncOfflineActionsResponse, UpdateCacheMetadataRequest,
    UpdateSyncStatusRequest,
};
use crate::shared::AppError;
use crate::state::AppState;
use serde_json::Value;
use tauri::State;

/// オフラインアクションを保存
#[tauri::command]
pub async fn save_offline_action(
    state: State<'_, AppState>,
    request: SaveOfflineActionRequest,
) -> Result<ApiResponse<SaveOfflineActionResponse>, AppError> {
    let result = state.offline_handler.save_offline_action(request).await;
    Ok(ApiResponse::from_result(result))
}

/// オフラインアクションを取得
#[tauri::command]
pub async fn get_offline_actions(
    state: State<'_, AppState>,
    request: GetOfflineActionsRequest,
) -> Result<ApiResponse<Vec<OfflineAction>>, AppError> {
    let result = state.offline_handler.get_offline_actions(request).await;
    Ok(ApiResponse::from_result(result))
}

/// オフラインアクションを同期
#[tauri::command]
pub async fn sync_offline_actions(
    state: State<'_, AppState>,
    request: SyncOfflineActionsRequest,
) -> Result<ApiResponse<SyncOfflineActionsResponse>, AppError> {
    let result = state.offline_handler.sync_offline_actions(request).await;
    Ok(ApiResponse::from_result(result))
}

/// キャッシュステータスを取得
#[tauri::command]
pub async fn get_cache_status(
    state: State<'_, AppState>,
) -> Result<ApiResponse<CacheStatusResponse>, AppError> {
    let result = state.offline_handler.get_cache_status().await;
    Ok(ApiResponse::from_result(result))
}

/// 同期キューに追加
#[tauri::command]
pub async fn add_to_sync_queue(
    state: State<'_, AppState>,
    request: AddToSyncQueueRequest,
) -> Result<ApiResponse<i64>, AppError> {
    let result = state.offline_handler.add_to_sync_queue(request).await;
    Ok(ApiResponse::from_result(result))
}

/// キャッシュメタデータを更新
#[tauri::command]
pub async fn update_cache_metadata(
    state: State<'_, AppState>,
    request: UpdateCacheMetadataRequest,
) -> Result<ApiResponse<Value>, AppError> {
    let result = state.offline_handler.update_cache_metadata(request).await;
    Ok(ApiResponse::from_result(result))
}

/// 楽観的更新を保存
#[tauri::command]
pub async fn save_optimistic_update(
    state: State<'_, AppState>,
    entity_type: String,
    entity_id: String,
    original_data: Option<String>,
    updated_data: String,
) -> Result<ApiResponse<String>, AppError> {
    let request = OptimisticUpdateRequest {
        entity_type,
        entity_id,
        original_data,
        updated_data,
    };

    let result = state.offline_handler.save_optimistic_update(request).await;
    Ok(ApiResponse::from_result(result))
}

/// 楽観的更新を確定
#[tauri::command]
pub async fn confirm_optimistic_update(
    state: State<'_, AppState>,
    update_id: String,
) -> Result<ApiResponse<Value>, AppError> {
    let result = state
        .offline_handler
        .confirm_optimistic_update(update_id)
        .await;
    Ok(ApiResponse::from_result(result))
}

/// 楽観的更新をロールバック
#[tauri::command]
pub async fn rollback_optimistic_update(
    state: State<'_, AppState>,
    update_id: String,
) -> Result<ApiResponse<Option<String>>, AppError> {
    let result = state
        .offline_handler
        .rollback_optimistic_update(update_id)
        .await;
    Ok(ApiResponse::from_result(result))
}

/// 期限切れキャッシュをクリーンアップ
#[tauri::command]
pub async fn cleanup_expired_cache(
    state: State<'_, AppState>,
) -> Result<ApiResponse<i32>, AppError> {
    let result = state.offline_handler.cleanup_expired_cache().await;
    Ok(ApiResponse::from_result(result))
}

/// 同期ステータスを更新
#[tauri::command]
pub async fn update_sync_status(
    state: State<'_, AppState>,
    entity_type: String,
    entity_id: String,
    sync_status: String,
    conflict_data: Option<String>,
) -> Result<ApiResponse<Value>, AppError> {
    let request = UpdateSyncStatusRequest {
        entity_type,
        entity_id,
        sync_status,
        conflict_data,
    };

    let result = state.offline_handler.update_sync_status(request).await;
    Ok(ApiResponse::from_result(result))
}
