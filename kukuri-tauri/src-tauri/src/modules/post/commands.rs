use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Post {
    pub id: String,
    pub content: String,
    pub author_pubkey: String,
    pub topic_id: String,
    pub created_at: i64,
    pub likes: u32,
    pub replies: u32,
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
    _state: State<'_, AppState>,
    request: GetPostsRequest,
) -> Result<Vec<Post>, String> {
    // TODO: データベースから取得する実装
    // 現在はモックデータを返す
    let limit = request.limit.unwrap_or(20);
    let mut posts = vec![];

    for i in 0..limit.min(5) {
        posts.push(Post {
            id: format!("post_{i}"),
            content: format!("これはテスト投稿 {i} です。"),
            author_pubkey: "test_pubkey".to_string(),
            topic_id: request.topic_id.clone().unwrap_or_else(|| "1".to_string()),
            created_at: 1722000000 + (i as i64 * 100),
            likes: i * 2,
            replies: i,
        });
    }

    Ok(posts)
}

#[tauri::command]
pub async fn create_post(
    state: State<'_, AppState>,
    request: CreatePostRequest,
) -> Result<Post, String> {
    // TODO: Nostrイベントとして発行し、データベースに保存する実装

    // 現在のユーザーの公開鍵を取得
    let keys = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {e}"))?;

    let post = Post {
        id: uuid::Uuid::new_v4().to_string(),
        content: request.content,
        author_pubkey: keys.public_key().to_hex(),
        topic_id: request.topic_id,
        created_at: chrono::Utc::now().timestamp(),
        likes: 0,
        replies: 0,
    };

    Ok(post)
}

#[tauri::command]
pub async fn delete_post(state: State<'_, AppState>, id: String) -> Result<(), String> {
    // TODO: Nostrイベントとして削除イベントを発行する実装

    // 現在のユーザーの確認
    let _keys = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {e}"))?;

    println!("Deleting post: {id}");
    Ok(())
}

#[tauri::command]
pub async fn like_post(state: State<'_, AppState>, post_id: String) -> Result<(), String> {
    // TODO: Nostrリアクションイベントを発行する実装

    // 現在のユーザーの確認
    let _keys = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {e}"))?;

    println!("Liking post: {post_id}");
    Ok(())
}
