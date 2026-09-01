use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use kukuri_desktop_runtime::{DesktopRuntime, StoreStartupError, resolve_db_path_from_env};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use tokio::sync::watch;

pub(crate) struct DesktopState {
    pub(crate) runtime: Arc<DesktopRuntime>,
}

pub(crate) const LEGAL_BUNDLE_VERSION: i32 = 3;

/// 18歳以上の自己申告(#858、ADR 0046)の現行版。文書同意とは独立に管理し、
/// 申告文言の重要変更時のみ上げて再申告を求める。
pub(crate) const AGE_ATTESTATION_VERSION: i32 = 1;

/// アプリ同意の対象文書と現行版。同意は bundle 単一フラグではなく文書単位で
/// 記録する(#857)。現状は全文書が legal bundle 版と同じ版番号を共有しているが、
/// 記録・判定は slug ごとに独立している。
pub(crate) const APP_LEGAL_DOCUMENTS: &[(&str, i32)] = &[
    ("terms", LEGAL_BUNDLE_VERSION),
    ("privacy", LEGAL_BUNDLE_VERSION),
];

const APP_CONSENT_FILE_EXTENSION: &str = "app-consent.json";

/// 文書単位のアプリ同意記録(#857)。対象文書 slug・版・日時・同意時の表示言語・
/// アプリ版を保存する。旧形式(`accepted_bundle_version` の単一フラグ)は
/// 意図的に読み替えず、未同意として再同意を求める。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AppConsentDocumentRecord {
    pub(crate) slug: String,
    pub(crate) version: i32,
    pub(crate) accepted_at: i64,
    pub(crate) language: String,
    pub(crate) app_version: String,
}

/// 18歳以上の自己申告記録(#858)。文書同意とは別の行為として記録する。
/// 生年月日等は収集せず、申告の事実・版・日時・表示言語・アプリ版のみ保存する。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AgeAttestationRecord {
    pub(crate) version: i32,
    pub(crate) attested_at: i64,
    pub(crate) language: String,
    pub(crate) app_version: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct AppConsentStore {
    #[serde(default)]
    pub(crate) records: Vec<AppConsentDocumentRecord>,
    #[serde(default)]
    pub(crate) age_attestations: Vec<AgeAttestationRecord>,
}

/// 文書単位の同意状態ビュー。startup gate と `get_app_consent_status` の両方で使う。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppConsentDocumentStatus {
    pub(crate) slug: String,
    pub(crate) current_version: i32,
    pub(crate) accepted_version: Option<i32>,
    pub(crate) accepted_at: Option<i64>,
    pub(crate) accepted_language: Option<String>,
    pub(crate) accepted_app_version: Option<String>,
}

/// 年齢自己申告の状態ビュー。文書同意とは別枠で startup gate と
/// `get_app_consent_status` に載せる。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgeAttestationStatus {
    pub(crate) current_version: i32,
    pub(crate) attested_version: Option<i32>,
    pub(crate) attested_at: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum DesktopStartupStatus {
    Initializing,
    Ready,
    ConsentRequired {
        documents: Vec<AppConsentDocumentStatus>,
        age_attestation: AgeAttestationStatus,
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
    status: watch::Sender<DesktopStartupStatus>,
}

impl DesktopStartupState {
    pub(crate) fn initializing() -> Self {
        Self::new(DesktopStartupStatus::Initializing)
    }

    pub(crate) fn consent_required(
        documents: Vec<AppConsentDocumentStatus>,
        age_attestation: AgeAttestationStatus,
    ) -> Self {
        Self::new(DesktopStartupStatus::ConsentRequired {
            documents,
            age_attestation,
        })
    }

    fn new(status: DesktopStartupStatus) -> Self {
        let (status, _) = watch::channel(status);
        Self { status }
    }

    pub(crate) fn status(&self) -> DesktopStartupStatus {
        self.status.borrow().clone()
    }

    pub(crate) fn set_status(&self, next: DesktopStartupStatus) {
        self.status.send_replace(next);
    }

    pub(crate) fn subscribe(&self) -> watch::Receiver<DesktopStartupStatus> {
        self.status.subscribe()
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

impl From<kukuri_desktop_runtime::CommunityNodeTesterFeedbackError> for CommandError {
    fn from(error: kukuri_desktop_runtime::CommunityNodeTesterFeedbackError) -> Self {
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

impl From<kukuri_desktop_runtime::CommunityNodeReportError> for CommandError {
    fn from(error: kukuri_desktop_runtime::CommunityNodeReportError) -> Self {
        Self {
            code: error.code,
            message: error.message,
            status: error.status,
            retry_after_seconds: None,
        }
    }
}

pub(crate) fn map_error(error: anyhow::Error) -> CommandError {
    if let Some(rejection) = error.downcast_ref::<kukuri_core::MetaverseResourceRejection>() {
        return CommandError {
            code: rejection.code(),
            message: rejection.to_string(),
            status: Some(
                if rejection.reason == kukuri_core::MetaverseResourceRejectionReason::RateExceeded {
                    429
                } else {
                    422
                },
            ),
            retry_after_seconds: None,
        };
    }
    if let Some(request_error) =
        error.downcast_ref::<kukuri_desktop_runtime::DomeHostingRequestError>()
    {
        return CommandError {
            code: request_error.code.clone(),
            message: request_error.message.clone(),
            status: Some(request_error.status),
            retry_after_seconds: None,
        };
    }
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

pub(crate) async fn build_desktop_state(
    app_handle: &tauri::AppHandle,
) -> Result<DesktopState, StartupError> {
    let db_path = resolve_db_path(app_handle).map_err(StartupError::unknown)?;
    let runtime = DesktopRuntime::from_env(db_path)
        .await
        .map_err(StartupError::from_runtime_error)?;
    let runtime = Arc::new(runtime);
    // トレイ常駐中はフロントのポーリング(hidden で停止)が CN セッションを維持できないため、
    // runtime 常駐のセッション維持スケジューラをここで起動する(停止は shutdown / プロセス終了)。
    runtime.start_community_node_session_scheduler().await;

    spawn_runtime_event_bridge(app_handle, &runtime);
    runtime.start_sync_status_observer().await;

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

/// 同意記録ファイルを読む。ファイル欠落・破損・旧形式(bundle 単一フラグ)は
/// いずれも空の store(= 全文書未同意)として扱う。
pub(crate) fn load_app_consent_store(db_path: &Path) -> AppConsentStore {
    let Ok(bytes) = std::fs::read(app_consent_path(db_path)) else {
        return AppConsentStore::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

pub(crate) fn save_app_consent_store(
    db_path: &Path,
    store: &AppConsentStore,
) -> Result<(), String> {
    let path = app_consent_path(db_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create consent dir: {error}"))?;
    }
    let bytes = serde_json::to_vec_pretty(store)
        .map_err(|error| format!("failed to encode consent record: {error}"))?;
    std::fs::write(&path, bytes).map_err(|error| {
        format!(
            "failed to write consent record `{}`: {error}",
            path.display()
        )
    })
}

/// slug ごとの最新同意記録(版が最大のもの。同版なら日時が新しいもの)。
fn latest_app_consent_record<'a>(
    store: &'a AppConsentStore,
    slug: &str,
) -> Option<&'a AppConsentDocumentRecord> {
    store
        .records
        .iter()
        .filter(|record| record.slug == slug)
        .max_by_key(|record| (record.version, record.accepted_at))
}

pub(crate) fn app_consent_documents_status(
    store: &AppConsentStore,
) -> Vec<AppConsentDocumentStatus> {
    APP_LEGAL_DOCUMENTS
        .iter()
        .map(|(slug, current_version)| {
            let latest = latest_app_consent_record(store, slug);
            AppConsentDocumentStatus {
                slug: (*slug).to_string(),
                current_version: *current_version,
                accepted_version: latest.map(|record| record.version),
                accepted_at: latest.map(|record| record.accepted_at),
                accepted_language: latest.map(|record| record.language.clone()),
                accepted_app_version: latest.map(|record| record.app_version.clone()),
            }
        })
        .collect()
}

/// 全対象文書について現行版以上の同意記録があるか。
pub(crate) fn app_consent_documents_satisfied(store: &AppConsentStore) -> bool {
    APP_LEGAL_DOCUMENTS.iter().all(|(slug, current_version)| {
        latest_app_consent_record(store, slug)
            .map(|record| record.version >= *current_version)
            .unwrap_or(false)
    })
}

/// 現行版以上の年齢自己申告記録があるか(#858)。文書同意とは独立に判定する。
pub(crate) fn age_attestation_satisfied(store: &AppConsentStore) -> bool {
    latest_age_attestation_record(store)
        .map(|record| record.version >= AGE_ATTESTATION_VERSION)
        .unwrap_or(false)
}

fn latest_age_attestation_record(store: &AppConsentStore) -> Option<&AgeAttestationRecord> {
    store
        .age_attestations
        .iter()
        .max_by_key(|record| (record.version, record.attested_at))
}

pub(crate) fn age_attestation_status(store: &AppConsentStore) -> AgeAttestationStatus {
    let latest = latest_age_attestation_record(store);
    AgeAttestationStatus {
        current_version: AGE_ATTESTATION_VERSION,
        attested_version: latest.map(|record| record.version),
        attested_at: latest.map(|record| record.attested_at),
    }
}

/// 起動 gate の総合判定: 全文書同意 + 年齢自己申告が揃っているか。
pub(crate) fn app_consent_satisfied(store: &AppConsentStore) -> bool {
    app_consent_documents_satisfied(store) && age_attestation_satisfied(store)
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
        let error = CommandError::from(kukuri_desktop_runtime::CommunityNodeIndexingRequestError {
            code: "CHANNEL_SECRET_CONFLICT".to_string(),
            message: "channel capability conflicts with the existing registration".to_string(),
            status: Some(409),
            retry_after_seconds: None,
        });
        let json = serde_json::to_string(&error).expect("serialize indexing request error");
        assert_eq!(
            json,
            r#"{"code":"CHANNEL_SECRET_CONFLICT","message":"channel capability conflicts with the existing registration","status":409}"#
        );
    }

    #[test]
    fn trust_relation_error_preserves_stable_unavailable_code() {
        let error = CommandError::from(kukuri_desktop_runtime::CommunityNodeTrustRelationError {
            code: "RELATION_NOT_FOUND".to_string(),
            message: "no relation observed for this pair".to_string(),
            status: Some(404),
        });
        let json = serde_json::to_string(&error).expect("serialize trust relation error");
        assert_eq!(
            json,
            r#"{"code":"RELATION_NOT_FOUND","message":"no relation observed for this pair","status":404}"#
        );
    }

    fn record(slug: &str, version: i32, accepted_at: i64) -> AppConsentDocumentRecord {
        AppConsentDocumentRecord {
            slug: slug.to_string(),
            version,
            accepted_at,
            language: "ja".to_string(),
            app_version: "0.1.8".to_string(),
        }
    }

    fn attestation(version: i32, attested_at: i64) -> AgeAttestationRecord {
        AgeAttestationRecord {
            version,
            attested_at,
            language: "ja".to_string(),
            app_version: "0.1.8".to_string(),
        }
    }

    #[test]
    fn app_consent_satisfied_requires_every_document_at_current_or_newer_version() {
        assert_eq!(LEGAL_BUNDLE_VERSION, 3);
        assert!(!app_consent_documents_satisfied(&AppConsentStore::default()));

        // terms だけ同意しても不十分。
        let partial = AppConsentStore {
            records: vec![record("terms", LEGAL_BUNDLE_VERSION, 1)],
            age_attestations: Vec::new(),
        };
        assert!(!app_consent_documents_satisfied(&partial));

        // 旧版の記録は再同意が必要。
        let outdated = AppConsentStore {
            records: vec![record("terms", 1, 1), record("privacy", 1, 1)],
            age_attestations: Vec::new(),
        };
        assert!(!app_consent_documents_satisfied(&outdated));

        let satisfied = AppConsentStore {
            records: vec![
                record("terms", LEGAL_BUNDLE_VERSION, 1),
                record("privacy", LEGAL_BUNDLE_VERSION + 1, 1),
            ],
            age_attestations: Vec::new(),
        };
        assert!(app_consent_documents_satisfied(&satisfied));
    }

    // #858: 文書同意が揃っていても年齢自己申告が無ければ gate は開かない。
    // 逆に、申告だけあって文書同意が無い場合も開かない(別状態の独立判定)。
    #[test]
    fn app_consent_satisfied_requires_age_attestation_in_addition_to_documents() {
        let documents_only = AppConsentStore {
            records: vec![
                record("terms", LEGAL_BUNDLE_VERSION, 1),
                record("privacy", LEGAL_BUNDLE_VERSION, 1),
            ],
            age_attestations: Vec::new(),
        };
        assert!(app_consent_documents_satisfied(&documents_only));
        assert!(!age_attestation_satisfied(&documents_only));
        assert!(!app_consent_satisfied(&documents_only));

        let attestation_only = AppConsentStore {
            records: Vec::new(),
            age_attestations: vec![attestation(AGE_ATTESTATION_VERSION, 1)],
        };
        assert!(age_attestation_satisfied(&attestation_only));
        assert!(!app_consent_satisfied(&attestation_only));

        let both = AppConsentStore {
            records: documents_only.records.clone(),
            age_attestations: attestation_only.age_attestations.clone(),
        };
        assert!(app_consent_satisfied(&both));
    }

    #[test]
    fn age_attestation_status_reports_latest_record() {
        let empty_status = age_attestation_status(&AppConsentStore::default());
        assert_eq!(empty_status.current_version, AGE_ATTESTATION_VERSION);
        assert_eq!(empty_status.attested_version, None);
        assert_eq!(empty_status.attested_at, None);

        let store = AppConsentStore {
            records: Vec::new(),
            age_attestations: vec![
                attestation(AGE_ATTESTATION_VERSION, 100),
                attestation(AGE_ATTESTATION_VERSION, 200),
            ],
        };
        let status = age_attestation_status(&store);
        assert_eq!(status.attested_version, Some(AGE_ATTESTATION_VERSION));
        assert_eq!(status.attested_at, Some(200));
    }

    #[test]
    fn app_consent_status_reports_latest_record_per_document() {
        let store = AppConsentStore {
            records: vec![
                record("terms", 1, 100),
                record("terms", LEGAL_BUNDLE_VERSION, 200),
                record("privacy", 1, 150),
            ],
        };
        let status = app_consent_documents_status(&store);
        assert_eq!(status.len(), APP_LEGAL_DOCUMENTS.len());

        let terms = status.iter().find(|doc| doc.slug == "terms").unwrap();
        assert_eq!(terms.current_version, LEGAL_BUNDLE_VERSION);
        assert_eq!(terms.accepted_version, Some(LEGAL_BUNDLE_VERSION));
        assert_eq!(terms.accepted_at, Some(200));
        assert_eq!(terms.accepted_language.as_deref(), Some("ja"));
        assert_eq!(terms.accepted_app_version.as_deref(), Some("0.1.8"));

        let privacy = status.iter().find(|doc| doc.slug == "privacy").unwrap();
        assert_eq!(privacy.accepted_version, Some(1));
    }

    // #857: 旧形式(bundle 単一フラグ)の記録は読み替えず、全文書未同意として
    // 再同意を求める。
    #[test]
    fn legacy_bundle_consent_file_requires_reconsent() {
        let dir = std::env::temp_dir().join(format!(
            "kukuri-consent-legacy-test-{}",
            current_unix_seconds()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let db_path = dir.join("kukuri.db");
        std::fs::write(
            db_path.with_extension(APP_CONSENT_FILE_EXTENSION),
            r#"{"accepted_bundle_version":2,"accepted_at":1700000000}"#,
        )
        .expect("write legacy consent file");

        let store = load_app_consent_store(&db_path);
        assert!(store.records.is_empty());
        assert!(!app_consent_satisfied(&store));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn canonical_legal_documents_match_the_runtime_bundle_version() {
        const TERMS: &str = include_str!("../../../../docs/legal/terms-of-service.md");
        const PRIVACY: &str = include_str!("../../../../docs/legal/privacy-policy.md");
        let expected = format!("Legal bundle version: {LEGAL_BUNDLE_VERSION}");

        assert!(TERMS.contains(&expected));
        assert!(PRIVACY.contains(&expected));
        for required_clause in [
            "利用資格と成人向け表現",
            "18歳以上",
            "公的な年齢確認",
            "投稿コンテンツの権利帰属",
            "必要な権利または許諾",
            "限定的な利用許諾",
            "投稿撤回後の取扱い",
            "広告、宣伝、生成 AI の学習",
        ] {
            assert!(
                TERMS.contains(required_clause),
                "terms must contain `{required_clause}`"
            );
        }
    }

    #[test]
    fn app_consent_round_trips_through_disk() {
        let dir =
            std::env::temp_dir().join(format!("kukuri-consent-test-{}", current_unix_seconds()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let db_path = dir.join("kukuri.db");

        assert!(load_app_consent_store(&db_path).records.is_empty());

        let store = AppConsentStore {
            records: vec![
                record("terms", LEGAL_BUNDLE_VERSION, 1_700_000_000),
                record("privacy", LEGAL_BUNDLE_VERSION, 1_700_000_000),
            ],
            age_attestations: vec![attestation(AGE_ATTESTATION_VERSION, 1_700_000_000)],
        };
        save_app_consent_store(&db_path, &store).expect("save consent");

        let loaded = load_app_consent_store(&db_path);
        assert_eq!(loaded.records.len(), 2);
        assert_eq!(loaded.records[0].slug, "terms");
        assert_eq!(loaded.records[0].language, "ja");
        assert_eq!(loaded.records[0].app_version, "0.1.8");
        assert_eq!(loaded.age_attestations.len(), 1);
        assert!(app_consent_satisfied(&loaded));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn startup_state_status_can_be_updated() {
        let state = DesktopStartupState::initializing();
        assert!(matches!(state.status(), DesktopStartupStatus::Initializing));
        state.set_status(DesktopStartupStatus::ConsentRequired {
            documents: app_consent_documents_status(&AppConsentStore::default()),
            age_attestation: age_attestation_status(&AppConsentStore::default()),
        });
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
