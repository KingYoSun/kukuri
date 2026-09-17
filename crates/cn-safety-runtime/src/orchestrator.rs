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
use sha2::{Digest, Sha256};

use kukuri_cn_safety::provider::{ProviderScanRequest, ProviderScanResult, ScanError, ScanOutcome};
use kukuri_cn_safety::{
    ModerationEventBody, SafetyPolicy, SafetyProvider, SafetyRiskSignal, SafetyVerdict,
    derived_tags_for_index, route,
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
    /// index に載せてよい descriptive 検索タグ（ADR 0028 §2.6）。
    ///
    /// `derived_tags_for_index` を適用済みの値（`allow` verdict のみ非空。critical /
    /// Match Data / 生スコア由来は除外済み）。indexer はこの値をそのまま使う。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub derived_tags: Vec<String>,
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

/// policy と provider 構成から scan 構成の fingerprint を導く（#1050）。
///
/// `SafetyPolicy` の serde 表現（policy_version・閾値・各 action を含む）と、登録順の
/// provider `config_fingerprint()` を連結して sha256 にする。いずれかが変われば保存済み
/// verdict は再利用されない。
pub fn compute_scan_config_fingerprint(
    policy: &SafetyPolicy,
    providers: &[Arc<dyn SafetyProvider>],
) -> String {
    let mut hasher = Sha256::new();
    let policy_json = serde_json::to_string(policy).expect("SafetyPolicy is serializable");
    hasher.update(policy_json.as_bytes());
    for provider in providers {
        hasher.update(b"\n");
        hasher.update(provider.config_fingerprint().as_bytes());
    }
    hex::encode(hasher.finalize())
}

/// safety provider を駆動し verdict / artifact を組み立てる orchestrator。
pub struct SafetyOrchestrator {
    providers: Vec<Arc<dyn SafetyProvider>>,
    policy: SafetyPolicy,
    issuer_node_id: String,
    clock: Arc<dyn ScanClock>,
    ids: Arc<dyn EventIdGenerator>,
    scan_config_fingerprint: String,
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

    /// 構築時に確定した scan 構成の fingerprint（保存済み verdict の再利用鍵の一部。#1050）。
    pub fn scan_config_fingerprint(&self) -> &str {
        &self.scan_config_fingerprint
    }

    /// 適用中の policy。
    pub fn policy(&self) -> &SafetyPolicy {
        &self.policy
    }

    /// 単一 subject を scan して verdict / 未署名 artifact を返す。
    ///
    /// providers を登録順に逐次実行し、`Err` は `map_scan_error` で `ProviderScanResult` に
    /// 合成する。集約結果を `route()` に渡して verdict を得て、未署名 moderation artifact を
    /// 生成する。
    pub async fn scan_subject(&self, request: &ProviderScanRequest) -> SafetyScanReport {
        self.scan_subject_guarded(request, None).await
    }

    pub async fn scan_subject_guarded(
        &self,
        request: &ProviderScanRequest,
        guard: Option<&dyn crate::ScanReferenceGuard>,
    ) -> SafetyScanReport {
        let mut scan_results = Vec::with_capacity(self.providers.len());
        for provider in &self.providers {
            let result = match guard {
                Some(guard) => provider.scan_guarded(request, guard).await,
                None => provider.scan(request).await,
            };
            match result {
                Ok(result) => scan_results.push(result),
                Err(error) => scan_results.push(synthesize_failure(provider.as_ref(), &error)),
            }
        }

        self.report_from_results(request, scan_results)
    }

    pub fn supports_content_reuse(&self) -> bool {
        self.providers
            .iter()
            .all(|provider| provider.supports_content_reuse())
    }

    pub fn moderation_metrics(&self) -> Option<Arc<kukuri_cn_safety::metrics::ModerationMetrics>> {
        self.providers
            .iter()
            .find_map(|provider| provider.moderation_metrics())
    }

    pub fn cached_results_match(
        &self,
        request: &ProviderScanRequest,
        results: &[ProviderScanResult],
    ) -> bool {
        use kukuri_cn_safety::{ProviderDecisionBasis, ScanInputKind};
        results.len() == self.providers.len()
            && results
                .iter()
                .zip(&self.providers)
                .all(|(result, provider)| {
                    if result.provider != provider.name()
                        || !provider.capabilities().contains(&result.capability)
                        || result.outcome.is_fail_closed()
                    {
                        return false;
                    }
                    if result.decision_basis == ProviderDecisionBasis::CategoryFlags {
                        let Some(coverage) = &result.coverage else {
                            return false;
                        };
                        if result.known_hash_match
                            || coverage.audio_scanned
                            || coverage.evaluated_categories.is_empty()
                            || coverage.preprocessing_version.is_empty()
                            || coverage
                                .evaluated_categories
                                .iter()
                                .any(|category| coverage.unsupported_categories.contains(category))
                        {
                            return false;
                        }
                        match coverage.input {
                            ScanInputKind::Text => {
                                request.media_hint.is_none() && coverage.frames == 0
                            }
                            ScanInputKind::Image => {
                                request.media_hint.is_some() && coverage.frames == 1
                            }
                            ScanInputKind::Video => {
                                request.media_hint.is_some()
                                    && (1..=8).contains(&coverage.frames)
                                    && coverage
                                        .duration_ms
                                        .is_some_and(|duration| duration > 0 && duration <= 600_000)
                            }
                        }
                    } else {
                        true
                    }
                })
    }

    pub fn failed_report(
        &self,
        request: &ProviderScanRequest,
        error: &ScanError,
    ) -> SafetyScanReport {
        self.report_from_results(
            request,
            self.providers
                .iter()
                .map(|provider| synthesize_failure(provider.as_ref(), error))
                .collect(),
        )
    }

    pub fn report_from_results(
        &self,
        request: &ProviderScanRequest,
        scan_results: Vec<ProviderScanResult>,
    ) -> SafetyScanReport {
        let verdict = route(&scan_results, &self.policy, self.clock.now_rfc3339());
        let (moderation_event, risk_signal) = build_artifacts(
            &verdict,
            request,
            &self.issuer_node_id,
            self.ids.as_ref(),
            &self.policy,
        );
        let derived_tags = derived_tags_for_index(&verdict, &scan_results);

        SafetyScanReport {
            verdict,
            scan_results,
            moderation_event,
            risk_signal,
            derived_tags,
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
        decision_basis: Default::default(),
        coverage: None,
        provider: provider.name().to_string(),
        capability,
        outcome: map_scan_error(error),
        known_hash_match: false,
        score: None,
        labels: Vec::new(),
        derived_tags: Vec::new(),
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

        let policy = self
            .policy
            .unwrap_or_else(SafetyPolicy::public_node_default);
        let scan_config_fingerprint = compute_scan_config_fingerprint(&policy, &self.providers);
        // Both the subject shortcut and shared content cache belong to this issuer.
        // A signing-identity rotation must not reuse another node's stored advisories.
        let scan_config_fingerprint = hex::encode(Sha256::digest(
            serde_json::to_vec(&["node-scan-v1", &issuer_node_id, &scan_config_fingerprint])
                .expect("scan identity fields"),
        ));
        Ok(SafetyOrchestrator {
            providers: self.providers,
            policy,
            issuer_node_id,
            clock: self.clock,
            ids: self.ids,
            scan_config_fingerprint,
        })
    }
}
