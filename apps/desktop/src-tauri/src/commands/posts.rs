use ::tracing::{info, warn};
use kukuri_desktop_runtime::{
    BookmarkPostRequest, CreatePostRequest, CreateRepostRequest, GetBlobMediaRequest,
    GetBlobPreviewRequest, ListProfileTimelineRequest, ListThreadRequest, ListTimelineRequest,
    RemoveBookmarkedPostRequest, WithdrawPostRequest,
};

use crate::state::{CommandError, DesktopState, map_error};

#[tauri::command]
pub async fn create_post(
    state: tauri::State<'_, DesktopState>,
    request: CreatePostRequest,
) -> Result<String, CommandError> {
    state.runtime.create_post(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn withdraw_post(
    state: tauri::State<'_, DesktopState>,
    request: WithdrawPostRequest,
) -> Result<String, CommandError> {
    state.runtime.withdraw_post(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn create_repost(
    state: tauri::State<'_, DesktopState>,
    request: CreateRepostRequest,
) -> Result<String, CommandError> {
    state
        .runtime
        .create_repost(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn list_bookmarked_posts(
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<kukuri_app_api::BookmarkedPostView>, CommandError> {
    state
        .runtime
        .list_bookmarked_posts()
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn bookmark_post(
    state: tauri::State<'_, DesktopState>,
    request: BookmarkPostRequest,
) -> Result<kukuri_app_api::BookmarkedPostView, CommandError> {
    state
        .runtime
        .bookmark_post(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn remove_bookmarked_post(
    state: tauri::State<'_, DesktopState>,
    request: RemoveBookmarkedPostRequest,
) -> Result<(), CommandError> {
    state
        .runtime
        .remove_bookmarked_post(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn list_timeline(
    state: tauri::State<'_, DesktopState>,
    request: ListTimelineRequest,
) -> Result<kukuri_app_api::TimelineView, CommandError> {
    state
        .runtime
        .list_timeline(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn list_thread(
    state: tauri::State<'_, DesktopState>,
    request: ListThreadRequest,
) -> Result<kukuri_app_api::TimelineView, CommandError> {
    state.runtime.list_thread(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn list_profile_timeline(
    state: tauri::State<'_, DesktopState>,
    request: ListProfileTimelineRequest,
) -> Result<kukuri_app_api::TimelineView, CommandError> {
    state
        .runtime
        .list_profile_timeline(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn get_blob_preview_url(
    state: tauri::State<'_, DesktopState>,
    request: GetBlobPreviewRequest,
) -> Result<Option<String>, CommandError> {
    state
        .runtime
        .get_blob_preview_url(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn get_blob_media_payload(
    state: tauri::State<'_, DesktopState>,
    request: GetBlobMediaRequest,
) -> Result<Option<kukuri_app_api::BlobMediaPayload>, CommandError> {
    let hash = request.hash.clone();
    let mime = request.mime.clone();
    info!(hash = %hash, mime = %mime, "received get_blob_media_payload command");
    match state.runtime.get_blob_media_payload(request).await {
        Ok(Some(payload)) => {
            info!(
                hash = %hash,
                mime = %mime,
                bytes_base64_len = payload.bytes_base64.len(),
                "returning get_blob_media_payload response"
            );
            Ok(Some(payload))
        }
        Ok(None) => {
            warn!(hash = %hash, mime = %mime, "get_blob_media_payload returned no blob");
            Ok(None)
        }
        Err(error) => {
            let error = map_error(error);
            warn!(
                hash = %hash,
                mime = %mime,
                error = %error.message,
                "get_blob_media_payload command failed"
            );
            Err(error)
        }
    }
}
