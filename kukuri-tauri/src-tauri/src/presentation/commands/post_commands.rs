use crate::{
    presentation::{
        dto::{
            ApiResponse,
            post_dto::{
                BatchBookmarkRequest, BatchGetPostsRequest, BatchReactRequest, BookmarkPostRequest,
                CreatePostRequest, DeletePostRequest, GetPostsRequest, PostResponse,
                ReactToPostRequest,
            },
        },
        handlers::PostHandler,
    },
    shared::AppError,
    state::AppState,
};
use tauri::State;

async fn ensure_authenticated(state: &State<'_, AppState>) -> Result<String, AppError> {
    let keys = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| AppError::Unauthorized(format!("ログインが必要です: {e}")))?;
    Ok(keys.public_key().to_hex())
}

/// 投稿を作成する
#[tauri::command]
pub async fn create_post(
    state: State<'_, AppState>,
    request: CreatePostRequest,
) -> Result<ApiResponse<PostResponse>, AppError> {
    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    let result = handler.create_post(request).await;
    Ok(ApiResponse::from_result(result))
}

/// 投稿を取得する
#[tauri::command]
pub async fn get_posts(
    state: State<'_, AppState>,
    request: GetPostsRequest,
) -> Result<ApiResponse<Vec<PostResponse>>, AppError> {
    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    let result = handler.get_posts(request).await;
    Ok(ApiResponse::from_result(result))
}

/// 投稿を削除する
#[tauri::command]
pub async fn delete_post(
    state: State<'_, AppState>,
    request: DeletePostRequest,
) -> Result<ApiResponse<()>, AppError> {
    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    let result = handler.delete_post(request).await;
    Ok(ApiResponse::from_result(result))
}

/// 投稿にリアクションする
#[tauri::command]
pub async fn react_to_post(
    state: State<'_, AppState>,
    request: ReactToPostRequest,
) -> Result<ApiResponse<()>, AppError> {
    ensure_authenticated(&state).await?;
    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    let result = handler.react_to_post(request).await;
    Ok(ApiResponse::from_result(result))
}

/// 投稿をブックマークする
#[tauri::command]
pub async fn bookmark_post(
    state: State<'_, AppState>,
    request: BookmarkPostRequest,
) -> Result<ApiResponse<()>, AppError> {
    let user_pubkey = ensure_authenticated(&state).await?;
    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    let result = handler.bookmark_post(request, &user_pubkey).await;
    Ok(ApiResponse::from_result(result))
}

/// ブックマークを解除する
#[tauri::command]
pub async fn unbookmark_post(
    state: State<'_, AppState>,
    request: BookmarkPostRequest,
) -> Result<ApiResponse<()>, AppError> {
    let user_pubkey = ensure_authenticated(&state).await?;
    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    let result = handler.unbookmark_post(request, &user_pubkey).await;
    Ok(ApiResponse::from_result(result))
}

/// 投稿にいいねする（旧APIとの互換性のため）
#[tauri::command]
pub async fn like_post(
    state: State<'_, AppState>,
    post_id: String,
) -> Result<ApiResponse<()>, AppError> {
    let request = ReactToPostRequest {
        post_id,
        reaction: "+".to_string(),
    };

    react_to_post(state, request).await
}

/// ブックマーク済み投稿IDを取得する
#[tauri::command]
pub async fn get_bookmarked_post_ids(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<String>>, AppError> {
    let user_pubkey = ensure_authenticated(&state).await?;
    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    let result = handler.get_bookmarked_post_ids(&user_pubkey).await;
    Ok(ApiResponse::from_result(result))
}

// バッチ処理コマンド

/// 複数の投稿を一括取得する
#[tauri::command]
pub async fn batch_get_posts(
    state: State<'_, AppState>,
    request: BatchGetPostsRequest,
) -> Result<ApiResponse<Vec<PostResponse>>, AppError> {
    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    let result = handler.batch_get_posts(request).await;
    Ok(ApiResponse::from_result(result))
}

/// 複数のリアクションを一括処理する
#[tauri::command]
pub async fn batch_react(
    state: State<'_, AppState>,
    request: BatchReactRequest,
) -> Result<ApiResponse<Vec<Result<(), String>>>, AppError> {
    ensure_authenticated(&state).await?;
    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    let result = handler.batch_react(request).await;
    Ok(ApiResponse::from_result(result))
}

/// 複数のブックマークを一括処理する
#[tauri::command]
pub async fn batch_bookmark(
    state: State<'_, AppState>,
    request: BatchBookmarkRequest,
) -> Result<ApiResponse<Vec<Result<(), String>>>, AppError> {
    let user_pubkey = ensure_authenticated(&state).await?;
    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    let result = handler.batch_bookmark(request, &user_pubkey).await;
    Ok(ApiResponse::from_result(result))
}

/// 投稿をブーストする（旧APIとの互換性のため）
#[tauri::command]
pub async fn boost_post(
    state: State<'_, AppState>,
    post_id: String,
) -> Result<ApiResponse<()>, AppError> {
    let request = ReactToPostRequest {
        post_id,
        reaction: "boost".to_string(),
    };

    react_to_post(state, request).await
}

/// 単一の投稿を取得する（旧APIとの互換性のため）
#[tauri::command]
pub async fn get_post(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<serde_json::Value>, AppError> {
    state
        .post_service
        .get_post(&id)
        .await?
        .map(|p| serde_json::to_value(p).map_err(AppError::from))
        .transpose()
}

/// トピック別の投稿を取得する（旧APIとの互換性のため）
#[tauri::command]
pub async fn get_posts_by_topic(
    state: State<'_, AppState>,
    topic_id: String,
    limit: Option<usize>,
) -> Result<Vec<serde_json::Value>, AppError> {
    let posts = state
        .post_service
        .get_posts_by_topic(&topic_id, limit.unwrap_or(50))
        .await?;

    posts
        .into_iter()
        .map(|p| serde_json::to_value(p).map_err(AppError::from))
        .collect()
}

/// 保留中の投稿を同期する
#[tauri::command]
pub async fn sync_posts(state: State<'_, AppState>) -> Result<u32, AppError> {
    state.post_service.sync_pending_posts().await
}
