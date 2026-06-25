use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use kukuri_desktop_runtime::{DesktopRuntime, resolve_db_path_from_env};
use serde::{Deserialize, Serialize};
use tauri::Manager;

pub(crate) struct DesktopState {
    pub(crate) runtime: Arc<DesktopRuntime>,
}

pub(crate) const LEGAL_BUNDLE_VERSION: i32 = 1;

const APP_CONSENT_FILE_EXTENSION: &str = "app-consent.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AppConsentRecord {
    pub(crate) accepted_bundle_version: i32,
    pub(crate) accepted_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum DesktopStartupStatus {
    Ready,
    ConsentRequired {
        current_bundle_version: i32,
        accepted_bundle_version: Option<i32>,
    },
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
    status: Mutex<DesktopStartupStatus>,
}

impl DesktopStartupState {
    pub(crate) fn ready() -> Self {
        Self {
            status: Mutex::new(DesktopStartupStatus::Ready),
        }
    }

    pub(crate) fn consent_required(
        current_bundle_version: i32,
        accepted_bundle_version: Option<i32>,
    ) -> Self {
        Self {
            status: Mutex::new(DesktopStartupStatus::ConsentRequired {
                current_bundle_version,
                accepted_bundle_version,
            }),
        }
    }

    pub(crate) fn failed(error: String, db_path: Option<PathBuf>) -> Self {
        Self {
            status: Mutex::new(failed_status(error, db_path)),
        }
    }

    pub(crate) fn status(&self) -> DesktopStartupStatus {
        self.status
            .lock()
            .expect("startup status lock poisoned")
            .clone()
    }

    pub(crate) fn set_status(&self, next: DesktopStartupStatus) {
        *self
            .status
            .lock()
            .expect("startup status lock poisoned") = next;
    }
}

pub(crate) fn failed_status(error: String, db_path: Option<PathBuf>) -> DesktopStartupStatus {
    DesktopStartupStatus::Failed {
        error: DesktopStartupErrorView {
            kind: classify_startup_error(&error),
            message: "kukuri could not open the local app database.".to_string(),
            detail: error,
            db_path: db_path.map(|path| path.display().to_string()),
        },
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

fn app_consent_path(db_path: &Path) -> PathBuf {
    db_path.with_extension(APP_CONSENT_FILE_EXTENSION)
}

pub(crate) fn load_app_consent(db_path: &Path) -> Option<AppConsentRecord> {
    let bytes = std::fs::read(app_consent_path(db_path)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub(crate) fn save_app_consent(
    db_path: &Path,
    record: &AppConsentRecord,
) -> Result<(), String> {
    let path = app_consent_path(db_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create consent dir: {error}"))?;
    }
    let bytes = serde_json::to_vec_pretty(record)
        .map_err(|error| format!("failed to encode consent record: {error}"))?;
    std::fs::write(&path, bytes)
        .map_err(|error| format!("failed to write consent record `{}`: {error}", path.display()))
}

pub(crate) fn consent_satisfied(accepted_bundle_version: Option<i32>) -> bool {
    accepted_bundle_version
        .map(|version| version >= LEGAL_BUNDLE_VERSION)
        .unwrap_or(false)
}

pub(crate) fn current_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consent_satisfied_requires_current_or_newer_version() {
        assert!(!consent_satisfied(None));
        assert!(!consent_satisfied(Some(LEGAL_BUNDLE_VERSION - 1)));
        assert!(consent_satisfied(Some(LEGAL_BUNDLE_VERSION)));
        assert!(consent_satisfied(Some(LEGAL_BUNDLE_VERSION + 1)));
    }

    #[test]
    fn app_consent_round_trips_through_disk() {
        let dir = std::env::temp_dir().join(format!("kukuri-consent-test-{}", current_unix_seconds()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let db_path = dir.join("kukuri.db");

        assert!(load_app_consent(&db_path).is_none());

        let record = AppConsentRecord {
            accepted_bundle_version: LEGAL_BUNDLE_VERSION,
            accepted_at: 1_700_000_000,
        };
        save_app_consent(&db_path, &record).expect("save consent");

        let loaded = load_app_consent(&db_path).expect("load consent");
        assert_eq!(loaded.accepted_bundle_version, LEGAL_BUNDLE_VERSION);
        assert_eq!(loaded.accepted_at, 1_700_000_000);
        assert!(consent_satisfied(Some(loaded.accepted_bundle_version)));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn startup_state_status_can_be_updated() {
        let state = DesktopStartupState::consent_required(LEGAL_BUNDLE_VERSION, None);
        assert!(matches!(
            state.status(),
            DesktopStartupStatus::ConsentRequired { .. }
        ));
        state.set_status(DesktopStartupStatus::Ready);
        assert!(matches!(state.status(), DesktopStartupStatus::Ready));
    }
}
