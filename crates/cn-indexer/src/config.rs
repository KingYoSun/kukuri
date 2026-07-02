//! cn-indexer の起動設定と relay validation 起動 gate（#413 / ADR 0025 §6.4）。
//!
//! indexing = Model C（docs replica sync participant）は peer discovery を成立させるために relay を
//! 前提にする。CN は relay 抜き構成を許容するため、indexing 起動時に relay を **config 検査**として
//! validate する。自前 relay（operator config の `features.iroh_relay` 有効）または外部 relay URL
//! （`external_relay_urls`）のどちらかが設定されていることを必須とし、両方未設定なら indexing を
//! 起動しない（fail-closed）。
//!
//! 到達性の実測（liveness probe）はしない。設定の有無のみで判定することで決定論的にテストでき、
//! `IrohDocsNode` の「relay 未活性でも継続」挙動と矛盾しない。

use anyhow::{Context, Result, bail};

use kukuri_cn_core::{
    SafetyRuntimeConfig, SafetyRuntimeProviderEntry, SafetyRuntimeProvidersConfig,
};
use kukuri_cn_safety_runtime::SAFETY_SIGNING_KEY_ENV;

/// safety provider slot（#406。operator config の `safety.providers.*` と 1:1）の env 名。
/// 値は provider 実装名（現状 `mock` のみ。#391 / #411 で本番実装名を追加）。
/// `<SLOT>_REQUIRED`（bool）で required 宣言を併記できる。
pub const SAFETY_PROVIDER_KNOWN_CSAM_ENV: &str = "COMMUNITY_NODE_SAFETY_PROVIDER_KNOWN_CSAM";
pub const SAFETY_PROVIDER_GENERAL_ENV: &str = "COMMUNITY_NODE_SAFETY_PROVIDER_GENERAL";
pub const SAFETY_PROVIDER_UNKNOWN_CSAM_ENV: &str = "COMMUNITY_NODE_SAFETY_PROVIDER_UNKNOWN_CSAM";
/// signed moderation event を発行するか（operator config の
/// `safety.events.emit_signed_moderation_events` に対応。既定 true）。
pub const SAFETY_EMIT_SIGNED_EVENTS_ENV: &str = "COMMUNITY_NODE_SAFETY_EMIT_SIGNED_EVENTS";
/// signed event 無効時に risk signal issuer として使う node id。
pub const SAFETY_ISSUER_NODE_ID_ENV: &str = "COMMUNITY_NODE_SAFETY_ISSUER_NODE_ID";

/// relay validation の結果。fail-closed gate の単一判定点。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelayValidation {
    /// 自前 relay（operator が iroh_relay capability を有効化）で成立。
    OwnRelay,
    /// 外部 relay URL 設定で成立。
    ExternalRelay,
    /// 自前 relay と外部 relay の両方で成立。
    OwnAndExternalRelay,
}

/// cn-indexer の relay 構成。
///
/// `has_own_relay` は operator config の `features.iroh_relay` 由来（自前 relay を提供する構成か）、
/// `external_relay_urls` は cn-indexer 自身が discovery / relay-assist に使う外部 relay URL。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RelayConfig {
    pub has_own_relay: bool,
    pub external_relay_urls: Vec<String>,
}

impl RelayConfig {
    pub fn new(has_own_relay: bool, external_relay_urls: Vec<String>) -> Self {
        Self {
            has_own_relay,
            external_relay_urls: normalize_relay_urls(external_relay_urls),
        }
    }

    /// indexing 起動の relay validation gate（ADR 0025 §6.4）。
    ///
    /// 自前 relay も外部 relay も無ければ `Err`（indexing を起動しない）。どちらかが有れば
    /// どの経路で成立したかを返す。
    pub fn validate_for_startup(&self) -> Result<RelayValidation> {
        let has_external = !self.external_relay_urls.is_empty();
        match (self.has_own_relay, has_external) {
            (true, true) => Ok(RelayValidation::OwnAndExternalRelay),
            (true, false) => Ok(RelayValidation::OwnRelay),
            (false, true) => Ok(RelayValidation::ExternalRelay),
            (false, false) => bail!(
                "cn-indexer requires a validated relay to start indexing: enable the node's own \
                 iroh_relay capability or configure COMMUNITY_NODE_INDEXER_EXTERNAL_RELAY_URLS"
            ),
        }
    }
}

/// 空白除去・重複排除した relay URL 一覧。
fn normalize_relay_urls(urls: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    urls.into_iter()
        .map(|url| url.trim().to_string())
        .filter(|url| !url.is_empty())
        .filter(|url| seen.insert(url.clone()))
        .collect()
}

/// cn-indexer の全体設定。
///
/// `Debug` は手動実装で `channel_secret_key` を秘匿する（誤ってログへ暗号鍵を出さない）。
#[derive(Clone)]
pub struct IndexerConfig {
    /// Postgres 接続 URL（supported set / request / channel secret の scope state）。
    pub database_url: String,
    /// docs / blob store の永続ディレクトリ。
    pub data_dir: std::path::PathBuf,
    /// relay 構成（起動 gate）。
    pub relay: RelayConfig,
    /// channel secret を復号する鍵 material（cn-user-api と同じ値）。
    pub channel_secret_key: String,
    /// ArcadeDB index 投影の接続設定。
    pub arcadedb: ArcadeDbConfig,
    /// safety scan runtime（#406）の構成。provider 未構成なら scan service は構築されない。
    pub safety: SafetyRuntimeConfig,
}

impl std::fmt::Debug for IndexerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexerConfig")
            .field("database_url", &self.database_url)
            .field("data_dir", &self.data_dir)
            .field("relay", &self.relay)
            .field("channel_secret_key", &"<redacted>")
            .field("arcadedb", &self.arcadedb)
            // SafetyRuntimeConfig の Debug は signing_key を秘匿する。
            .field("safety", &self.safety)
            .finish()
    }
}

/// ArcadeDB index 投影の接続設定。
///
/// `Debug` は手動実装で `password` を秘匿する。
#[derive(Clone, PartialEq, Eq)]
pub struct ArcadeDbConfig {
    pub base_url: String,
    pub database: String,
    pub username: String,
    pub password: String,
}

impl std::fmt::Debug for ArcadeDbConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArcadeDbConfig")
            .field("base_url", &self.base_url)
            .field("database", &self.database)
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .finish()
    }
}

impl IndexerConfig {
    /// 環境変数から設定を読む。
    ///
    /// relay validation gate（§6.4）は起動側（`run_from_env`）で `relay.validate_for_startup()` を
    /// 呼んで適用する。ここでは値の読み取りのみを行う。
    pub fn from_env() -> Result<Self> {
        let database_url = std::env::var("COMMUNITY_NODE_DATABASE_URL")
            .context("COMMUNITY_NODE_DATABASE_URL is required")?;
        let data_dir = std::env::var("COMMUNITY_NODE_INDEXER_DATA_DIR")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "./data/cn-indexer".to_string())
            .into();
        let has_own_relay =
            kukuri_cn_core::parse_bool_env("COMMUNITY_NODE_INDEXER_OWN_RELAY", false)?;
        let external_relay_urls =
            kukuri_cn_core::parse_csv_env("COMMUNITY_NODE_INDEXER_EXTERNAL_RELAY_URLS");
        let channel_secret_key = std::env::var("COMMUNITY_NODE_CHANNEL_SECRET_KEY")
            .context("COMMUNITY_NODE_CHANNEL_SECRET_KEY is required")?;
        let arcadedb = ArcadeDbConfig {
            base_url: std::env::var("COMMUNITY_NODE_ARCADEDB_URL")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "http://127.0.0.1:2480".to_string()),
            database: std::env::var("COMMUNITY_NODE_ARCADEDB_DATABASE")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "kukuri_index".to_string()),
            username: std::env::var("COMMUNITY_NODE_ARCADEDB_USER")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "root".to_string()),
            password: std::env::var("COMMUNITY_NODE_ARCADEDB_PASSWORD").unwrap_or_default(),
        };
        let safety = SafetyRuntimeConfig {
            providers: SafetyRuntimeProvidersConfig {
                known_csam: safety_provider_entry(
                    non_empty_env(SAFETY_PROVIDER_KNOWN_CSAM_ENV),
                    kukuri_cn_core::parse_bool_env(
                        &format!("{SAFETY_PROVIDER_KNOWN_CSAM_ENV}_REQUIRED"),
                        false,
                    )?,
                ),
                general: safety_provider_entry(
                    non_empty_env(SAFETY_PROVIDER_GENERAL_ENV),
                    kukuri_cn_core::parse_bool_env(
                        &format!("{SAFETY_PROVIDER_GENERAL_ENV}_REQUIRED"),
                        false,
                    )?,
                ),
                unknown_csam: safety_provider_entry(
                    non_empty_env(SAFETY_PROVIDER_UNKNOWN_CSAM_ENV),
                    kukuri_cn_core::parse_bool_env(
                        &format!("{SAFETY_PROVIDER_UNKNOWN_CSAM_ENV}_REQUIRED"),
                        false,
                    )?,
                ),
            },
            signing_key: non_empty_env(SAFETY_SIGNING_KEY_ENV),
            emit_signed_events: kukuri_cn_core::parse_bool_env(
                SAFETY_EMIT_SIGNED_EVENTS_ENV,
                true,
            )?,
            issuer_node_id: non_empty_env(SAFETY_ISSUER_NODE_ID_ENV),
        };
        Ok(Self {
            database_url,
            data_dir,
            relay: RelayConfig::new(has_own_relay, external_relay_urls),
            channel_secret_key,
            arcadedb,
            safety,
        })
    }
}

/// 空 / 空白のみの env は未設定として扱う。
fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// provider slot env の値から entry を組む。空 / 空白は「slot 未構成」。
fn safety_provider_entry(
    provider: Option<String>,
    required: bool,
) -> Option<SafetyRuntimeProviderEntry> {
    provider.map(|provider| SafetyRuntimeProviderEntry { provider, required })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex, OnceLock};

    const TEST_SECRET: &str = "0000000000000000000000000000000000000000000000000000000000000001";
    const SAFETY_PROVIDER_KNOWN_CSAM_REQUIRED_ENV: &str =
        "COMMUNITY_NODE_SAFETY_PROVIDER_KNOWN_CSAM_REQUIRED";
    const SAFETY_PROVIDER_GENERAL_REQUIRED_ENV: &str =
        "COMMUNITY_NODE_SAFETY_PROVIDER_GENERAL_REQUIRED";
    const SAFETY_PROVIDER_UNKNOWN_CSAM_REQUIRED_ENV: &str =
        "COMMUNITY_NODE_SAFETY_PROVIDER_UNKNOWN_CSAM_REQUIRED";
    const INDEXER_ENV_KEYS: &[&str] = &[
        "COMMUNITY_NODE_DATABASE_URL",
        "COMMUNITY_NODE_INDEXER_DATA_DIR",
        "COMMUNITY_NODE_INDEXER_OWN_RELAY",
        "COMMUNITY_NODE_INDEXER_EXTERNAL_RELAY_URLS",
        "COMMUNITY_NODE_CHANNEL_SECRET_KEY",
        "COMMUNITY_NODE_ARCADEDB_URL",
        "COMMUNITY_NODE_ARCADEDB_DATABASE",
        "COMMUNITY_NODE_ARCADEDB_USER",
        "COMMUNITY_NODE_ARCADEDB_PASSWORD",
        SAFETY_PROVIDER_KNOWN_CSAM_ENV,
        SAFETY_PROVIDER_KNOWN_CSAM_REQUIRED_ENV,
        SAFETY_PROVIDER_GENERAL_ENV,
        SAFETY_PROVIDER_GENERAL_REQUIRED_ENV,
        SAFETY_PROVIDER_UNKNOWN_CSAM_ENV,
        SAFETY_PROVIDER_UNKNOWN_CSAM_REQUIRED_ENV,
        SAFETY_SIGNING_KEY_ENV,
        SAFETY_EMIT_SIGNED_EVENTS_ENV,
        SAFETY_ISSUER_NODE_ID_ENV,
    ];

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock")
    }

    fn with_clean_indexer_env<T>(f: impl FnOnce() -> T) -> T {
        let _guard = env_lock();
        let snapshot: Vec<(&'static str, Option<String>)> = INDEXER_ENV_KEYS
            .iter()
            .map(|key| (*key, std::env::var(key).ok()))
            .collect();
        for key in INDEXER_ENV_KEYS {
            unsafe { std::env::remove_var(key) };
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        for (key, value) in snapshot {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    fn set_minimal_indexer_env() {
        unsafe {
            std::env::set_var("COMMUNITY_NODE_DATABASE_URL", "postgres://example");
            std::env::set_var("COMMUNITY_NODE_CHANNEL_SECRET_KEY", "test-channel-secret");
        }
    }

    #[test]
    fn startup_fails_without_own_or_external_relay() {
        let config = RelayConfig::new(false, vec![]);
        assert!(config.validate_for_startup().is_err());
    }

    #[test]
    fn startup_succeeds_with_own_relay() {
        let config = RelayConfig::new(true, vec![]);
        assert_eq!(
            config.validate_for_startup().unwrap(),
            RelayValidation::OwnRelay
        );
    }

    #[test]
    fn startup_succeeds_with_external_relay() {
        let config = RelayConfig::new(false, vec!["https://relay.example.net".to_string()]);
        assert_eq!(
            config.validate_for_startup().unwrap(),
            RelayValidation::ExternalRelay
        );
    }

    #[test]
    fn startup_reports_both_when_own_and_external_present() {
        let config = RelayConfig::new(true, vec!["https://relay.example.net".to_string()]);
        assert_eq!(
            config.validate_for_startup().unwrap(),
            RelayValidation::OwnAndExternalRelay
        );
    }

    #[test]
    fn blank_external_relay_urls_do_not_satisfy_gate() {
        let config = RelayConfig::new(false, vec!["   ".to_string(), String::new()]);
        assert!(config.external_relay_urls.is_empty());
        assert!(config.validate_for_startup().is_err());
    }

    #[test]
    fn external_relay_urls_are_deduplicated() {
        let config = RelayConfig::new(
            false,
            vec![
                "https://relay.example.net".to_string(),
                "https://relay.example.net".to_string(),
                " https://relay.example.net ".to_string(),
            ],
        );
        assert_eq!(config.external_relay_urls.len(), 1);
    }

    #[test]
    fn unset_safety_provider_env_yields_no_entry() {
        // slot 未構成（env 未設定）は entry を作らない → provider 全 slot 未構成なら
        // scan service は構築されない（fail-closed。cn-core 側 contract test で固定）。
        assert!(safety_provider_entry(None, true).is_none());
    }

    #[test]
    fn safety_provider_entry_keeps_name_and_required_flag() {
        let entry = safety_provider_entry(Some("mock".to_string()), true).unwrap();
        assert_eq!(entry.provider, "mock");
        assert!(entry.required);
    }

    #[test]
    fn safety_emit_signed_events_defaults_to_true() {
        // operator config の default（emit_signed_moderation_events = true）と一致する。
        assert!(SafetyRuntimeConfig::default().emit_signed_events);
    }

    #[test]
    fn safety_service_is_absent_without_provider_config() {
        with_clean_indexer_env(|| {
            set_minimal_indexer_env();

            let config = IndexerConfig::from_env().unwrap();
            assert!(config.safety.providers.is_empty());

            let store = Arc::new(kukuri_cn_core::MemorySafetyArtifactStore::new());
            let service = kukuri_cn_core::build_safety_scan_service(&config.safety, store).unwrap();
            assert!(service.is_none());
        });
    }

    #[test]
    fn safety_env_parses_provider_slots() {
        with_clean_indexer_env(|| {
            set_minimal_indexer_env();
            unsafe {
                std::env::set_var(SAFETY_PROVIDER_KNOWN_CSAM_ENV, " mock ");
                std::env::set_var(SAFETY_PROVIDER_KNOWN_CSAM_REQUIRED_ENV, "true");
                std::env::set_var(SAFETY_PROVIDER_GENERAL_ENV, "mock");
                std::env::set_var(SAFETY_PROVIDER_GENERAL_REQUIRED_ENV, "false");
                std::env::set_var(SAFETY_PROVIDER_UNKNOWN_CSAM_ENV, "mock");
                std::env::set_var(SAFETY_PROVIDER_UNKNOWN_CSAM_REQUIRED_ENV, "1");
                std::env::set_var(SAFETY_SIGNING_KEY_ENV, TEST_SECRET);
                std::env::set_var(SAFETY_EMIT_SIGNED_EVENTS_ENV, "off");
                std::env::set_var(SAFETY_ISSUER_NODE_ID_ENV, " issuer-node ");
            }

            let config = IndexerConfig::from_env().unwrap();
            let known = config.safety.providers.known_csam.as_ref().unwrap();
            assert_eq!(known.provider, "mock");
            assert!(known.required);
            let general = config.safety.providers.general.as_ref().unwrap();
            assert_eq!(general.provider, "mock");
            assert!(!general.required);
            let unknown = config.safety.providers.unknown_csam.as_ref().unwrap();
            assert_eq!(unknown.provider, "mock");
            assert!(unknown.required);
            assert_eq!(config.safety.signing_key.as_deref(), Some(TEST_SECRET));
            assert!(!config.safety.emit_signed_events);
            assert_eq!(config.safety.issuer_node_id.as_deref(), Some("issuer-node"));
        });
    }
}
