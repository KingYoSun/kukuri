use crate::{
    presentation::dto::{
        ApiResponse,
        direct_message_dto::{
            DirectMessageConversationListDto, DirectMessagePage,
            ListDirectMessageConversationsRequest, ListDirectMessagesRequest,
            MarkDirectMessageConversationReadRequest, SendDirectMessageRequest,
            SendDirectMessageResponse,
        },
    },
    presentation::handlers::DirectMessageHandler,
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
        .map_err(|err| AppError::Unauthorized(format!("ログインが必要です: {err}")))
}

/// kind 4 メッセージ送信のスケルトン。
#[tauri::command]
pub async fn send_direct_message(
    state: State<'_, AppState>,
    request: SendDirectMessageRequest,
) -> Result<ApiResponse<SendDirectMessageResponse>, AppError> {
    let owner_npub = ensure_authenticated(&state).await?;
    let handler = DirectMessageHandler::new(state.direct_message_service.clone());
    let result = handler.send_direct_message(&owner_npub, request).await;
    Ok(ApiResponse::from_result(result))
}

/// 会話履歴取得のスケルトン。
#[tauri::command]
pub async fn list_direct_messages(
    state: State<'_, AppState>,
    request: ListDirectMessagesRequest,
) -> Result<ApiResponse<DirectMessagePage>, AppError> {
    let owner_npub = ensure_authenticated(&state).await?;
    let handler = DirectMessageHandler::new(state.direct_message_service.clone());
    let result = handler.list_direct_messages(&owner_npub, request).await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn list_direct_message_conversations(
    state: State<'_, AppState>,
    request: ListDirectMessageConversationsRequest,
) -> Result<ApiResponse<DirectMessageConversationListDto>, AppError> {
    let owner_npub = ensure_authenticated(&state).await?;
    let handler = DirectMessageHandler::new(state.direct_message_service.clone());
    let result = handler
        .list_direct_message_conversations(&owner_npub, request)
        .await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn mark_direct_message_conversation_read(
    state: State<'_, AppState>,
    request: MarkDirectMessageConversationReadRequest,
) -> Result<ApiResponse<()>, AppError> {
    let owner_npub = ensure_authenticated(&state).await?;
    let handler = DirectMessageHandler::new(state.direct_message_service.clone());
    let result = handler
        .mark_conversation_as_read(&owner_npub, request)
        .await
        .map(|_| ());
    Ok(ApiResponse::from_result(result))
}
