use crate::{
    presentation::dto::{
        ApiResponse,
        direct_message_dto::{
            DirectMessagePage, ListDirectMessagesRequest, SendDirectMessageRequest,
            SendDirectMessageResponse,
        },
    },
    shared::{AppError, Result as AppResult},
    state::AppState,
};
use tauri::State;

async fn ensure_authenticated(state: &State<'_, AppState>) -> AppResult<String> {
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
    let _sender_pubkey = ensure_authenticated(&state).await?;

    let result: AppResult<SendDirectMessageResponse> = Err(AppError::NotImplemented(format!(
        "send_direct_message for recipient {} is not implemented yet",
        request.recipient_npub
    )));

    Ok(ApiResponse::from_result(result))
}

/// 会話履歴取得のスケルトン。
#[tauri::command]
pub async fn list_direct_messages(
    state: State<'_, AppState>,
    request: ListDirectMessagesRequest,
) -> Result<ApiResponse<DirectMessagePage>, AppError> {
    let _sender_pubkey = ensure_authenticated(&state).await?;

    let result: AppResult<DirectMessagePage> = Err(AppError::NotImplemented(format!(
        "list_direct_messages for conversation {} is not implemented yet",
        request.conversation_npub
    )));

    Ok(ApiResponse::from_result(result))
}
