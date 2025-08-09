use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;
use sqlx::Row;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Post {
    pub id: String,
    pub content: String,
    pub author_pubkey: String,
    pub topic_id: String,
    pub created_at: i64,
    pub likes: u32,
    pub boosts: u32,
    pub replies: u32,
    pub is_synced: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePostRequest {
    pub content: String,
    pub topic_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetPostsRequest {
    pub topic_id: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[tauri::command]
pub async fn get_posts(
    state: State<'_, AppState>,
    request: GetPostsRequest,
) -> Result<Vec<Post>, String> {
    // データベースから投稿を取得
    let pool = &state.db_pool;
    let limit = request.limit.unwrap_or(50) as i64;
    let offset = request.offset.unwrap_or(0) as i64;
    
    let mut posts = vec![];
    
    // トピックIDでフィルタリングするかどうか
    let rows = if let Some(topic_id) = &request.topic_id {
        // トピックタグを含む投稿を検索
        let topic_tag = format!(r#"["t","{}"]"#, topic_id);
        sqlx::query(
            r#"
            SELECT event_id, public_key, content, created_at, tags
            FROM events
            WHERE kind = 1
            AND tags LIKE '%' || ? || '%'
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#
        )
        .bind(&topic_tag)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool.as_ref())
        .await
        .map_err(|e| format!("データベースエラー: {}", e))?
    } else {
        // 全ての投稿を取得
        sqlx::query(
            r#"
            SELECT event_id, public_key, content, created_at, tags
            FROM events
            WHERE kind = 1
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool.as_ref())
        .await
        .map_err(|e| format!("データベースエラー: {}", e))?
    };
    
    // 結果をPost構造体に変換
    for row in rows {
        let event_id: String = row.try_get("event_id").unwrap_or_default();
        let public_key: String = row.try_get("public_key").unwrap_or_default();
        let content: String = row.try_get("content").unwrap_or_default();
        let created_at: i64 = row.try_get("created_at").unwrap_or(0);
        let tags_json: String = row.try_get("tags").unwrap_or_default();
        
        // タグからトピックIDを抽出
        let mut topic_id = String::new();
        if let Ok(tags) = serde_json::from_str::<Vec<Vec<String>>>(&tags_json) {
            for tag in tags {
                if tag.len() >= 2 && tag[0] == "t" {
                    topic_id = tag[1].clone();
                    break;
                }
            }
        }
        
        // いいね、ブースト、返信の数を取得（簡易版：今は0）
        // 将来的には関連イベントをカウントする
        
        posts.push(Post {
            id: event_id,
            content,
            author_pubkey: public_key,
            topic_id,
            created_at,
            likes: 0,
            boosts: 0,
            replies: 0,
            is_synced: true, // DBに保存されているものは同期済み
        });
    }
    
    Ok(posts)
}

#[tauri::command]
pub async fn create_post(
    state: State<'_, AppState>,
    request: CreatePostRequest,
) -> Result<Post, String> {
    // Nostrイベントとして発行し、データベースに保存する実装
    
    // 現在のユーザーの公開鍵を取得
    let keys = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {e}"))?;

    // EventManagerを使用してトピック投稿を作成・送信
    let event_id = state
        .event_manager
        .publish_topic_post(&request.topic_id, &request.content, None)
        .await
        .map_err(|e| format!("投稿の作成に失敗しました: {e}"))?;

    let post = Post {
        id: event_id.to_hex(),
        content: request.content,
        author_pubkey: keys.public_key().to_hex(),
        topic_id: request.topic_id,
        created_at: chrono::Utc::now().timestamp(),
        likes: 0,
        boosts: 0,
        replies: 0,
        is_synced: true, // Nostrに送信済み
    };

    Ok(post)
}

#[tauri::command]
pub async fn delete_post(state: State<'_, AppState>, id: String) -> Result<(), String> {
    use nostr_sdk::EventId;
    
    // 現在のユーザーの確認
    let _keys = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {e}"))?;
    
    // EventIdに変換
    let event_id = EventId::from_hex(&id).map_err(|e| format!("無効なイベントID: {e}"))?;
    
    // EventPublisherを使用して削除イベントを作成
    let publisher = state.event_manager.event_publisher.read().await;
    let deletion_event = publisher
        .create_deletion(vec![event_id], Some("投稿の削除"))
        .map_err(|e| format!("削除イベントの作成に失敗しました: {e}"))?;
    
    // 削除イベントを送信
    let client_manager = state.event_manager.client_manager.read().await;
    client_manager
        .publish_event(deletion_event)
        .await
        .map_err(|e| format!("削除イベントの送信に失敗しました: {e}"))?;
    
    println!("Deleted post: {id}");
    Ok(())
}

#[tauri::command]
pub async fn like_post(state: State<'_, AppState>, post_id: String) -> Result<(), String> {
    use nostr_sdk::EventId;
    
    // 現在のユーザーの確認
    let _keys = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {e}"))?;

    // EventId型に変換
    let event_id = EventId::from_hex(&post_id).map_err(|e| format!("無効なイベントID: {e}"))?;

    // Nostrリアクションイベントを発行（「+」はNIP-25で定義された標準的ないいね）
    state
        .event_manager
        .send_reaction(&event_id, "+")
        .await
        .map_err(|e| format!("いいねに失敗しました: {e}"))?;

    println!("Liked post: {post_id}");
    Ok(())
}

#[tauri::command]
pub async fn boost_post(state: State<'_, AppState>, post_id: String) -> Result<(), String> {
    // 現在のユーザーの確認
    let _keys = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {e}"))?;

    // ブースト機能は現在未実装
    Err(format!("ブースト機能は現在実装中です (post_id: {})", post_id))
}

#[tauri::command]
pub async fn bookmark_post(state: State<'_, AppState>, post_id: String) -> Result<(), String> {
    // 現在のユーザーの確認
    let keys = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {e}"))?;

    let user_pubkey = keys.public_key().to_hex();

    // ブックマークを追加
    state
        .bookmark_manager
        .add_bookmark(&user_pubkey, &post_id)
        .await
        .map_err(|e| format!("ブックマークの追加に失敗しました: {e}"))?;

    println!("Bookmarked post: {post_id}");
    Ok(())
}

#[tauri::command]
pub async fn unbookmark_post(state: State<'_, AppState>, post_id: String) -> Result<(), String> {
    // 現在のユーザーの確認
    let keys = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {e}"))?;

    let user_pubkey = keys.public_key().to_hex();

    // ブックマークを削除
    state
        .bookmark_manager
        .remove_bookmark(&user_pubkey, &post_id)
        .await
        .map_err(|e| format!("ブックマークの削除に失敗しました: {e}"))?;

    println!("Unbookmarked post: {post_id}");
    Ok(())
}

#[tauri::command]
pub async fn get_bookmarked_post_ids(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    // 現在のユーザーの確認
    let keys = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {e}"))?;

    let user_pubkey = keys.public_key().to_hex();

    // ブックマークされた投稿IDのリストを取得
    let post_ids = state
        .bookmark_manager
        .get_bookmarked_post_ids(&user_pubkey)
        .await
        .map_err(|e| format!("ブックマークの取得に失敗しました: {e}"))?;

    Ok(post_ids)
}
