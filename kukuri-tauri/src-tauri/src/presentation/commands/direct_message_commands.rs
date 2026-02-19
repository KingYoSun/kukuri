use crate::{
    presentation::dto::{
        ApiResponse,
        direct_message_dto::{
            DirectMessageConversationListDto, DirectMessagePage,
            ListDirectMessageConversationsRequest, ListDirectMessagesRequest,
            MarkDirectMessageConversationReadRequest, SeedDirectMessageRequest,
            SeedDirectMessageResponse, SendDirectMessageRequest, SendDirectMessageResponse,
        },
    },
    presentation::handlers::DirectMessageHandler,
    shared::AppError,
    state::AppState,
};
use nostr_sdk::prelude::{FromBech32, Keys, SecretKey, ToBech32};
use tauri::State;
use tracing::warn;

async fn ensure_authenticated(
    state: &State<'_, AppState>,
    fallback_nsec: Option<&str>,
) -> Result<String, AppError> {
    if let Ok(pair) = state.key_manager.current_keypair().await {
        return Ok(pair.npub.clone());
    }

    if let Some(user) = state.auth_service.get_current_user().await?
        && let Ok(nsec) = state.key_manager.export_private_key(&user.npub).await
    {
        let restored = state.key_manager.import_private_key(&nsec).await?;
        return Ok(restored.npub);
    }

    if let Some(nsec) = fallback_nsec
        && let Ok(secret_key) = SecretKey::from_bech32(nsec)
    {
        let keys = Keys::new(secret_key);
        if let Err(err) = state.key_manager.import_private_key(nsec).await {
            warn!(
                error = %err,
                "Failed to import fallback nsec during authentication; continuing with derived npub"
            );
        }
        return keys
            .public_key()
            .to_bech32()
            .map_err(|err| AppError::Crypto(format!("Failed to convert npub: {err}")));
    }

    Err(AppError::Unauthorized(
        "Authentication required: failed to load key material".to_string(),
    ))
}

fn is_e2e_allowed() -> bool {
    cfg!(debug_assertions)
        || tauri::is_dev()
        || matches!(std::env::var("TAURI_ENV_DEBUG"), Ok(value) if value == "true")
}

#[tauri::command]
pub async fn send_direct_message(
    state: State<'_, AppState>,
    request: SendDirectMessageRequest,
) -> Result<ApiResponse<SendDirectMessageResponse>, AppError> {
    let owner_npub = ensure_authenticated(&state, None).await?;
    let handler = DirectMessageHandler::new(state.direct_message_service.clone());
    let result = handler.send_direct_message(&owner_npub, request).await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn list_direct_messages(
    state: State<'_, AppState>,
    request: ListDirectMessagesRequest,
) -> Result<ApiResponse<DirectMessagePage>, AppError> {
    let owner_npub = ensure_authenticated(&state, None).await?;
    let handler = DirectMessageHandler::new(state.direct_message_service.clone());
    let result = handler.list_direct_messages(&owner_npub, request).await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn list_direct_message_conversations(
    state: State<'_, AppState>,
    request: ListDirectMessageConversationsRequest,
) -> Result<ApiResponse<DirectMessageConversationListDto>, AppError> {
    let owner_npub = ensure_authenticated(&state, None).await?;
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
    let owner_npub = ensure_authenticated(&state, None).await?;
    let handler = DirectMessageHandler::new(state.direct_message_service.clone());
    let result = handler
        .mark_conversation_as_read(&owner_npub, request)
        .await
        .map(|_| ());
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn seed_direct_message_for_e2e(
    state: State<'_, AppState>,
    request: SeedDirectMessageRequest,
) -> Result<ApiResponse<SeedDirectMessageResponse>, AppError> {
    if !is_e2e_allowed() {
        return Err(AppError::Unauthorized(
            "E2E direct message seeding is disabled".to_string(),
        ));
    }

    let owner_nsec = request.recipient_nsec.as_deref();
    let owner_npub = ensure_authenticated(&state, owner_nsec).await?;
    let content = request
        .content
        .unwrap_or_else(|| "Seeded direct message for E2E".to_string());
    let created_at = request
        .created_at
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let message = state
        .direct_message_service
        .seed_incoming_message_for_e2e(&owner_npub, &content, Some(created_at), owner_nsec)
        .await?;

    let response = SeedDirectMessageResponse {
        conversation_npub: message.conversation_npub.clone(),
        created_at: message.created_at_millis(),
        content: message
            .decrypted_content
            .clone()
            .unwrap_or_else(String::new),
    };

    Ok(ApiResponse::from_result(Ok(response)))
}
