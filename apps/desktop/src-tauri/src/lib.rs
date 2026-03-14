use std::sync::Arc;

use kukuri_desktop_runtime::{
    CreatePostRequest, DesktopRuntime, GetBlobMediaRequest, GetBlobPreviewRequest,
    ImportPeerTicketRequest, ListThreadRequest, ListTimelineRequest, UnsubscribeTopicRequest,
    resolve_db_path_from_env,
};
use tauri::Manager;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

struct DesktopState {
    runtime: Arc<DesktopRuntime>,
}

fn map_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("warn,kukuri_desktop_tauri_lib=info,kukuri_app_api=info")
    });

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .try_init();
}

fn resolve_db_path(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data dir: {error}"))?;
    resolve_db_path_from_env(&app_data_dir).map_err(map_error)
}

#[tauri::command]
async fn create_post(
    state: tauri::State<'_, DesktopState>,
    request: CreatePostRequest,
) -> Result<String, String> {
    state.runtime.create_post(request).await.map_err(map_error)
}

#[tauri::command]
async fn list_timeline(
    state: tauri::State<'_, DesktopState>,
    request: ListTimelineRequest,
) -> Result<kukuri_app_api::TimelineView, String> {
    state
        .runtime
        .list_timeline(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn list_thread(
    state: tauri::State<'_, DesktopState>,
    request: ListThreadRequest,
) -> Result<kukuri_app_api::TimelineView, String> {
    state.runtime.list_thread(request).await.map_err(map_error)
}

#[tauri::command]
async fn get_sync_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<kukuri_app_api::SyncStatus, String> {
    state.runtime.get_sync_status().await.map_err(map_error)
}

#[tauri::command]
async fn import_peer_ticket(
    state: tauri::State<'_, DesktopState>,
    request: ImportPeerTicketRequest,
) -> Result<(), String> {
    state
        .runtime
        .import_peer_ticket(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn unsubscribe_topic(
    state: tauri::State<'_, DesktopState>,
    request: UnsubscribeTopicRequest,
) -> Result<(), String> {
    state
        .runtime
        .unsubscribe_topic(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn get_local_peer_ticket(
    state: tauri::State<'_, DesktopState>,
) -> Result<Option<String>, String> {
    state.runtime.local_peer_ticket().await.map_err(map_error)
}

#[tauri::command]
async fn get_blob_preview_url(
    state: tauri::State<'_, DesktopState>,
    request: GetBlobPreviewRequest,
) -> Result<Option<String>, String> {
    state
        .runtime
        .get_blob_preview_url(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn get_blob_media_payload(
    state: tauri::State<'_, DesktopState>,
    request: GetBlobMediaRequest,
) -> Result<Option<kukuri_app_api::BlobMediaPayload>, String> {
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
            let error_message = map_error(error);
            warn!(
                hash = %hash,
                mime = %mime,
                error = %error_message,
                "get_blob_media_payload command failed"
            );
            Err(error_message)
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .setup(|app| {
            let db_path = resolve_db_path(app.handle())?;
            let runtime = tauri::async_runtime::block_on(DesktopRuntime::from_env(db_path))
                .map_err(map_error)?;
            app.manage(DesktopState {
                runtime: Arc::new(runtime),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_post,
            list_timeline,
            list_thread,
            get_sync_status,
            import_peer_ticket,
            unsubscribe_topic,
            get_local_peer_ticket,
            get_blob_media_payload,
            get_blob_preview_url
        ])
        .run(tauri::generate_context!())
        .expect("failed to run kukuri desktop tauri app");
}
