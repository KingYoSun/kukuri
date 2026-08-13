use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use kukuri_desktop_runtime::{DesktopRuntime, StoreStartupError, resolve_db_path_from_env};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

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

/// 起動失敗の内部表現。分類済みの `kind` と表示用 `message` を持ち、`build_desktop_state`
/// の Err として src-tauri 内を流れる。`kind` は anyhow エラーの typed downcast で
/// 決まる(WP-Q2、従来の文字列 contains 判定を置換)。
#[derive(Debug)]
pub(crate) struct StartupError {
    pub(crate) kind: DesktopStartupErrorKind,
    pub(crate) message: String,
}

impl StartupError {
    /// 分類できない起動失敗(db パス解決失敗など)。
    fn unknown(message: String) -> Self {
        Self {
            kind: DesktopStartupErrorKind::Unknown,
            message,
        }
    }

    /// runtime 構築時の anyhow エラーを typed 分類して包む。
    fn from_runtime_error(error: anyhow::Error) -> Self {
        Self {
            kind: classify_startup_error(&error),
            message: error_message(error),
        }
    }
}

impl std::fmt::Display for StartupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
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

    pub(crate) fn failed(error: StartupError, db_path: Option<PathBuf>) -> Self {
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
        *self.status.lock().expect("startup status lock poisoned") = next;
    }
}

pub(crate) fn failed_status(error: StartupError, db_path: Option<PathBuf>) -> DesktopStartupStatus {
    DesktopStartupStatus::Failed {
        error: DesktopStartupErrorView {
            kind: error.kind,
            message: "kukuri could not open the local app database.".to_string(),
            detail: error.message,
            db_path: db_path.map(|path| path.display().to_string()),
        },
    }
}

/// Tauri コマンドの構造化エラー封筒(WP-C3)。invoke 側には
/// `{"code":"...","message":"..."}` の JSON で届く。message は従来の平文エラーと
/// 同一で、code は機械判定用(ドメイン別 code の拡充は Q6 の領分)。
#[derive(Clone, Debug, Serialize)]
pub(crate) struct CommandError {
    pub(crate) code: String,
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) retry_after_seconds: Option<u64>,
}

pub(crate) const COMMAND_FAILED_CODE: &str = "command_failed";

impl From<anyhow::Error> for CommandError {
    fn from(error: anyhow::Error) -> Self {
        Self {
            code: COMMAND_FAILED_CODE.to_string(),
            message: error_message(error),
            status: None,
            retry_after_seconds: None,
        }
    }
}

impl From<String> for CommandError {
    fn from(message: String) -> Self {
        Self {
            code: COMMAND_FAILED_CODE.to_string(),
            message,
            status: None,
            retry_after_seconds: None,
        }
    }
}

impl From<kukuri_desktop_runtime::CommunityNodeIndexQueryError> for CommandError {
    fn from(error: kukuri_desktop_runtime::CommunityNodeIndexQueryError) -> Self {
        Self {
            code: error.code,
            message: error.message,
            status: error.status,
            retry_after_seconds: error.retry_after_seconds,
        }
    }
}

impl From<kukuri_desktop_runtime::CommunityNodeIndexingRequestError> for CommandError {
    fn from(error: kukuri_desktop_runtime::CommunityNodeIndexingRequestError) -> Self {
        Self {
            code: error.code,
            message: error.message,
            status: error.status,
            retry_after_seconds: error.retry_after_seconds,
        }
    }
}

impl From<kukuri_desktop_runtime::CommunityNodeTrustRelationError> for CommandError {
    fn from(error: kukuri_desktop_runtime::CommunityNodeTrustRelationError) -> Self {
        Self {
            code: error.code,
            message: error.message,
            status: error.status,
            retry_after_seconds: None,
        }
    }
}

pub(crate) fn map_error(error: anyhow::Error) -> CommandError {
    CommandError::from(error)
}

fn error_message(error: anyhow::Error) -> String {
    format!("{error:#}")
}

pub(crate) fn resolve_db_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data dir: {error}"))?;
    resolve_db_path_from_env(&app_data_dir).map_err(error_message)
}

pub(crate) fn build_desktop_state(
    app_handle: &tauri::AppHandle,
) -> Result<DesktopState, StartupError> {
    let db_path = resolve_db_path(app_handle).map_err(StartupError::unknown)?;
    let runtime = tauri::async_runtime::block_on(DesktopRuntime::from_env(db_path))
        .map_err(StartupError::from_runtime_error)?;
    let runtime = Arc::new(runtime);
    // トレイ常駐中はフロントのポーリング(hidden で停止)が CN セッションを維持できないため、
    // runtime 常駐のセッション維持スケジューラをここで起動する(停止は shutdown / プロセス終了)。
    tauri::async_runtime::block_on(runtime.start_community_node_session_scheduler());

    spawn_runtime_event_bridge(app_handle, &runtime);
    tauri::async_runtime::block_on(runtime.start_sync_status_observer());

    Ok(DesktopState { runtime })
}

fn spawn_runtime_event_bridge(app_handle: &tauri::AppHandle, runtime: &Arc<DesktopRuntime>) {
    let mut rx = runtime.subscribe_events();
    let app = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let _ = app.emit("kukuri://runtime-event", &event);
        }
    });
}

fn app_consent_path(db_path: &Path) -> PathBuf {
    db_path.with_extension(APP_CONSENT_FILE_EXTENSION)
}

pub(crate) fn load_app_consent(db_path: &Path) -> Option<AppConsentRecord> {
    let bytes = std::fs::read(app_consent_path(db_path)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub(crate) fn save_app_consent(db_path: &Path, record: &AppConsentRecord) -> Result<(), String> {
    let path = app_consent_path(db_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create consent dir: {error}"))?;
    }
    let bytes = serde_json::to_vec_pretty(record)
        .map_err(|error| format!("failed to encode consent record: {error}"))?;
    std::fs::write(&path, bytes).map_err(|error| {
        format!(
            "failed to write consent record `{}`: {error}",
            path.display()
        )
    })
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

/// 起動時の anyhow エラーを typed に分類する(WP-Q2)。store が返す
/// `StoreStartupError`(接続 / migration を区別)を downcast で判定する。
/// anyhow の downcast は `.context()` を跨いでチェーンを辿るため、途中で文脈が
/// 付与されても root の typed variant に到達する。store 由来でないエラーは Unknown。
fn classify_startup_error(error: &anyhow::Error) -> DesktopStartupErrorKind {
    match error.downcast_ref::<StoreStartupError>() {
        Some(StoreStartupError::Migration(_)) => DesktopStartupErrorKind::DatabaseMigration,
        Some(StoreStartupError::Open { .. }) => DesktopStartupErrorKind::DatabaseOpen,
        None => DesktopStartupErrorKind::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // IPC エラー封筒の wire 形状(WP-C3)。TS 側 normalizeInvokeError と対になる
    // 同一バイナリ内契約 — 形状を変える場合は両側同時に変更する。
    #[test]
    fn command_error_serializes_to_code_message_envelope() {
        let error = map_error(anyhow::anyhow!("outer").context("inner"));
        let json = serde_json::to_string(&error).expect("serialize command error");
        assert_eq!(
            json,
            r#"{"code":"command_failed","message":"inner: outer"}"#
        );
    }

    #[test]
    fn command_error_from_string_preserves_message() {
        let error = CommandError::from("failed to resolve app data dir: boom".to_string());
        assert_eq!(error.code, COMMAND_FAILED_CODE);
        assert_eq!(error.message, "failed to resolve app data dir: boom");
    }

    #[test]
    fn community_index_error_serializes_status_and_retry_after() {
        let error = CommandError::from(kukuri_desktop_runtime::CommunityNodeIndexQueryError {
            code: "RATE_LIMITED".to_string(),
            message: "try again later".to_string(),
            status: Some(429),
            retry_after_seconds: Some(17),
        });
        let json = serde_json::to_string(&error).expect("serialize index error");
        assert_eq!(
            json,
            r#"{"code":"RATE_LIMITED","message":"try again later","status":429,"retry_after_seconds":17}"#
        );
    }

    #[test]
    fn community_indexing_request_error_preserves_stable_conflict_code() {
        let error = CommandError::from(
            kukuri_desktop_runtime::CommunityNodeIndexingRequestError {
                code: "CHANNEL_SECRET_CONFLICT".to_string(),
                message: "channel capability conflicts with the existing registration".to_string(),
                status: Some(409),
                retry_after_seconds: None,
            },
        );
        let json = serde_json::to_string(&error).expect("serialize indexing request error");
        assert_eq!(
            json,
            r#"{"code":"CHANNEL_SECRET_CONFLICT","message":"channel capability conflicts with the existing registration","status":409}"#
        );
    }

    #[test]
    fn trust_relation_error_preserves_stable_unavailable_code() {
        let error = CommandError::from(
            kukuri_desktop_runtime::CommunityNodeTrustRelationError {
                code: "RELATION_NOT_FOUND".to_string(),
                message: "no relation observed for this pair".to_string(),
                status: Some(404),
            },
        );
        let json = serde_json::to_string(&error).expect("serialize trust relation error");
        assert_eq!(
            json,
            r#"{"code":"RELATION_NOT_FOUND","message":"no relation observed for this pair","status":404}"#
        );
    }

    #[test]
    fn consent_satisfied_requires_current_or_newer_version() {
        assert!(!consent_satisfied(None));
        assert!(!consent_satisfied(Some(LEGAL_BUNDLE_VERSION - 1)));
        assert!(consent_satisfied(Some(LEGAL_BUNDLE_VERSION)));
        assert!(consent_satisfied(Some(LEGAL_BUNDLE_VERSION + 1)));
    }

    #[test]
    fn app_consent_round_trips_through_disk() {
        let dir =
            std::env::temp_dir().join(format!("kukuri-consent-test-{}", current_unix_seconds()));
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

    // WP-Q2: store 由来でないエラーは、message に "migration"/"sqlite"/"database" を
    // 含んでいても Unknown に分類される(= 旧実装の文字列判定による誤分類が起きない)。
    // Open/Migration の typed 分類は store 側の connect_file_*_is_typed_as_* テストが担保。
    #[test]
    fn classify_startup_error_ignores_message_strings_for_non_store_errors() {
        for message in [
            "some unrelated database noise",
            "migration-like wording in an unrelated failure",
            "failed to connect sqlite database (but not a StoreStartupError)",
        ] {
            let error = anyhow::anyhow!(message);
            assert!(
                matches!(
                    classify_startup_error(&error),
                    DesktopStartupErrorKind::Unknown
                ),
                "non-store error must classify as Unknown regardless of message: {message}"
            );
        }
    }
}
