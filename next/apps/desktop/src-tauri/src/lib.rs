use std::path::PathBuf;
use std::sync::Arc;

use next_desktop_runtime::{
    CreatePostRequest, DesktopRuntime, ImportPeerTicketRequest, ListThreadRequest,
    ListTimelineRequest,
};
use tauri::Manager;

struct DesktopState {
    runtime: Arc<DesktopRuntime>,
}

fn map_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn resolve_db_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    if let Some(explicit_dir) = std::env::var("KUKURI_NEXT_APP_DATA_DIR")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        let path = PathBuf::from(explicit_dir);
        std::fs::create_dir_all(&path)
            .map_err(|error| format!("failed to create explicit app data dir: {error}"))?;
        return Ok(path.join("kukuri-next.db"));
    }

    let mut app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data dir: {error}"))?;
    if let Some(instance) = std::env::var("KUKURI_NEXT_INSTANCE")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        app_data_dir = app_data_dir.join(instance.trim());
    }
    std::fs::create_dir_all(&app_data_dir)
        .map_err(|error| format!("failed to create app data dir: {error}"))?;
    Ok(app_data_dir.join("kukuri-next.db"))
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
) -> Result<next_app_api::TimelineView, String> {
    state.runtime.list_timeline(request).await.map_err(map_error)
}

#[tauri::command]
async fn list_thread(
    state: tauri::State<'_, DesktopState>,
    request: ListThreadRequest,
) -> Result<next_app_api::TimelineView, String> {
    state.runtime.list_thread(request).await.map_err(map_error)
}

#[tauri::command]
async fn get_sync_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<next_app_api::SyncStatus, String> {
    state.runtime.get_sync_status().await.map_err(map_error)
}

#[tauri::command]
async fn import_peer_ticket(
    state: tauri::State<'_, DesktopState>,
    request: ImportPeerTicketRequest,
) -> Result<(), String> {
    state.runtime.import_peer_ticket(request).await.map_err(map_error)
}

#[tauri::command]
async fn get_local_peer_ticket(
    state: tauri::State<'_, DesktopState>,
) -> Result<Option<String>, String> {
    state.runtime.local_peer_ticket().await.map_err(map_error)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
            get_local_peer_ticket
        ])
        .run(tauri::generate_context!())
        .expect("failed to run next desktop tauri app");
}
