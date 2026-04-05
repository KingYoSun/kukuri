use kukuri_desktop_runtime::{
    DeleteDirectMessageMessageRequest, DirectMessageRequest, ListDirectMessageMessagesRequest,
    SendDirectMessageRequest,
};

use crate::state::{DesktopState, map_error};

#[tauri::command]
pub async fn open_direct_message(
    state: tauri::State<'_, DesktopState>,
    request: DirectMessageRequest,
) -> Result<kukuri_app_api::DirectMessageConversationView, String> {
    state.runtime.open_direct_message(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn list_direct_messages(
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<kukuri_app_api::DirectMessageConversationView>, String> {
    state.runtime.list_direct_messages().await.map_err(map_error)
}

#[tauri::command]
pub async fn list_direct_message_messages(
    state: tauri::State<'_, DesktopState>,
    request: ListDirectMessageMessagesRequest,
) -> Result<kukuri_app_api::DirectMessageTimelineView, String> {
    state
        .runtime
        .list_direct_message_messages(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn send_direct_message(
    state: tauri::State<'_, DesktopState>,
    request: SendDirectMessageRequest,
) -> Result<String, String> {
    state.runtime.send_direct_message(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn delete_direct_message_message(
    state: tauri::State<'_, DesktopState>,
    request: DeleteDirectMessageMessageRequest,
) -> Result<(), String> {
    state
        .runtime
        .delete_direct_message_message(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn clear_direct_message(
    state: tauri::State<'_, DesktopState>,
    request: DirectMessageRequest,
) -> Result<(), String> {
    state.runtime.clear_direct_message(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn get_direct_message_status(
    state: tauri::State<'_, DesktopState>,
    request: DirectMessageRequest,
) -> Result<kukuri_app_api::DirectMessageStatusView, String> {
    state.runtime.get_direct_message_status(request).await.map_err(map_error)
}
