use crate::{
    presentation::{
        dto::{
            ApiResponse,
            topic_dto::{
                CreateTopicRequest, DeleteTopicRequest, GetTopicStatsRequest, JoinTopicRequest,
                TopicResponse, TopicStatsResponse, UpdateTopicRequest,
            },
        },
        handlers::TopicHandler,
    },
    state::AppState,
};
use tauri::State;

/// トピックを作成する
#[tauri::command]
pub async fn create_topic(
    state: State<'_, AppState>,
    request: CreateTopicRequest,
) -> Result<ApiResponse<TopicResponse>, String> {
    // 認証チェック
    let _user_pubkey = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {}", e))?
        .public_key()
        .to_hex();

    let handler = TopicHandler::new(state.topic_service.clone());
    match handler.create_topic(request).await {
        Ok(topic) => Ok(ApiResponse::success(topic)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// 単一のトピックを取得する
#[tauri::command]
pub async fn get_topic(
    state: State<'_, AppState>,
    id: String,
) -> Result<ApiResponse<Option<TopicResponse>>, String> {
    let handler = TopicHandler::new(state.topic_service.clone());
    match handler.get_topic(&id).await {
        Ok(topic) => Ok(ApiResponse::success(topic)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// すべてのトピックを取得する
#[tauri::command]
pub async fn get_topics(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<TopicResponse>>, String> {
    let handler = TopicHandler::new(state.topic_service.clone());
    match handler.get_all_topics().await {
        Ok(topics) => Ok(ApiResponse::success(topics)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// 参加中のトピックを取得する
#[tauri::command]
pub async fn get_joined_topics(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<TopicResponse>>, String> {
    // 認証チェック
    let _user_pubkey = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {}", e))?
        .public_key()
        .to_hex();

    let handler = TopicHandler::new(state.topic_service.clone());
    match handler.get_joined_topics().await {
        Ok(topics) => Ok(ApiResponse::success(topics)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// トピックを更新する
#[tauri::command]
pub async fn update_topic(
    state: State<'_, AppState>,
    request: UpdateTopicRequest,
) -> Result<ApiResponse<TopicResponse>, String> {
    // 認証チェック
    let _user_pubkey = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {}", e))?
        .public_key()
        .to_hex();

    // TODO: TopicHandler::update_topicメソッドの実装が必要
    Ok(ApiResponse::error("Not implemented yet".to_string()))
}

/// トピックを削除する
#[tauri::command]
pub async fn delete_topic(
    state: State<'_, AppState>,
    request: DeleteTopicRequest,
) -> Result<ApiResponse<()>, String> {
    // 認証チェック
    let _user_pubkey = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {}", e))?
        .public_key()
        .to_hex();

    // TODO: TopicHandler::delete_topicメソッドの実装が必要
    Ok(ApiResponse::error("Not implemented yet".to_string()))
}

/// トピックに参加する
#[tauri::command]
pub async fn join_topic(
    state: State<'_, AppState>,
    request: JoinTopicRequest,
) -> Result<ApiResponse<()>, String> {
    // 認証チェック
    let user_pubkey = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {}", e))?
        .public_key()
        .to_hex();

    let handler = TopicHandler::new(state.topic_service.clone());
    match handler.join_topic(request, &user_pubkey).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// トピックから離脱する
#[tauri::command]
pub async fn leave_topic(
    state: State<'_, AppState>,
    request: JoinTopicRequest,
) -> Result<ApiResponse<()>, String> {
    // 認証チェック
    let user_pubkey = state
        .key_manager
        .get_keys()
        .await
        .map_err(|e| format!("ログインが必要です: {}", e))?
        .public_key()
        .to_hex();

    let handler = TopicHandler::new(state.topic_service.clone());
    match handler.leave_topic(request, &user_pubkey).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

/// トピックの統計情報を取得する
#[tauri::command]
pub async fn get_topic_stats(
    state: State<'_, AppState>,
    request: GetTopicStatsRequest,
) -> Result<ApiResponse<TopicStatsResponse>, String> {
    let handler = TopicHandler::new(state.topic_service.clone());
    match handler.get_topic_stats(request).await {
        Ok(stats) => Ok(ApiResponse::success(stats)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}
