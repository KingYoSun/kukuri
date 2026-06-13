use std::{path::PathBuf, sync::Arc};

use kukuri_desktop_runtime::{DesktopRuntime, resolve_db_path_from_env};
use serde::Serialize;
use tauri::Manager;

pub(crate) struct DesktopState {
    pub(crate) runtime: Arc<DesktopRuntime>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum DesktopStartupStatus {
    Ready,
    Failed {
        error: DesktopStartupErrorView,
    },
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DesktopStartupErrorView {
    pub(crate) kind: DesktopStartupErrorKind,
    pub(crate) message: String,
    pub(crate) detail: String,
    pub(crate) db_path: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DesktopStartupErrorKind {
    DatabaseOpen,
    DatabaseMigration,
    Unknown,
}

pub(crate) struct DesktopStartupState {
    status: DesktopStartupStatus,
}

impl DesktopStartupState {
    pub(crate) fn ready() -> Self {
        Self {
            status: DesktopStartupStatus::Ready,
        }
    }

    pub(crate) fn failed(error: String, db_path: Option<PathBuf>) -> Self {
        Self {
            status: DesktopStartupStatus::Failed {
                error: DesktopStartupErrorView {
                    kind: classify_startup_error(&error),
                    message: "kukuri could not open the local app database.".to_string(),
                    detail: error,
                    db_path: db_path.map(|path| path.display().to_string()),
                },
            },
        }
    }

    pub(crate) fn status(&self) -> DesktopStartupStatus {
        self.status.clone()
    }
}

pub(crate) fn map_error(error: anyhow::Error) -> String {
    format!("{error:#}")
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

fn classify_startup_error(error: &str) -> DesktopStartupErrorKind {
    let normalized = error.to_lowercase();
    if normalized.contains("migration") || normalized.contains("_sqlx_migrations") {
        return DesktopStartupErrorKind::DatabaseMigration;
    }
    if normalized.contains("failed to connect sqlite database")
        || normalized.contains("sqlite")
        || normalized.contains("database")
    {
        return DesktopStartupErrorKind::DatabaseOpen;
    }
    DesktopStartupErrorKind::Unknown
}
