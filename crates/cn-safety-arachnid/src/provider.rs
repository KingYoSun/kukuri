//! `SafetyProvider` 実装（#391）。
//!
//! Shield の scan 結果を domain の `ProviderScanResult` に写像する。写像表:
//!
//! | classification | match_type | 写像 |
//! |---|---|---|
//! | `csam` / `harmful-abusive-material` | `exact` / なし | `known_hash_match=true` + `Csam` ラベル（capability `KnownCsamHashMatch`）→ router で `Exclude` / `csam_confirmed` |
//! | `csam` / `harmful-abusive-material` | `near` | 同上（capability は `PerceptualHashMatch`）|
//! | `test` | 任意 | `ProviderTest` ラベル（critical でない）→ router で `Exclude` / `provider_test_match`（`csam_confirmed` と区別） |
//! | `no-known-match` | - | `ScanOutcome::NoKnownMatch`（**safe の証明ではない**） |
//!
//! `harmful-abusive-material` は CSAM の法的定義に該当しない有害・虐待的既知コンテンツだが、
//! Shield の既知リスト一致であることに変わりはないため、保守的に known match（category `Csam`）
//! として除外する（over-exclusion 側に倒す。ADR 0027 追補に記録）。

use std::sync::Arc;

use async_trait::async_trait;

use kukuri_cn_safety::provider::{
    MediaFetcher, ProviderScanRequest, ProviderScanResult, ScanError, ScanOutcome,
};
use kukuri_cn_safety::{SafetyCategory, SafetyLabel, SafetyProvider, SafetyProviderCapability};

use crate::client::{ShieldClassification, ShieldClient, ShieldMatchType, ShieldScanResult};
use crate::config::{ShieldConfigError, ShieldProviderConfig};

/// provider の安定識別子。config（provider slot）の値・verdict の `provider` に使う。
pub const PROVIDER_NAME: &str = "project-arachnid-shield";

const CAPABILITIES: [SafetyProviderCapability; 2] = [
    SafetyProviderCapability::KnownCsamHashMatch,
    SafetyProviderCapability::PerceptualHashMatch,
];

/// Project Arachnid Shield を known-CSAM（child-safety known-match）provider として実装する。
pub struct ProjectArachnidShieldProvider {
    client: ShieldClient,
    /// media_hint → 一時 bytes の取得手段。未構成なら media scan は `Unavailable`（fail-closed）。
    fetcher: Option<Arc<dyn MediaFetcher>>,
}

impl std::fmt::Debug for ProjectArachnidShieldProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectArachnidShieldProvider")
            .field("name", &PROVIDER_NAME)
            .field("client", &self.client)
            .field("fetcher_configured", &self.fetcher.is_some())
            .finish()
    }
}

impl ProjectArachnidShieldProvider {
    /// 設定から provider を組み立てる。credentials は設定が指す env から読む。
    ///
    /// credentials 欠落は Err（呼び出し側で起動失敗 = fail-closed）。
    pub fn new(config: &ShieldProviderConfig) -> Result<Self, ShieldConfigError> {
        Ok(Self {
            client: ShieldClient::from_config(config)?,
            fetcher: None,
        })
    }

    /// env（base URL / timeout の上書き含む）から provider を組み立てる。
    pub fn from_env() -> Result<Self, ShieldConfigError> {
        Self::new(&ShieldProviderConfig::from_env()?)
    }

    /// 明示的な credentials から組み立てる（テスト / env 以外の secret 供給経路用）。
    pub fn with_credentials(
        config: &ShieldProviderConfig,
        credentials: crate::config::ShieldCredentials,
    ) -> Result<Self, ShieldConfigError> {
        Ok(Self {
            client: ShieldClient::new(config, credentials)?,
            fetcher: None,
        })
    }

    /// media_hint から一時 bytes を取得する fetcher を接続する（media scan pipeline 側が注入）。
    pub fn with_media_fetcher(mut self, fetcher: Arc<dyn MediaFetcher>) -> Self {
        self.fetcher = Some(fetcher);
        self
    }
}

#[async_trait]
impl SafetyProvider for ProjectArachnidShieldProvider {
    fn name(&self) -> &str {
        PROVIDER_NAME
    }

    fn capabilities(&self) -> &[SafetyProviderCapability] {
        &CAPABILITIES
    }

    async fn scan(&self, request: &ProviderScanRequest) -> Result<ProviderScanResult, ScanError> {
        let media_hint = request
            .media_hint
            .as_deref()
            .map(str::trim)
            .filter(|hint| !hint.is_empty());

        // media の無い対象（text-only post 等）は hash 照合の対象が存在しない。
        // NoKnownMatch（safe の証明ではない）として「known-CSAM scan を実施した」記録だけ残す。
        let Some(media_hint) = media_hint else {
            return Ok(base_result(ScanOutcome::NoKnownMatch));
        };

        // media_hint があるのに fetcher が未構成なら scan 不能 → fail-closed。
        let Some(fetcher) = &self.fetcher else {
            return Err(ScanError::Unavailable(
                "project-arachnid-shield has no media fetcher configured; \
                 cannot scan referenced media (fail-closed)"
                    .to_string(),
            ));
        };

        let media = fetcher.fetch(media_hint).await?;
        let scan = self
            .client
            .scan_media(media.bytes, &media.content_type)
            .await
            .map_err(ScanError::from)?;
        Ok(scan_result_from(scan))
    }
}

/// 検知なしの素の結果。
fn base_result(outcome: ScanOutcome) -> ProviderScanResult {
    ProviderScanResult {
        provider: PROVIDER_NAME.to_string(),
        capability: SafetyProviderCapability::KnownCsamHashMatch,
        outcome,
        known_hash_match: false,
        score: None,
        labels: Vec::new(),
        derived_tags: Vec::new(),
    }
}

/// Shield 応答を domain の `ProviderScanResult` に写像する（module doc の写像表）。
fn scan_result_from(scan: ShieldScanResult) -> ProviderScanResult {
    match scan.classification {
        ShieldClassification::Csam | ShieldClassification::HarmfulAbusiveMaterial => {
            let capability = match scan.match_type {
                Some(ShieldMatchType::Near) => SafetyProviderCapability::PerceptualHashMatch,
                // exact / match_type 欠落は known hash match として扱う（保守側）。
                _ => SafetyProviderCapability::KnownCsamHashMatch,
            };
            ProviderScanResult {
                provider: PROVIDER_NAME.to_string(),
                capability,
                outcome: ScanOutcome::Completed,
                known_hash_match: true,
                score: None,
                labels: vec![
                    SafetyLabel::new(SafetyCategory::Csam).with_provider_capability(capability),
                ],
                derived_tags: Vec::new(),
            }
        }
        // provider self-test データへの一致。csam_confirmed と区別する専用 route（#391）。
        ShieldClassification::Test => ProviderScanResult {
            provider: PROVIDER_NAME.to_string(),
            capability: SafetyProviderCapability::KnownCsamHashMatch,
            outcome: ScanOutcome::Completed,
            known_hash_match: false,
            score: None,
            labels: vec![
                SafetyLabel::new(SafetyCategory::ProviderTest)
                    .with_provider_capability(SafetyProviderCapability::KnownCsamHashMatch),
            ],
            derived_tags: Vec::new(),
        },
        ShieldClassification::NoKnownMatch => base_result(ScanOutcome::NoKnownMatch),
    }
}
