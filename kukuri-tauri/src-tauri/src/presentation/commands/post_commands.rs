use crate::application::services::{AuthService, PostService};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePostRequest {
    pub content: String,
    pub topic_id: String,
    pub tags: Vec<String>,
}

// v2コマンドに移行
// #[tauri::command]
// pub async fn create_post(
//     request: CreatePostRequest,
//     post_service: State<'_, Arc<PostService>>,
//     auth_service: State<'_, Arc<AuthService>>,
// ) -> Result<serde_json::Value, String> {
//     let current_user = auth_service
//         .get_current_user()
//         .await
//         .map_err(|e| e.to_string())?
//         .ok_or("Not authenticated")?;

//     let mut post = post_service
//         .create_post(request.content, current_user, request.topic_id)
//         .await
//         .map_err(|e| e.to_string())?;

//     if !request.tags.is_empty() {
//         post = post.with_tags(request.tags);
//     }

//     serde_json::to_value(post).map_err(|e| e.to_string())
// }

#[tauri::command]
pub async fn get_post(
    id: String,
    post_service: State<'_, Arc<PostService>>,
) -> Result<Option<serde_json::Value>, String> {
    let post = post_service
        .get_post(&id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(post.map(|p| serde_json::to_value(p).unwrap()))
}

#[tauri::command]
pub async fn get_posts_by_topic(
    topic_id: String,
    limit: Option<usize>,
    post_service: State<'_, Arc<PostService>>,
) -> Result<Vec<serde_json::Value>, String> {
    let posts = post_service
        .get_posts_by_topic(&topic_id, limit.unwrap_or(50))
        .await
        .map_err(|e| e.to_string())?;

    Ok(posts
        .into_iter()
        .map(|p| serde_json::to_value(p).unwrap())
        .collect())
}

// v2コマンドに移行
// #[tauri::command]
// pub async fn like_post(
//     post_id: String,
//     post_service: State<'_, Arc<PostService>>,
// ) -> Result<(), String> {
//     post_service
//         .like_post(&post_id)
//         .await
//         .map_err(|e| e.to_string())
// }

// v2コマンドに移行
// #[tauri::command]
// pub async fn boost_post(
//     post_id: String,
//     post_service: State<'_, Arc<PostService>>,
// ) -> Result<(), String> {
//     post_service
//         .boost_post(&post_id)
//         .await
//         .map_err(|e| e.to_string())
// }

// v2コマンドに移行
// #[tauri::command]
// pub async fn delete_post(
//     id: String,
//     post_service: State<'_, Arc<PostService>>,
// ) -> Result<(), String> {
//     post_service
//         .delete_post(&id)
//         .await
//         .map_err(|e| e.to_string())
// }

#[tauri::command]
pub async fn sync_posts(
    post_service: State<'_, Arc<PostService>>,
) -> Result<u32, String> {
    post_service
        .sync_pending_posts()
        .await
        .map_err(|e| e.to_string())
}