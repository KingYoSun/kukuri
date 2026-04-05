use std::{path::PathBuf, sync::Arc};

use kukuri_desktop_runtime::{DesktopRuntime, resolve_db_path_from_env};
use tauri::Manager;

pub(crate) struct DesktopState {
    pub(crate) runtime: Arc<DesktopRuntime>,
}

pub(crate) fn map_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

pub(crate) fn resolve_db_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data dir: {error}"))?;
    resolve_db_path_from_env(&app_data_dir).map_err(map_error)
}

pub(crate) fn build_desktop_state(app_handle: &tauri::AppHandle) -> Result<DesktopState, String> {
    let db_path = resolve_db_path(app_handle)?;
    let runtime = tauri::async_runtime::block_on(DesktopRuntime::from_env(db_path))
        .map_err(map_error)?;

    Ok(DesktopState {
        runtime: Arc::new(runtime),
    })
}
