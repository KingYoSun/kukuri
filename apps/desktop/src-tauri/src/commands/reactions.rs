use kukuri_desktop_runtime::{
    BookmarkCustomReactionRequest, CreateCustomReactionAssetRequest,
    ListRecentReactionsRequest, RemoveBookmarkedCustomReactionRequest, ToggleReactionRequest,
};

use crate::state::{DesktopState, map_error};

#[tauri::command]
pub async fn toggle_reaction(
    state: tauri::State<'_, DesktopState>,
    request: ToggleReactionRequest,
) -> Result<kukuri_app_api::ReactionStateView, String> {
    state.runtime.toggle_reaction(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn list_my_custom_reaction_assets(
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<kukuri_app_api::CustomReactionAssetView>, String> {
    state
        .runtime
        .list_my_custom_reaction_assets()
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn list_recent_reactions(
    state: tauri::State<'_, DesktopState>,
    request: ListRecentReactionsRequest,
) -> Result<Vec<kukuri_app_api::RecentReactionView>, String> {
    state.runtime.list_recent_reactions(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn create_custom_reaction_asset(
    state: tauri::State<'_, DesktopState>,
    request: CreateCustomReactionAssetRequest,
) -> Result<kukuri_app_api::CustomReactionAssetView, String> {
    state
        .runtime
        .create_custom_reaction_asset(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn list_bookmarked_custom_reactions(
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<kukuri_app_api::BookmarkedCustomReactionView>, String> {
    state
        .runtime
        .list_bookmarked_custom_reactions()
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn bookmark_custom_reaction(
    state: tauri::State<'_, DesktopState>,
    request: BookmarkCustomReactionRequest,
) -> Result<kukuri_app_api::BookmarkedCustomReactionView, String> {
    state
        .runtime
        .bookmark_custom_reaction(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn remove_bookmarked_custom_reaction(
    state: tauri::State<'_, DesktopState>,
    request: RemoveBookmarkedCustomReactionRequest,
) -> Result<(), String> {
    state
        .runtime
        .remove_bookmarked_custom_reaction(request)
        .await
        .map_err(map_error)
}
