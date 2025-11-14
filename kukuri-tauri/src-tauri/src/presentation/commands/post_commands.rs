use crate::{
    presentation::{
        dto::{
            ApiResponse,
            post_dto::{
                BookmarkPostRequest, CreatePostRequest, DeletePostRequest,
                FollowingFeedPageResponse, GetPostsRequest, ListFollowingFeedRequest,
                ListTrendingPostsRequest, ListTrendingPostsResponse, PostResponse,
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
    state
        .key_manager
        .current_keypair()
        .await
        .map(|pair| pair.public_key.clone())
        .map_err(|e| AppError::Unauthorized(format!("ログインが必要です: {e}")))
}

/// 投稿を作成する
#[tauri::command]
pub async fn create_post(
    state: State<'_, AppState>,
    request: CreatePostRequest,
) -> Result<ApiResponse<PostResponse>, AppError> {
    let handler = PostHandler::new(
        state.post_service.clone(),
        state.auth_service.clone(),
        state.topic_service.clone(),
    );
    let result = handler.create_post(request).await;
    Ok(ApiResponse::from_result(result))
}

/// 投稿を取得する
#[tauri::command]
pub async fn get_posts(
    state: State<'_, AppState>,
    request: GetPostsRequest,
) -> Result<ApiResponse<Vec<PostResponse>>, AppError> {
    let handler = PostHandler::new(
        state.post_service.clone(),
        state.auth_service.clone(),
        state.topic_service.clone(),
    );
    let result = handler.get_posts(request).await;
    Ok(ApiResponse::from_result(result))
}

/// トレンドトピックごとの投稿を取得する
#[tauri::command]
pub async fn list_trending_posts(
    state: State<'_, AppState>,
    request: ListTrendingPostsRequest,
) -> Result<ApiResponse<ListTrendingPostsResponse>, AppError> {
    let handler = PostHandler::new(
        state.post_service.clone(),
        state.auth_service.clone(),
        state.topic_service.clone(),
    );
    let result = handler.list_trending_posts(request).await;
    Ok(ApiResponse::from_result(result))
}

/// 投稿を削除する
#[tauri::command]
pub async fn delete_post(
    state: State<'_, AppState>,
    request: DeletePostRequest,
) -> Result<ApiResponse<()>, AppError> {
    let handler = PostHandler::new(
        state.post_service.clone(),
        state.auth_service.clone(),
        state.topic_service.clone(),
    );
    let result = handler.delete_post(request).await;
    Ok(ApiResponse::from_result(result))
}

/// 投稿にリアクションする（内部ヘルパー）
async fn react_to_post(
    state: &State<'_, AppState>,
    request: ReactToPostRequest,
) -> Result<ApiResponse<()>, AppError> {
    ensure_authenticated(state).await?;
    let handler = PostHandler::new(
        state.post_service.clone(),
        state.auth_service.clone(),
        state.topic_service.clone(),
    );
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
    let handler = PostHandler::new(
        state.post_service.clone(),
        state.auth_service.clone(),
        state.topic_service.clone(),
    );
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
    let handler = PostHandler::new(
        state.post_service.clone(),
        state.auth_service.clone(),
        state.topic_service.clone(),
    );
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

    react_to_post(&state, request).await
}

/// ブックマーク済み投稿IDを取得する
#[tauri::command]
pub async fn get_bookmarked_post_ids(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<String>>, AppError> {
    let user_pubkey = ensure_authenticated(&state).await?;
    let handler = PostHandler::new(
        state.post_service.clone(),
        state.auth_service.clone(),
        state.topic_service.clone(),
    );
    let result = handler.get_bookmarked_post_ids(&user_pubkey).await;
    Ok(ApiResponse::from_result(result))
}

/// フォロー中フィードを取得する
#[tauri::command]
pub async fn list_following_feed(
    state: State<'_, AppState>,
    request: ListFollowingFeedRequest,
) -> Result<ApiResponse<FollowingFeedPageResponse>, AppError> {
    let user_pubkey = ensure_authenticated(&state).await?;
    let handler = PostHandler::new(
        state.post_service.clone(),
        state.auth_service.clone(),
        state.topic_service.clone(),
    );
    let result = handler.list_following_feed(&user_pubkey, request).await;
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

    react_to_post(&state, request).await
}
