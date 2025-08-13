use crate::{
    presentation::{
        dto::{
            post_dto::{
                BatchBookmarkRequest, BatchGetPostsRequest, BatchReactRequest,
                BookmarkPostRequest, CreatePostRequest, DeletePostRequest, GetPostsRequest,
                PostResponse, ReactToPostRequest,
            },
            ApiResponse,
        },
        handlers::post_handler::PostHandler,
    },
    state::AppState,
};
use tauri::State;

/// 投稿を作成する
#[tauri::command]
pub async fn create_post_v2(
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
    
    // 処理を実行（ハンドラーは再利用）
    match state.post_handler.create_post(request).await {
        Ok(post) => Ok(ApiResponse::success(post)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// 投稿を取得する
#[tauri::command]
pub async fn get_posts_v2(
    state: State<'_, AppState>,
    request: GetPostsRequest,
) -> Result<ApiResponse<Vec<PostResponse>>, String> {
    match state.post_handler.get_posts(request).await {
        Ok(posts) => Ok(ApiResponse::success(posts)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// 投稿を削除する
#[tauri::command]
pub async fn delete_post_v2(
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
    
    match state.post_handler.delete_post(request).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// 投稿にリアクションする
#[tauri::command]
pub async fn react_to_post_v2(
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
    
    match state.post_handler.react_to_post(request).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// 投稿をブックマークする
#[tauri::command]
pub async fn bookmark_post_v2(
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

    match state.post_handler.bookmark_post(request, &user_pubkey).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// ブックマークを解除する
#[tauri::command]
pub async fn unbookmark_post_v2(
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

    match state.post_handler.unbookmark_post(request, &user_pubkey).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// 投稿にいいねする（旧APIとの互換性のため）
#[tauri::command]
pub async fn like_post_v2(
    state: State<'_, AppState>,
    post_id: String,
) -> Result<ApiResponse<()>, String> {
    let request = ReactToPostRequest {
        post_id,
        reaction: "+".to_string(),
    };
    
    react_to_post_v2(state, request).await
}

/// ブックマーク済み投稿IDを取得する
#[tauri::command]
pub async fn get_bookmarked_post_ids_v2(
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
    
    match state.post_handler.get_bookmarked_post_ids(&user_pubkey).await {
        Ok(post_ids) => Ok(ApiResponse::success(post_ids)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

// バッチ処理コマンド

/// 複数の投稿を一括取得する
#[tauri::command]
pub async fn batch_get_posts_v2(
    state: State<'_, AppState>,
    request: BatchGetPostsRequest,
) -> Result<ApiResponse<Vec<PostResponse>>, String> {
    match state.post_handler.batch_get_posts(request).await {
        Ok(posts) => Ok(ApiResponse::success(posts)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// 複数のリアクションを一括処理する
#[tauri::command]
pub async fn batch_react_v2(
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
    
    match state.post_handler.batch_react(request).await {
        Ok(results) => Ok(ApiResponse::success(results)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// 複数のブックマークを一括処理する
#[tauri::command]
pub async fn batch_bookmark_v2(
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
    
    match state.post_handler.batch_bookmark(request, &user_pubkey).await {
        Ok(results) => Ok(ApiResponse::success(results)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// 投稿をブーストする（旧APIとの互換性のため）
#[tauri::command]
pub async fn boost_post_v2(
    state: State<'_, AppState>,
    post_id: String,
) -> Result<ApiResponse<()>, String> {
    let request = ReactToPostRequest {
        post_id,
        reaction: "boost".to_string(),
    };
    
    react_to_post_v2(state, request).await
}