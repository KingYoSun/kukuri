use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;
use sqlx::Row;
use tracing::info;

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
pub async fn get_topics(state: State<'_, AppState>) -> Result<Vec<Topic>, String> {
    // データベースから取得する実装
    let pool = &state.db_pool;
    
    // まずテーブルが存在するか確認し、存在しない場合は作成
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS topics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            topic_id TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            description TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )
        "#
    )
    .execute(pool.as_ref())
    .await
    .map_err(|e| format!("テーブル作成エラー: {}", e))?;
    
    // デフォルトトピックが存在しない場合は追加
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM topics WHERE topic_id = 'public'")
        .fetch_one(pool.as_ref())
        .await
        .unwrap_or(0);
        
    if count == 0 {
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            r#"
            INSERT INTO topics (topic_id, name, description, created_at, updated_at)
            VALUES ('public', '#public', '公開トピック - すべてのユーザーが参加できるメインのトピック', ?, ?)
            "#
        )
        .bind(now)
        .bind(now)
        .execute(pool.as_ref())
        .await
        .ok();
    }
    
    // 全トピックを取得
    let rows = sqlx::query(
        r#"
        SELECT topic_id, name, description, created_at, updated_at
        FROM topics
        ORDER BY created_at ASC
        "#
    )
    .fetch_all(pool.as_ref())
    .await
    .map_err(|e| format!("データベースエラー: {}", e))?;
    
    let topics = rows
        .iter()
        .map(|row| Topic {
            id: row.try_get("topic_id").unwrap_or_default(),
            name: row.try_get("name").unwrap_or_default(),
            description: row.try_get("description").unwrap_or_default(),
            created_at: row.try_get("created_at").unwrap_or(0),
            updated_at: row.try_get("updated_at").unwrap_or(0),
        })
        .collect();
    
    Ok(topics)
}

#[tauri::command]
pub async fn create_topic(
    state: State<'_, AppState>,
    request: CreateTopicRequest,
) -> Result<Topic, String> {
    // データベースに保存する実装
    let pool = &state.db_pool;
    let topic_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();
    
    sqlx::query(
        r#"
        INSERT INTO topics (topic_id, name, description, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?)
        "#
    )
    .bind(&topic_id)
    .bind(&request.name)
    .bind(&request.description)
    .bind(now)
    .bind(now)
    .execute(pool.as_ref())
    .await
    .map_err(|e| format!("データベースエラー: {}", e))?;
    
    let topic = Topic {
        id: topic_id,
        name: request.name,
        description: request.description,
        created_at: now,
        updated_at: now,
    };

    Ok(topic)
}

#[tauri::command]
pub async fn update_topic(
    state: State<'_, AppState>,
    request: UpdateTopicRequest,
) -> Result<Topic, String> {
    // データベースを更新する実装
    let pool = &state.db_pool;
    let now = chrono::Utc::now().timestamp();
    
    // 既存のcreated_atを取得
    let created_at: i64 = sqlx::query_scalar(
        "SELECT created_at FROM topics WHERE topic_id = ?"
    )
    .bind(&request.id)
    .fetch_one(pool.as_ref())
    .await
    .map_err(|e| format!("トピックが見つかりません: {}", e))?;
    
    // 更新
    sqlx::query(
        r#"
        UPDATE topics
        SET name = ?, description = ?, updated_at = ?
        WHERE topic_id = ?
        "#
    )
    .bind(&request.name)
    .bind(&request.description)
    .bind(now)
    .bind(&request.id)
    .execute(pool.as_ref())
    .await
    .map_err(|e| format!("データベースエラー: {}", e))?;
    
    let topic = Topic {
        id: request.id,
        name: request.name,
        description: request.description,
        created_at,
        updated_at: now,
    };

    Ok(topic)
}

#[tauri::command]
pub async fn delete_topic(state: State<'_, AppState>, id: String) -> Result<(), String> {
    // データベースから削除する実装
    let pool = &state.db_pool;
    
    // publicトピックは削除できない
    if id == "public" {
        return Err("デフォルトトピックは削除できません".to_string());
    }
    
    let result = sqlx::query(
        "DELETE FROM topics WHERE topic_id = ?"
    )
    .bind(&id)
    .execute(pool.as_ref())
    .await
    .map_err(|e| format!("データベースエラー: {}", e))?;
    
    if result.rows_affected() == 0 {
        return Err("トピックが見つかりません".to_string());
    }
    
    info!("Deleted topic: {id}");
    Ok(())
}
