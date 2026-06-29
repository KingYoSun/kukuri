//! scan orchestration（#353 段階3b）。
//!
//! 登録された `SafetyProvider` を登録順に逐次実行し、各 `ProviderScanResult`（provider が
//! `Err` を返した場合は `ScanError` を `ScanOutcome` に写像して合成）を集約して `route()` に
//! 渡し、`SafetyVerdict` と未署名 moderation artifact を返す。
//!
//! fail-closed の要: provider が `Err` を返しても結果集合から除外せず、必ず `Failed` /
//! `Unavailable` の `ProviderScanResult` を合成する。これにより一部成功 + 一部失敗の取りこぼしを
//! 防ぐ。

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use kukuri_cn_safety::provider::{ProviderScanRequest, ProviderScanResult, ScanError, ScanOutcome};
use kukuri_cn_safety::{
    ModerationEventBody, SafetyPolicy, SafetyProvider, SafetyRiskSignal, SafetyVerdict, route,
};

use crate::artifacts::build_artifacts;
use crate::clock::ScanClock;
use crate::error::SafetyRuntimeError;
use crate::id::EventIdGenerator;

/// orchestrator の scan 出力。
///
/// `verdict` に加え、監査・テスト用の生 scan 結果と、未署名の moderation artifact を含む。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SafetyScanReport {
    pub verdict: SafetyVerdict,
    /// route() に渡した provider 結果（provider 失敗は写像済み）。
    pub scan_results: Vec<ProviderScanResult>,
    /// 未署名 moderation event。indexable / target 欠落時は `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moderation_event: Option<ModerationEventBody>,
    /// risk signal。indexable / target 欠落 / content category 不明時は `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_signal: Option<SafetyRiskSignal>,
}

/// `ScanError` を fail-closed な `ScanOutcome` に写像する。
///
/// `Unavailable` は `Unavailable`、`Timeout` / `Protocol` は `Failed`。どのエラーも `allow` に
/// 落ちる `ScanOutcome` には写像しない。
pub fn map_scan_error(error: &ScanError) -> ScanOutcome {
    match error {
        ScanError::Unavailable(_) => ScanOutcome::Unavailable,
        ScanError::Timeout(_) | ScanError::Protocol(_) => ScanOutcome::Failed,
    }
}

/// safety provider を駆動し verdict / artifact を組み立てる orchestrator。
pub struct SafetyOrchestrator {
    providers: Vec<Arc<dyn SafetyProvider>>,
    policy: SafetyPolicy,
    issuer_node_id: String,
    clock: Arc<dyn ScanClock>,
    ids: Arc<dyn EventIdGenerator>,
}

impl std::fmt::Debug for SafetyOrchestrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // trait object（providers / clock / ids）は Debug を持たないため、構成の要約のみ出す。
        f.debug_struct("SafetyOrchestrator")
            .field("provider_count", &self.providers.len())
            .field("policy", &self.policy)
            .field("issuer_node_id", &self.issuer_node_id)
            .finish_non_exhaustive()
    }
}

impl SafetyOrchestrator {
    /// builder を開始する。
    pub fn builder(
        issuer_node_id: impl Into<String>,
        clock: Arc<dyn ScanClock>,
        ids: Arc<dyn EventIdGenerator>,
    ) -> SafetyOrchestratorBuilder {
        SafetyOrchestratorBuilder {
            providers: Vec::new(),
            policy: None,
            issuer_node_id: issuer_node_id.into(),
            clock,
            ids,
        }
    }

    /// 単一 subject を scan して verdict / 未署名 artifact を返す。
    ///
    /// providers を登録順に逐次実行し、`Err` は `map_scan_error` で `ProviderScanResult` に
    /// 合成する。集約結果を `route()` に渡して verdict を得て、未署名 moderation artifact を
    /// 生成する。
    pub async fn scan_subject(&self, request: &ProviderScanRequest) -> SafetyScanReport {
        let scanned_at = self.clock.now_rfc3339();

        let mut scan_results = Vec::with_capacity(self.providers.len());
        for provider in &self.providers {
            match provider.scan(request).await {
                Ok(result) => scan_results.push(result),
                Err(error) => scan_results.push(synthesize_failure(provider.as_ref(), &error)),
            }
        }

        let verdict = route(&scan_results, &self.policy, scanned_at);
        let (moderation_event, risk_signal) =
            build_artifacts(&verdict, request, &self.issuer_node_id, self.ids.as_ref());

        SafetyScanReport {
            verdict,
            scan_results,
            moderation_event,
            risk_signal,
        }
    }
}

/// provider が `Err` を返したときに合成する fail-closed な `ProviderScanResult`。
///
/// provider 名と最初の capability を保持し、`outcome` を写像値にする。エラーを握りつぶして
/// 結果から除外しないことで、一部失敗の取りこぼしを防ぐ。
fn synthesize_failure(provider: &dyn SafetyProvider, error: &ScanError) -> ProviderScanResult {
    let capability = provider
        .capabilities()
        .first()
        .copied()
        .expect("provider capabilities are validated as non-empty at build time");
    ProviderScanResult {
        provider: provider.name().to_string(),
        capability,
        outcome: map_scan_error(error),
        known_hash_match: false,
        score: None,
        labels: Vec::new(),
    }
}

/// `SafetyOrchestrator` の builder。
pub struct SafetyOrchestratorBuilder {
    providers: Vec<Arc<dyn SafetyProvider>>,
    policy: Option<SafetyPolicy>,
    issuer_node_id: String,
    clock: Arc<dyn ScanClock>,
    ids: Arc<dyn EventIdGenerator>,
}

impl SafetyOrchestratorBuilder {
    /// provider を登録順に追加する。
    pub fn provider(mut self, provider: Arc<dyn SafetyProvider>) -> Self {
        self.providers.push(provider);
        self
    }

    /// policy を指定する（未指定なら `SafetyPolicy::public_node_default()`）。
    pub fn policy(mut self, policy: SafetyPolicy) -> Self {
        self.policy = Some(policy);
        self
    }

    /// 構成を検証して `SafetyOrchestrator` を構築する。
    ///
    /// issuer node id の空、provider 不在、capability の無い provider、空 provider 名を拒否する。
    pub fn build(self) -> Result<SafetyOrchestrator, SafetyRuntimeError> {
        let issuer_node_id = self.issuer_node_id.trim().to_string();
        if issuer_node_id.is_empty() {
            return Err(SafetyRuntimeError::EmptyIssuerNodeId);
        }
        if self.providers.is_empty() {
            return Err(SafetyRuntimeError::NoProviders);
        }
        for provider in &self.providers {
            if provider.name().trim().is_empty() {
                return Err(SafetyRuntimeError::EmptyProviderName);
            }
            if provider.capabilities().is_empty() {
                return Err(SafetyRuntimeError::ProviderWithoutCapability {
                    provider: provider.name().to_string(),
                });
            }
        }

        Ok(SafetyOrchestrator {
            providers: self.providers,
            policy: self
                .policy
                .unwrap_or_else(SafetyPolicy::public_node_default),
            issuer_node_id,
            clock: self.clock,
            ids: self.ids,
        })
    }
}
