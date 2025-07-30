use crate::state::AppState;
use nostr_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize)]
pub struct NostrMetadata {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
    pub banner: Option<String>,
    pub nip05: Option<String>,
    pub lud16: Option<String>,
    pub website: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct RelayInfo {
    pub url: String,
    pub status: String,
}

#[derive(Serialize, Deserialize)]
pub struct NostrEvent {
    pub id: String,
    pub author: String,
    pub content: String,
    pub created_at: u64,
    pub kind: u32,
    pub tags: Vec<Vec<String>>,
}

/// Nostrクライアントを初期化（ログイン時に呼び出す）
#[tauri::command]
pub async fn initialize_nostr(state: State<'_, AppState>) -> Result<(), String> {
    let event_manager = &state.event_manager;
    let key_manager = &state.key_manager;

    // KeyManagerから鍵を使用して初期化
    event_manager
        .initialize_with_key_manager(key_manager)
        .await
        .map_err(|e| e.to_string())?;

    // 既存のNostrリレーへの接続を無効化
    // デフォルトリレーに接続
    // event_manager.connect_to_default_relays()
    //     .await
    //     .map_err(|e| e.to_string())?;

    // イベントストリームを開始
    // event_manager.start_event_stream()
    //     .await
    //     .map_err(|e| e.to_string())?;

    Ok(())
}

/// リレーを追加
#[tauri::command]
pub async fn add_relay(url: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .event_manager
        .add_relay(&url)
        .await
        .map_err(|e| e.to_string())
}

/// テキストノートを投稿
#[tauri::command]
pub async fn publish_text_note(
    content: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let event_id = state
        .event_manager
        .publish_text_note(&content)
        .await
        .map_err(|e| e.to_string())?;

    Ok(event_id.to_string())
}

/// トピック投稿を作成
#[tauri::command]
pub async fn publish_topic_post(
    topic_id: String,
    content: String,
    reply_to: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let reply_event_id = if let Some(reply_id) = reply_to {
        Some(EventId::from_hex(&reply_id).map_err(|e| e.to_string())?)
    } else {
        None
    };

    let event_id = state
        .event_manager
        .publish_topic_post(&topic_id, &content, reply_event_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(event_id.to_string())
}

/// リアクションを送信
#[tauri::command]
pub async fn send_reaction(
    event_id: String,
    reaction: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let target_event_id = EventId::from_hex(&event_id).map_err(|e| e.to_string())?;

    let reaction_event_id = state
        .event_manager
        .send_reaction(&target_event_id, &reaction)
        .await
        .map_err(|e| e.to_string())?;

    Ok(reaction_event_id.to_string())
}

/// メタデータを更新
#[tauri::command]
pub async fn update_nostr_metadata(
    metadata: NostrMetadata,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let mut nostr_metadata = Metadata::new();

    if let Some(name) = metadata.name {
        nostr_metadata = nostr_metadata.name(name);
    }
    if let Some(display_name) = metadata.display_name {
        nostr_metadata = nostr_metadata.display_name(display_name);
    }
    if let Some(about) = metadata.about {
        nostr_metadata = nostr_metadata.about(about);
    }
    if let Some(picture) = metadata.picture {
        if let Ok(url) = Url::parse(&picture) {
            nostr_metadata = nostr_metadata.picture(url);
        }
    }
    if let Some(banner) = metadata.banner {
        if let Ok(url) = Url::parse(&banner) {
            nostr_metadata = nostr_metadata.banner(url);
        }
    }
    if let Some(nip05) = metadata.nip05 {
        nostr_metadata = nostr_metadata.nip05(nip05);
    }
    if let Some(lud16) = metadata.lud16 {
        nostr_metadata = nostr_metadata.lud16(lud16);
    }
    if let Some(website) = metadata.website {
        if let Ok(url) = Url::parse(&website) {
            nostr_metadata = nostr_metadata.website(url);
        }
    }

    let event_id = state
        .event_manager
        .update_metadata(nostr_metadata)
        .await
        .map_err(|e| e.to_string())?;

    Ok(event_id.to_string())
}

/// トピックをサブスクライブ
#[tauri::command]
pub async fn subscribe_to_topic(
    topic_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .event_manager
        .subscribe_to_topic(&topic_id)
        .await
        .map_err(|e| e.to_string())
}

/// ユーザーをサブスクライブ
#[tauri::command]
pub async fn subscribe_to_user(pubkey: String, state: State<'_, AppState>) -> Result<(), String> {
    let public_key = PublicKey::from_hex(&pubkey).map_err(|e| e.to_string())?;

    state
        .event_manager
        .subscribe_to_user(public_key)
        .await
        .map_err(|e| e.to_string())
}

/// Nostr公開鍵を取得
#[tauri::command]
pub async fn get_nostr_pubkey(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let pubkey = state.event_manager.get_public_key().await;
    Ok(pubkey.map(|pk| pk.to_hex()))
}

/// イベントを削除
#[tauri::command]
pub async fn delete_events(
    event_ids: Vec<String>,
    reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let event_manager = &state.event_manager;
    let publisher = event_manager.event_publisher.read().await;

    let ids: Result<Vec<EventId>, _> = event_ids.iter().map(|id| EventId::from_hex(id)).collect();

    let ids = ids.map_err(|e| e.to_string())?;

    let event = publisher
        .create_deletion(ids, reason.as_deref())
        .map_err(|e| e.to_string())?;

    let client_manager = event_manager.client_manager.read().await;
    let event_id = client_manager
        .publish_event(event)
        .await
        .map_err(|e| e.to_string())?;

    Ok(event_id.to_string())
}

/// Nostrクライアントを切断
#[tauri::command]
pub async fn disconnect_nostr(state: State<'_, AppState>) -> Result<(), String> {
    state
        .event_manager
        .disconnect()
        .await
        .map_err(|e| e.to_string())
}

/// リレーの接続状態を取得
#[tauri::command]
pub async fn get_relay_status(state: State<'_, AppState>) -> Result<Vec<RelayInfo>, String> {
    let status = state
        .event_manager
        .get_relay_status()
        .await
        .map_err(|e| e.to_string())?;

    let relay_info: Vec<RelayInfo> = status
        .into_iter()
        .map(|(url, status)| RelayInfo { url, status })
        .collect();

    Ok(relay_info)
}
