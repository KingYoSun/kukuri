use kukuri_desktop_runtime::{
    CreateGameRoomRequest, CreateLiveSessionRequest, ListGameRoomsRequest, ListLiveSessionsRequest,
    LiveSessionCommandRequest, UpdateGameRoomRequest,
};

use crate::state::{DesktopState, map_error};

#[tauri::command]
pub async fn list_live_sessions(
    state: tauri::State<'_, DesktopState>,
    request: ListLiveSessionsRequest,
) -> Result<Vec<kukuri_app_api::LiveSessionView>, String> {
    state.runtime.list_live_sessions(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn create_live_session(
    state: tauri::State<'_, DesktopState>,
    request: CreateLiveSessionRequest,
) -> Result<String, String> {
    state.runtime.create_live_session(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn end_live_session(
    state: tauri::State<'_, DesktopState>,
    request: LiveSessionCommandRequest,
) -> Result<(), String> {
    state.runtime.end_live_session(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn join_live_session(
    state: tauri::State<'_, DesktopState>,
    request: LiveSessionCommandRequest,
) -> Result<(), String> {
    state.runtime.join_live_session(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn leave_live_session(
    state: tauri::State<'_, DesktopState>,
    request: LiveSessionCommandRequest,
) -> Result<(), String> {
    state.runtime.leave_live_session(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn list_game_rooms(
    state: tauri::State<'_, DesktopState>,
    request: ListGameRoomsRequest,
) -> Result<Vec<kukuri_app_api::GameRoomView>, String> {
    state.runtime.list_game_rooms(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn create_game_room(
    state: tauri::State<'_, DesktopState>,
    request: CreateGameRoomRequest,
) -> Result<String, String> {
    state.runtime.create_game_room(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn update_game_room(
    state: tauri::State<'_, DesktopState>,
    request: UpdateGameRoomRequest,
) -> Result<(), String> {
    state.runtime.update_game_room(request).await.map_err(map_error)
}
