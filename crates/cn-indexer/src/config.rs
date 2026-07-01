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
}

impl std::fmt::Debug for IndexerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexerConfig")
            .field("database_url", &self.database_url)
            .field("data_dir", &self.data_dir)
            .field("relay", &self.relay)
            .field("channel_secret_key", &"<redacted>")
            .field("arcadedb", &self.arcadedb)
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
        Ok(Self {
            database_url,
            data_dir,
            relay: RelayConfig::new(has_own_relay, external_relay_urls),
            channel_secret_key,
            arcadedb,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
