use crate::presentation::dto::event::{
    DeleteEventsRequest, EventResponse, NostrMetadataDto, PublishTextNoteRequest,
    PublishTopicPostRequest, SendReactionRequest, SetDefaultP2PTopicRequest, SubscribeRequest,
    UpdateMetadataRequest,
};
use crate::state::AppState;
use tauri::State;

/// Nostrクライアントを初期化（ログイン時に呼び出す）
#[tauri::command]
pub async fn initialize_nostr(state: State<'_, AppState>) -> Result<String, String> {
    state
        .event_handler
        .initialize_nostr()
        .await
        .map(|response| serde_json::to_string(&response).unwrap())
        .map_err(|e| e.to_string())
}

/// テキストノートを投稿
#[tauri::command]
pub async fn publish_text_note(
    content: String,
    state: State<'_, AppState>,
) -> Result<EventResponse, String> {
    let request = PublishTextNoteRequest { content };

    state
        .event_handler
        .publish_text_note(request)
        .await
        .map_err(|e| e.to_string())
}

/// トピック投稿を作成
#[tauri::command]
pub async fn publish_topic_post(
    topic_id: String,
    content: String,
    reply_to: Option<String>,
    state: State<'_, AppState>,
) -> Result<EventResponse, String> {
    let request = PublishTopicPostRequest {
        topic_id,
        content,
        reply_to,
    };

    state
        .event_handler
        .publish_topic_post(request)
        .await
        .map_err(|e| e.to_string())
}

/// リアクションを送信
#[tauri::command]
pub async fn send_reaction(
    event_id: String,
    reaction: String,
    state: State<'_, AppState>,
) -> Result<EventResponse, String> {
    let request = SendReactionRequest { event_id, reaction };

    state
        .event_handler
        .send_reaction(request)
        .await
        .map_err(|e| e.to_string())
}

/// メタデータを更新
#[tauri::command]
pub async fn update_nostr_metadata(
    metadata: NostrMetadataDto,
    state: State<'_, AppState>,
) -> Result<EventResponse, String> {
    let request = UpdateMetadataRequest { metadata };

    state
        .event_handler
        .update_metadata(request)
        .await
        .map_err(|e| e.to_string())
}

/// トピックをサブスクライブ
#[tauri::command]
pub async fn subscribe_to_topic(
    topic_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let request = SubscribeRequest { topic_id };

    state
        .event_handler
        .subscribe_to_topic(request)
        .await
        .map(|response| serde_json::to_string(&response).unwrap())
        .map_err(|e| e.to_string())
}

/// ユーザーをサブスクライブ
#[tauri::command]
pub async fn subscribe_to_user(
    pubkey: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    state
        .event_handler
        .subscribe_to_user(pubkey)
        .await
        .map(|response| serde_json::to_string(&response).unwrap())
        .map_err(|e| e.to_string())
}

/// Nostr公開鍵を取得
#[tauri::command]
pub async fn get_nostr_pubkey(state: State<'_, AppState>) -> Result<String, String> {
    state
        .event_handler
        .get_nostr_pubkey()
        .await
        .map(|response| serde_json::to_string(&response).unwrap())
        .map_err(|e| e.to_string())
}

/// イベントを削除
#[tauri::command]
pub async fn delete_events(
    event_ids: Vec<String>,
    reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<EventResponse, String> {
    let request = DeleteEventsRequest { event_ids, reason };

    state
        .event_handler
        .delete_events(request)
        .await
        .map_err(|e| e.to_string())
}

/// Nostrクライアントを切断
#[tauri::command]
pub async fn disconnect_nostr(state: State<'_, AppState>) -> Result<String, String> {
    state
        .event_handler
        .disconnect_nostr()
        .await
        .map(|response| serde_json::to_string(&response).unwrap())
        .map_err(|e| e.to_string())
}

/// 既定のP2P配信トピックを設定
#[tauri::command]
pub async fn set_default_p2p_topic(
    topic_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let request = SetDefaultP2PTopicRequest { topic_id };
    state
        .event_handler
        .set_default_p2p_topic(request)
        .await
        .map(|response| serde_json::to_string(&response).unwrap())
        .map_err(|e| e.to_string())
}
