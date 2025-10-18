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

/// トピックを作成する
#[tauri::command]
pub async fn create_topic(
    state: State<'_, AppState>,
    request: CreateTopicRequest,
) -> Result<ApiResponse<TopicResponse>, AppError> {
    ensure_authenticated(&state).await?;
    let handler = TopicHandler::new(state.topic_service.clone());
    let result = handler.create_topic(request).await;
    Ok(ApiResponse::from_result(result))
}

/// 単一のトピックを取得する
#[tauri::command]
pub async fn get_topic(
    state: State<'_, AppState>,
    id: String,
) -> Result<ApiResponse<Option<TopicResponse>>, AppError> {
    let handler = TopicHandler::new(state.topic_service.clone());
    let result = handler.get_topic(&id).await;
    Ok(ApiResponse::from_result(result))
}

/// すべてのトピックを取得する
#[tauri::command]
pub async fn get_topics(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<TopicResponse>>, AppError> {
    let handler = TopicHandler::new(state.topic_service.clone());
    let result = handler.get_all_topics().await;
    Ok(ApiResponse::from_result(result))
}

/// 参加中のトピックを取得する
#[tauri::command]
pub async fn get_joined_topics(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<TopicResponse>>, AppError> {
    let user_pubkey = ensure_authenticated(&state).await?;
    let handler = TopicHandler::new(state.topic_service.clone());
    let result = handler.get_joined_topics(&user_pubkey).await;
    Ok(ApiResponse::from_result(result))
}

/// トピックを更新する
#[tauri::command]
pub async fn update_topic(
    state: State<'_, AppState>,
    request: UpdateTopicRequest,
) -> Result<ApiResponse<TopicResponse>, AppError> {
    ensure_authenticated(&state).await?;
    let handler = TopicHandler::new(state.topic_service.clone());
    let result = handler.update_topic(request).await;
    Ok(ApiResponse::from_result(result))
}

/// トピックを削除する
#[tauri::command]
pub async fn delete_topic(
    state: State<'_, AppState>,
    request: DeleteTopicRequest,
) -> Result<ApiResponse<()>, AppError> {
    ensure_authenticated(&state).await?;
    let handler = TopicHandler::new(state.topic_service.clone());
    let result = handler.delete_topic(request).await;
    Ok(ApiResponse::from_result(result))
}

/// トピックに参加する
#[tauri::command]
pub async fn join_topic(
    state: State<'_, AppState>,
    request: JoinTopicRequest,
) -> Result<ApiResponse<()>, AppError> {
    let user_pubkey = ensure_authenticated(&state).await?;
    let handler = TopicHandler::new(state.topic_service.clone());
    let result = handler.join_topic(request, &user_pubkey).await;
    Ok(ApiResponse::from_result(result))
}

/// トピックから離脱する
#[tauri::command]
pub async fn leave_topic(
    state: State<'_, AppState>,
    request: JoinTopicRequest,
) -> Result<ApiResponse<()>, AppError> {
    let user_pubkey = ensure_authenticated(&state).await?;
    let handler = TopicHandler::new(state.topic_service.clone());
    let result = handler.leave_topic(request, &user_pubkey).await;
    Ok(ApiResponse::from_result(result))
}

/// トピックの統計情報を取得する
#[tauri::command]
pub async fn get_topic_stats(
    state: State<'_, AppState>,
    request: GetTopicStatsRequest,
) -> Result<ApiResponse<TopicStatsResponse>, AppError> {
    let handler = TopicHandler::new(state.topic_service.clone());
    let result = handler.get_topic_stats(request).await;
    Ok(ApiResponse::from_result(result))
}
