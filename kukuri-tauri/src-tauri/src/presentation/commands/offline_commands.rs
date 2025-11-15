use crate::presentation::dto::ApiResponse;
use crate::infrastructure::offline::metrics::{
    self, OfflineRetryMetricsSnapshot, RetryOutcomeMetadata, RetryOutcomeStatus,
};
use crate::presentation::dto::offline::{
    AddToSyncQueueRequest, CacheStatusResponse, GetOfflineActionsRequest,
    ListSyncQueueItemsRequest, OfflineAction, OfflineRetryMetricsResponse, OptimisticUpdateRequest,
    RecordOfflineRetryOutcomeRequest, SaveOfflineActionRequest, SaveOfflineActionResponse,
    SyncOfflineActionsRequest, SyncOfflineActionsResponse, SyncQueueItemResponse,
    UpdateCacheMetadataRequest, UpdateSyncStatusRequest,
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

/// 同期キューの状態を取得
#[tauri::command]
pub async fn list_sync_queue_items(
    state: State<'_, AppState>,
    request: ListSyncQueueItemsRequest,
) -> Result<ApiResponse<Vec<SyncQueueItemResponse>>, AppError> {
    let result = state.offline_handler.list_sync_queue_items(request).await;
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

#[tauri::command]
pub async fn record_offline_retry_outcome(
    request: RecordOfflineRetryOutcomeRequest,
) -> Result<ApiResponse<OfflineRetryMetricsResponse>, AppError> {
    request.validate()?;
    let status = match request.status.as_str() {
        "success" => RetryOutcomeStatus::Success,
        _ => RetryOutcomeStatus::Failure,
    };

    let metadata = RetryOutcomeMetadata {
        job_id: request.job_id.clone(),
        job_reason: request.job_reason.clone(),
        trigger: request.trigger.clone(),
        user_pubkey: request.user_pubkey.clone(),
        retry_count: request.retry_count.map(|value| value as u32),
        max_retries: request.max_retries.map(|value| value as u32),
        backoff_ms: request.backoff_ms,
        duration_ms: request.duration_ms,
        success_count: request.success_count.map(|value| value as u32),
        failure_count: request.failure_count.map(|value| value as u32),
        timestamp_ms: request.timestamp_ms,
    };

    let snapshot = metrics::record_outcome(status, &metadata);
    Ok(ApiResponse::success(snapshot.into()))
}

#[tauri::command]
pub async fn get_offline_retry_metrics(
) -> Result<ApiResponse<OfflineRetryMetricsResponse>, AppError> {
    let snapshot = metrics::snapshot();
    Ok(ApiResponse::success(snapshot.into()))
}

impl From<OfflineRetryMetricsSnapshot> for OfflineRetryMetricsResponse {
    fn from(value: OfflineRetryMetricsSnapshot) -> Self {
        Self {
            total_success: value.total_success,
            total_failure: value.total_failure,
            consecutive_failure: value.consecutive_failure,
            last_success_ms: value.last_success_ms,
            last_failure_ms: value.last_failure_ms,
            last_outcome: value
                .last_outcome
                .map(|status| match status {
                    RetryOutcomeStatus::Success => "success".to_string(),
                    RetryOutcomeStatus::Failure => "failure".to_string(),
                }),
            last_job_id: value.last_job_id,
            last_job_reason: value.last_job_reason,
            last_trigger: value.last_trigger,
            last_user_pubkey: value.last_user_pubkey,
            last_retry_count: value.last_retry_count.map(|v| v as i32),
            last_max_retries: value.last_max_retries.map(|v| v as i32),
            last_backoff_ms: value.last_backoff_ms,
            last_duration_ms: value.last_duration_ms,
            last_success_count: value.last_success_count.map(|v| v as i32),
            last_failure_count: value.last_failure_count.map(|v| v as i32),
            last_timestamp_ms: value.last_timestamp_ms,
        }
    }
}
