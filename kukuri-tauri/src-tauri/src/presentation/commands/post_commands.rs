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
    state::AppState,
};
use tauri::State;

/// 投稿を作成する
#[tauri::command]
pub async fn create_post(
    state: State<'_, AppState>,
    request: CreatePostRequest,
) -> Result<ApiResponse<PostResponse>, String> {
    // 認証チェック
    let user_pubkey = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {}", e))?
        .public_key()
        .to_hex();

    // PostHandlerを使用
    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    match handler.create_post(request).await {
        Ok(post) => Ok(ApiResponse::success(post)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// 投稿を取得する
#[tauri::command]
pub async fn get_posts(
    state: State<'_, AppState>,
    request: GetPostsRequest,
) -> Result<ApiResponse<Vec<PostResponse>>, String> {
    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    match handler.get_posts(request).await {
        Ok(posts) => Ok(ApiResponse::success(posts)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// 投稿を削除する
#[tauri::command]
pub async fn delete_post(
    state: State<'_, AppState>,
    request: DeletePostRequest,
) -> Result<ApiResponse<()>, String> {
    // 認証チェック
    let _user_pubkey = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {}", e))?
        .public_key()
        .to_hex();

    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    match handler.delete_post(request).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// 投稿にリアクションする
#[tauri::command]
pub async fn react_to_post(
    state: State<'_, AppState>,
    request: ReactToPostRequest,
) -> Result<ApiResponse<()>, String> {
    // 認証チェック
    let _user_pubkey = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {}", e))?
        .public_key()
        .to_hex();

    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    match handler.react_to_post(request).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// 投稿をブックマークする
#[tauri::command]
pub async fn bookmark_post(
    state: State<'_, AppState>,
    request: BookmarkPostRequest,
) -> Result<ApiResponse<()>, String> {
    // 認証チェック
    let user_pubkey = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {}", e))?
        .public_key()
        .to_hex();

    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    match handler.bookmark_post(request, &user_pubkey).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// ブックマークを解除する
#[tauri::command]
pub async fn unbookmark_post(
    state: State<'_, AppState>,
    request: BookmarkPostRequest,
) -> Result<ApiResponse<()>, String> {
    // 認証チェック
    let user_pubkey = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {}", e))?
        .public_key()
        .to_hex();

    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    match handler.unbookmark_post(request, &user_pubkey).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// 投稿にいいねする（旧APIとの互換性のため）
#[tauri::command]
pub async fn like_post(
    state: State<'_, AppState>,
    post_id: String,
) -> Result<ApiResponse<()>, String> {
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
) -> Result<ApiResponse<Vec<String>>, String> {
    // 認証チェック
    let user_pubkey = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {}", e))?
        .public_key()
        .to_hex();

    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    match handler.get_bookmarked_post_ids(&user_pubkey).await {
        Ok(post_ids) => Ok(ApiResponse::success(post_ids)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

// バッチ処理コマンド

/// 複数の投稿を一括取得する
#[tauri::command]
pub async fn batch_get_posts(
    state: State<'_, AppState>,
    request: BatchGetPostsRequest,
) -> Result<ApiResponse<Vec<PostResponse>>, String> {
    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    match handler.batch_get_posts(request).await {
        Ok(posts) => Ok(ApiResponse::success(posts)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// 複数のリアクションを一括処理する
#[tauri::command]
pub async fn batch_react(
    state: State<'_, AppState>,
    request: BatchReactRequest,
) -> Result<ApiResponse<Vec<Result<(), String>>>, String> {
    // 認証チェック
    let _user_pubkey = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {}", e))?
        .public_key()
        .to_hex();

    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    match handler.batch_react(request).await {
        Ok(results) => Ok(ApiResponse::success(results)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// 複数のブックマークを一括処理する
#[tauri::command]
pub async fn batch_bookmark(
    state: State<'_, AppState>,
    request: BatchBookmarkRequest,
) -> Result<ApiResponse<Vec<Result<(), String>>>, String> {
    // 認証チェック
    let user_pubkey = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {}", e))?
        .public_key()
        .to_hex();

    let handler = PostHandler::new(state.post_service.clone(), state.auth_service.clone());
    match handler.batch_bookmark(request, &user_pubkey).await {
        Ok(results) => Ok(ApiResponse::success(results)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// 投稿をブーストする（旧APIとの互換性のため）
#[tauri::command]
pub async fn boost_post(
    state: State<'_, AppState>,
    post_id: String,
) -> Result<ApiResponse<()>, String> {
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
) -> Result<Option<serde_json::Value>, String> {
    match state.post_service.get_post(&id).await {
        Ok(post) => Ok(post.map(|p| serde_json::to_value(p).unwrap())),
        Err(e) => Err(e.to_string()),
    }
}

/// トピック別の投稿を取得する（旧APIとの互換性のため）
#[tauri::command]
pub async fn get_posts_by_topic(
    state: State<'_, AppState>,
    topic_id: String,
    limit: Option<usize>,
) -> Result<Vec<serde_json::Value>, String> {
    match state
        .post_service
        .get_posts_by_topic(&topic_id, limit.unwrap_or(50))
        .await
    {
        Ok(posts) => Ok(posts
            .into_iter()
            .map(|p| serde_json::to_value(p).unwrap())
            .collect()),
        Err(e) => Err(e.to_string()),
    }
}

/// 保留中の投稿を同期する
#[tauri::command]
pub async fn sync_posts(state: State<'_, AppState>) -> Result<u32, String> {
    state
        .post_service
        .sync_pending_posts()
        .await
        .map_err(|e| e.to_string())
}
