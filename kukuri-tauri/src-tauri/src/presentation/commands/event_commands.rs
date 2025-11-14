use crate::presentation::dto::ApiResponse;
use crate::presentation::dto::event::{
    EventResponse, NostrMetadataDto, PublishTextNoteRequest, PublishTopicPostRequest,
    SendReactionRequest, SubscribeRequest, UpdateMetadataRequest,
};
use crate::shared::AppError;
use crate::state::AppState;
use serde_json::Value;
use tauri::State;

/// Nostrクライアントを初期化（ログイン時に呼び出す）
#[tauri::command]
pub async fn initialize_nostr(
    state: State<'_, AppState>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let result = state.event_handler.initialize_nostr().await;
    Ok(ApiResponse::from_result(result))
}

/// テキストノートを投稿
#[tauri::command]
pub async fn publish_text_note(
    content: String,
    state: State<'_, AppState>,
) -> Result<ApiResponse<EventResponse>, AppError> {
    let request = PublishTextNoteRequest { content };

    let result = state.event_handler.publish_text_note(request).await;
    Ok(ApiResponse::from_result(result))
}

/// トピック投稿を作成
#[tauri::command]
pub async fn publish_topic_post(
    topic_id: String,
    content: String,
    reply_to: Option<String>,
    state: State<'_, AppState>,
) -> Result<ApiResponse<EventResponse>, AppError> {
    let request = PublishTopicPostRequest {
        topic_id,
        content,
        reply_to,
    };

    let result = state.event_handler.publish_topic_post(request).await;
    Ok(ApiResponse::from_result(result))
}

/// リアクションを送信
#[tauri::command]
pub async fn send_reaction(
    event_id: String,
    reaction: String,
    state: State<'_, AppState>,
) -> Result<ApiResponse<EventResponse>, AppError> {
    let request = SendReactionRequest { event_id, reaction };

    let result = state.event_handler.send_reaction(request).await;
    Ok(ApiResponse::from_result(result))
}

/// メタデータを更新
#[tauri::command]
pub async fn update_nostr_metadata(
    metadata: NostrMetadataDto,
    state: State<'_, AppState>,
) -> Result<ApiResponse<EventResponse>, AppError> {
    let request = UpdateMetadataRequest { metadata };

    let result = state.event_handler.update_metadata(request).await;
    Ok(ApiResponse::from_result(result))
}

/// トピックをサブスクライブ
#[tauri::command]
pub async fn subscribe_to_topic(
    topic_id: String,
    state: State<'_, AppState>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let request = SubscribeRequest { topic_id };

    let result = state.event_handler.subscribe_to_topic(request).await;
    Ok(ApiResponse::from_result(result))
}

/// ユーザーをサブスクライブ
#[tauri::command]
pub async fn subscribe_to_user(
    pubkey: String,
    state: State<'_, AppState>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let result = state.event_handler.subscribe_to_user(pubkey).await;
    Ok(ApiResponse::from_result(result))
}

/// 現在のNostr購読状態を取得
#[tauri::command]
pub async fn list_nostr_subscriptions(
    state: State<'_, AppState>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let result = state.event_handler.list_subscriptions().await;
    Ok(ApiResponse::from_result(result))
}

/// Nostrクライアントを切断
#[tauri::command]
pub async fn disconnect_nostr(
    state: State<'_, AppState>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let result = state.event_handler.disconnect_nostr().await;
    Ok(ApiResponse::from_result(result))
}
