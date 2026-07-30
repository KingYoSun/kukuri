//! Community-node固有のsafety永続化とprovider解決。
//!
//! DB非依存のscan serviceと構築処理は`cn-safety-runtime`が所有する。このmoduleには
//! Postgres storeと、feature/credentialを伴うprovider実装名の解決だけを置く。

use std::sync::Arc;

use anyhow::{Context as _, Result, bail};
use async_trait::async_trait;
use sqlx::PgPool;

use kukuri_cn_safety::provider::SubjectKind;
use kukuri_cn_safety::{SafetyProvider, SafetyRiskSignal, SafetyVerdict, SignedModerationEvent};
use kukuri_cn_safety_runtime::{
    SafetyArtifactStore, SafetyRuntimeProviderEntry, SafetyRuntimeProvidersConfig,
};

use crate::safety_events::{persist_risk_signal, persist_signed_moderation_event};
use crate::scan_verdicts::upsert_scan_verdict;

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
    async fn persist_event(&self, event: &SignedModerationEvent) -> Result<()> {
        persist_signed_moderation_event(&self.pool, event)
            .await
            .map(|_| ())
    }

    async fn persist_signal(
        &self,
        issuer_node_id: &str,
        signal: &SafetyRiskSignal,
    ) -> Result<String> {
        persist_risk_signal(&self.pool, issuer_node_id, signal)
            .await
            .map(|stored| stored.id)
    }

    async fn persist_verdict(
        &self,
        subject_kind: SubjectKind,
        subject_id: &str,
        verdict: &SafetyVerdict,
    ) -> Result<String> {
        upsert_scan_verdict(&self.pool, subject_kind, subject_id, verdict)
            .await
            .map(|stored| stored.id)
    }
}

/// Operator由来のslot設定を具体的なprovider実装へ解決する。
///
/// 未知名、slot不一致、credential欠落はすべて起動エラーとして返す。空設定はscan serviceを
/// 無効にする正規状態なので空vectorを返す。
pub fn resolve_safety_providers(
    providers: &SafetyRuntimeProvidersConfig,
) -> Result<Vec<Arc<dyn SafetyProvider>>> {
    let slots: [(&'static str, Option<&SafetyRuntimeProviderEntry>); 3] = [
        ("known_csam", providers.known_csam.as_ref()),
        ("general", providers.general.as_ref()),
        ("unknown_csam", providers.unknown_csam.as_ref()),
    ];
    slots
        .into_iter()
        .filter_map(|(slot, entry)| entry.map(|entry| (slot, entry)))
        .map(|(slot, entry)| resolve_provider(slot, entry))
        .collect()
}

fn resolve_provider(
    slot: &'static str,
    entry: &SafetyRuntimeProviderEntry,
) -> Result<Arc<dyn SafetyProvider>> {
    let normalized = entry.provider.trim().replace('_', "-");
    match normalized.as_str() {
        #[cfg(feature = "safety-mock-provider")]
        "mock" => Ok(mock_provider_for_slot(slot)),
        #[cfg(feature = "safety-arachnid-provider")]
        kukuri_cn_safety_arachnid::PROVIDER_NAME => arachnid_shield_provider(slot),
        #[cfg(feature = "safety-vlm-provider")]
        kukuri_cn_safety_vlm::PROVIDER_NAME => vlm_provider(slot),
        other => bail!(
            "unknown safety provider `{other}` for slot `{slot}` (fail-closed; enable the \
             matching cargo feature and use `mock` / `project-arachnid-shield` / \
             `openai-compatible-vlm`)"
        ),
    }
}

#[cfg(feature = "safety-vlm-provider")]
fn vlm_provider(slot: &'static str) -> Result<Arc<dyn SafetyProvider>> {
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
    let provider = VlmModerationProvider::from_env(profile)
        .context("failed to configure the openai-compatible-vlm safety provider")?;
    Ok(Arc::new(provider))
}

#[cfg(feature = "safety-arachnid-provider")]
fn arachnid_shield_provider(slot: &'static str) -> Result<Arc<dyn SafetyProvider>> {
    if slot != "known_csam" {
        bail!(
            "safety provider `{}` only supports the `known_csam` slot (got `{slot}`); \
             it is a known-match provider and must not be used for general / unknown-CSAM slots",
            kukuri_cn_safety_arachnid::PROVIDER_NAME
        );
    }
    let provider = kukuri_cn_safety_arachnid::ProjectArachnidShieldProvider::from_env()
        .context("failed to configure the project-arachnid-shield safety provider")?;
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
