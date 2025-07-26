use serde::{Deserialize, Serialize};
use tauri::State;
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Topic {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTopicRequest {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTopicRequest {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[tauri::command]
pub async fn get_topics(
    _state: State<'_, AppState>
) -> Result<Vec<Topic>, String> {
    // TODO: データベースから取得する実装
    // 現在はモックデータを返す
    Ok(vec![
        Topic {
            id: "1".to_string(),
            name: "Nostr開発".to_string(),
            description: "Nostrプロトコルに関する開発の話題".to_string(),
            created_at: 1722000000,
            updated_at: 1722000000,
        },
        Topic {
            id: "2".to_string(),
            name: "暗号技術".to_string(),
            description: "暗号化と分散システムについて".to_string(),
            created_at: 1722000100,
            updated_at: 1722000100,
        },
    ])
}

#[tauri::command]
pub async fn create_topic(
    _state: State<'_, AppState>,
    request: CreateTopicRequest
) -> Result<Topic, String> {
    // TODO: データベースに保存する実装
    let topic = Topic {
        id: uuid::Uuid::new_v4().to_string(),
        name: request.name,
        description: request.description,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };
    
    Ok(topic)
}

#[tauri::command]
pub async fn update_topic(
    _state: State<'_, AppState>,
    request: UpdateTopicRequest
) -> Result<Topic, String> {
    // TODO: データベースを更新する実装
    let topic = Topic {
        id: request.id,
        name: request.name,
        description: request.description,
        created_at: 1722000000, // TODO: 実際の値を取得
        updated_at: chrono::Utc::now().timestamp(),
    };
    
    Ok(topic)
}

#[tauri::command]
pub async fn delete_topic(
    _state: State<'_, AppState>,
    id: String
) -> Result<(), String> {
    // TODO: データベースから削除する実装
    println!("Deleting topic: {}", id);
    Ok(())
}