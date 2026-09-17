//! Community-node固有のsafety永続化とprovider解決。
//!
//! DB非依存のscan serviceと構築処理は`cn-safety-runtime`が所有する。このmoduleには
//! Postgres storeと、feature/credentialを伴うprovider実装名の解決だけを置く。

use std::sync::Arc;

use anyhow::{Context as _, Result, bail};
use async_trait::async_trait;
use sqlx::PgPool;

use kukuri_cn_safety::provider::{MediaFetcher, SubjectKind};
use kukuri_cn_safety::{
    ContentAdvisory, RiskSignalTarget, SafetyCategory, SafetyProvider, SafetyRiskSignal,
    SafetyVerdict, SignedModerationEvent,
};
use kukuri_cn_safety_runtime::{
    PersistedSignal, SafetyArtifactStore, SafetyRuntimeProviderEntry, SafetyRuntimeProvidersConfig,
    StoredVerdictRecord, VerdictPersistMeta,
};

use crate::safety_events::{
    attribute_risk_signal_subject_author, expire_superseded_advisory_signals,
    persist_risk_signal_deduplicated, persist_signed_moderation_event,
};
use crate::scan_verdicts::{get_scan_verdict, update_scan_verdict_advisories, upsert_scan_verdict};

#[derive(Clone, Debug)]
pub struct PgSafetyArtifactStore {
    pool: PgPool,
}

impl PgSafetyArtifactStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SafetyArtifactStore for PgSafetyArtifactStore {
    fn content_store(
        &self,
    ) -> Option<Arc<dyn kukuri_cn_safety_runtime::content_cache::ContentScanStore>> {
        Some(Arc::new(crate::PgContentScanStore::new(self.pool.clone())))
    }
    async fn persist_event(&self, event: &SignedModerationEvent) -> Result<()> {
        persist_signed_moderation_event(&self.pool, event)
            .await
            .map(|_| ())
    }

    async fn persist_signal(
        &self,
        issuer_node_id: &str,
        signal: &SafetyRiskSignal,
        subject_author: Option<&str>,
    ) -> Result<PersistedSignal> {
        persist_risk_signal_deduplicated(&self.pool, issuer_node_id, signal, subject_author)
            .await
            .map(|persisted| PersistedSignal {
                id: persisted.stored.id,
                newly_created: persisted.newly_created,
            })
    }

    async fn persist_verdict(
        &self,
        subject_kind: SubjectKind,
        subject_id: &str,
        verdict: &SafetyVerdict,
        meta: &VerdictPersistMeta,
    ) -> Result<String> {
        upsert_scan_verdict(&self.pool, subject_kind, subject_id, verdict, meta)
            .await
            .map(|stored| stored.id)
    }

    async fn load_verdict(
        &self,
        subject_kind: SubjectKind,
        subject_id: &str,
    ) -> Result<Option<StoredVerdictRecord>> {
        get_scan_verdict(&self.pool, subject_kind, subject_id)
            .await
            .map(|stored| stored.map(|stored| stored.to_record()))
    }

    async fn persist_advisories(
        &self,
        subject_kind: SubjectKind,
        subject_id: &str,
        advisories: &[ContentAdvisory],
    ) -> Result<()> {
        update_scan_verdict_advisories(&self.pool, subject_kind, subject_id, advisories).await
    }

    async fn attribute_subject_author(
        &self,
        target: RiskSignalTarget,
        target_id: &str,
        author: &str,
    ) -> Result<()> {
        attribute_risk_signal_subject_author(&self.pool, target, target_id, author).await
    }

    async fn expire_superseded_advisory_signals(
        &self,
        issuer_node_id: &str,
        target: RiskSignalTarget,
        target_id: &str,
        current_categories: &[SafetyCategory],
        expires_at: &str,
    ) -> Result<u64> {
        expire_superseded_advisory_signals(
            &self.pool,
            issuer_node_id,
            target,
            target_id,
            current_categories,
            expires_at,
        )
        .await
    }
}

/// Operator由来のslot設定を具体的なprovider実装へ解決する。
///
/// 未知名、slot不一致、credential欠落はすべて起動エラーとして返す。空設定はscan serviceを
/// 無効にする正規状態なので空vectorを返す。
///
/// `media_fetcher` は media 参照 scan 用の一時 fetch 手段（#609）。構成されていれば media を
/// 扱う provider（vlm / arachnid）へ接続する。未構成なら従来どおり media scan は
/// `Unavailable` → fail-closed。env のみから構築する `resolve_provider` に対する注入シーム。
pub fn resolve_safety_providers(
    providers: &SafetyRuntimeProvidersConfig,
    media_fetcher: Option<Arc<dyn MediaFetcher>>,
) -> Result<Vec<Arc<dyn SafetyProvider>>> {
    resolve_safety_providers_inner(providers, media_fetcher, None)
}

/// Production construction: OpenAI requests must use the node's persistent shared budget.
pub fn resolve_safety_providers_with_pool(
    providers: &SafetyRuntimeProvidersConfig,
    media_fetcher: Option<Arc<dyn MediaFetcher>>,
    pool: &PgPool,
) -> Result<Vec<Arc<dyn SafetyProvider>>> {
    resolve_safety_providers_inner(providers, media_fetcher, Some(pool))
}

fn resolve_safety_providers_inner(
    providers: &SafetyRuntimeProvidersConfig,
    media_fetcher: Option<Arc<dyn MediaFetcher>>,
    pool: Option<&PgPool>,
) -> Result<Vec<Arc<dyn SafetyProvider>>> {
    let slots: [(&'static str, Option<&SafetyRuntimeProviderEntry>); 3] = [
        ("known_csam", providers.known_csam.as_ref()),
        ("general", providers.general.as_ref()),
        ("unknown_csam", providers.unknown_csam.as_ref()),
    ];
    slots
        .into_iter()
        .filter_map(|(slot, entry)| entry.map(|entry| (slot, entry)))
        .map(|(slot, entry)| resolve_provider(slot, entry, media_fetcher.as_ref(), pool))
        .collect()
}

fn resolve_provider(
    slot: &'static str,
    entry: &SafetyRuntimeProviderEntry,
    media_fetcher: Option<&Arc<dyn MediaFetcher>>,
    pool: Option<&PgPool>,
) -> Result<Arc<dyn SafetyProvider>> {
    // feature 構成によっては未使用になる（mock は fetcher を要しない）。
    let _ = (media_fetcher, pool);
    let normalized = entry.provider.trim().replace('_', "-");
    match normalized.as_str() {
        #[cfg(feature = "safety-mock-provider")]
        "mock" => Ok(mock_provider_for_slot(slot)),
        #[cfg(feature = "safety-arachnid-provider")]
        kukuri_cn_safety_arachnid::PROVIDER_NAME => arachnid_shield_provider(slot, media_fetcher),
        #[cfg(feature = "safety-vlm-provider")]
        kukuri_cn_safety_vlm::PROVIDER_NAME => vlm_provider(slot, media_fetcher),
        #[cfg(feature = "safety-openai-provider")]
        kukuri_cn_safety_openai::PROVIDER_NAME => openai_provider(slot, media_fetcher, pool),
        other => {
            // 候補は build に実在する provider だけを挙げる（production binary のエラーが
            // 選択不能な mock を案内しないように。#614）。
            #[allow(unused_mut)]
            let mut supported: Vec<String> = Vec::new();
            #[cfg(feature = "safety-mock-provider")]
            supported.push("`mock`".to_string());
            #[cfg(feature = "safety-arachnid-provider")]
            supported.push(format!("`{}`", kukuri_cn_safety_arachnid::PROVIDER_NAME));
            #[cfg(feature = "safety-vlm-provider")]
            supported.push(format!("`{}`", kukuri_cn_safety_vlm::PROVIDER_NAME));
            #[cfg(feature = "safety-openai-provider")]
            supported.push(format!("`{}`", kukuri_cn_safety_openai::PROVIDER_NAME));
            let supported = if supported.is_empty() {
                "none (no safety provider feature is enabled in this build)".to_string()
            } else {
                supported.join(" / ")
            };
            bail!(
                "unknown safety provider `{other}` for slot `{slot}` (fail-closed; providers \
                 available in this build: {supported})"
            )
        }
    }
}

#[cfg(feature = "safety-openai-provider")]
fn openai_provider(
    slot: &str,
    fetcher: Option<&Arc<dyn MediaFetcher>>,
    pool: Option<&PgPool>,
) -> Result<Arc<dyn SafetyProvider>> {
    use kukuri_cn_safety_openai::{
        ModerationClient, ModerationConfig, ModerationCredentials, OpenAiModerationProvider,
    };
    use kukuri_cn_safety_video::{FfmpegVideoExtractor, VideoExtractConfig};
    if slot != "general" {
        bail!(
            "openai-moderation supports only the general slot; it is not a known-match or unknown-CSAM detector"
        );
    }
    let pool = pool.context("openai-moderation requires the persistent shared budget")?;
    let config = ModerationConfig::from_env().context("invalid OpenAI moderation configuration")?;
    let credentials =
        ModerationCredentials::from_env().context("missing OpenAI moderation credential")?;
    let client = ModerationClient::new(
        config,
        credentials,
        Arc::new(crate::PgModerationBudget::new(pool.clone())),
    )?;
    let mut provider = OpenAiModerationProvider::new(Arc::new(client));
    if let Some(fetcher) = fetcher {
        provider = provider.with_media_fetcher(fetcher.clone());
    }
    let extractor = FfmpegVideoExtractor::new(VideoExtractConfig::from_env()?)
        .context("video decoder is not ready for OpenAI moderation")?;
    Ok(Arc::new(provider.with_video_extractor(Arc::new(extractor))))
}

#[cfg(feature = "safety-vlm-provider")]
fn vlm_provider(
    slot: &'static str,
    media_fetcher: Option<&Arc<dyn MediaFetcher>>,
) -> Result<Arc<dyn SafetyProvider>> {
    use kukuri_cn_safety_vlm::{CapabilityProfile, VlmModerationProvider};

    // classifier 系 provider なので known_csam（known-match）slot には使えない（ADR 0028 §2.1:
    // basis を confirmed に昇格させない。known-match の役割は #391 系 provider が担う）。
    let profile = match slot {
        "general" => CapabilityProfile::General,
        "unknown_csam" => CapabilityProfile::UnknownCsam,
        _ => bail!(
            "safety provider `{}` only supports the `general` / `unknown_csam` slots \
             (got `{slot}`); it is a classifier provider and must not be used as the \
             known-CSAM match provider",
            kukuri_cn_safety_vlm::PROVIDER_NAME
        ),
    };
    let mut provider = VlmModerationProvider::from_env(profile)
        .context("failed to configure the openai-compatible-vlm safety provider")?;
    if let Some(fetcher) = media_fetcher {
        provider = provider.with_media_fetcher(fetcher.clone());
    }
    Ok(Arc::new(provider))
}

#[cfg(feature = "safety-arachnid-provider")]
fn arachnid_shield_provider(
    slot: &'static str,
    media_fetcher: Option<&Arc<dyn MediaFetcher>>,
) -> Result<Arc<dyn SafetyProvider>> {
    if slot != "known_csam" {
        bail!(
            "safety provider `{}` only supports the `known_csam` slot (got `{slot}`); \
             it is a known-match provider and must not be used for general / unknown-CSAM slots",
            kukuri_cn_safety_arachnid::PROVIDER_NAME
        );
    }
    let mut provider = kukuri_cn_safety_arachnid::ProjectArachnidShieldProvider::from_env()
        .context("failed to configure the project-arachnid-shield safety provider")?;
    if let Some(fetcher) = media_fetcher {
        provider = provider.with_media_fetcher(fetcher.clone());
    }
    Ok(Arc::new(provider))
}

#[cfg(feature = "safety-mock-provider")]
fn mock_provider_for_slot(slot: &'static str) -> Arc<dyn SafetyProvider> {
    use kukuri_cn_safety::{MockSafetyProvider, SafetyProviderCapability};

    let name = format!("mock-{slot}");
    let provider = match slot {
        "known_csam" => MockSafetyProvider::known_csam(name),
        "general" => MockSafetyProvider::with_capabilities(
            name,
            vec![
                SafetyProviderCapability::GeneralMediaModeration,
                SafetyProviderCapability::SpamAbuseModeration,
            ],
        ),
        _ => MockSafetyProvider::with_capabilities(
            name,
            vec![
                SafetyProviderCapability::NovelCsamImageClassifier,
                SafetyProviderCapability::CseTextClassifier,
            ],
        ),
    };
    Arc::new(provider)
}
