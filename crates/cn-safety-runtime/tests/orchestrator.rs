//! cn-safety-runtime の決定論的 contract テスト（#353 段階3b）。
//!
//! `cn-safety` の mock feature（MockSafetyProvider）と、ローカル定義の固定 clock / 連番 id を
//! 使い、orchestrator の scan → route → verdict → 未署名 artifact 経路を DB 非依存で検証する。

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use kukuri_cn_safety::provider::{
    ProviderScanRequest, ProviderScanResult, SafetyProvider, ScanError, ScanOutcome, SubjectKind,
};
use kukuri_cn_safety::verdict::{ReasonCode, SafetyAction, SafetyLabel};
use kukuri_cn_safety::{
    MockSafetyProvider, RiskSignalTarget, SafetyCategory, SafetyPolicy, SafetyProviderCapability,
    Visibility,
};
use kukuri_cn_safety_runtime::{
    EventIdGenerator, SafetyOrchestrator, SafetyRuntimeError, ScanClock, map_scan_error,
};

const SCANNED_AT: &str = "2026-06-29T09:00:00Z";

struct FixedClock(&'static str);
impl ScanClock for FixedClock {
    fn now_rfc3339(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Default)]
struct SequentialIdGenerator {
    next: AtomicU64,
}
impl EventIdGenerator for SequentialIdGenerator {
    fn next_id(&self) -> String {
        let n = self.next.fetch_add(1, Ordering::SeqCst);
        format!("evt-{n}")
    }
}

fn clock() -> Arc<dyn ScanClock> {
    Arc::new(FixedClock(SCANNED_AT))
}

fn ids() -> Arc<dyn EventIdGenerator> {
    Arc::new(SequentialIdGenerator::default())
}

fn blob_request() -> ProviderScanRequest {
    ProviderScanRequest::for_subject(SubjectKind::Blob, "blob-1")
}

#[derive(Clone)]
struct StaticProvider {
    name: &'static str,
    capabilities: Vec<SafetyProviderCapability>,
    result: ProviderScanResult,
}

#[async_trait]
impl SafetyProvider for StaticProvider {
    fn name(&self) -> &str {
        self.name
    }

    fn capabilities(&self) -> &[SafetyProviderCapability] {
        &self.capabilities
    }

    async fn scan(&self, _request: &ProviderScanRequest) -> Result<ProviderScanResult, ScanError> {
        Ok(self.result.clone())
    }
}

// --- ScanError 写像 ---

#[test]
fn scan_error_maps_to_fail_closed_outcomes() {
    assert_eq!(
        map_scan_error(&ScanError::Unavailable("x".into())),
        ScanOutcome::Unavailable
    );
    assert_eq!(
        map_scan_error(&ScanError::Timeout("x".into())),
        ScanOutcome::Failed
    );
    assert_eq!(
        map_scan_error(&ScanError::Protocol("x".into())),
        ScanOutcome::Failed
    );
}

// --- build 検証 ---

#[test]
fn build_rejects_empty_issuer() {
    let provider = Arc::new(MockSafetyProvider::known_csam("known"));
    let err = SafetyOrchestrator::builder("  ", clock(), ids())
        .provider(provider)
        .build()
        .unwrap_err();
    assert_eq!(err, SafetyRuntimeError::EmptyIssuerNodeId);
}

#[test]
fn build_rejects_no_providers() {
    let err = SafetyOrchestrator::builder("node-1", clock(), ids())
        .build()
        .unwrap_err();
    assert_eq!(err, SafetyRuntimeError::NoProviders);
}

#[test]
fn build_rejects_provider_without_capability() {
    let provider = Arc::new(MockSafetyProvider::with_capabilities("empty", vec![]));
    let err = SafetyOrchestrator::builder("node-1", clock(), ids())
        .provider(provider)
        .build()
        .unwrap_err();
    assert_eq!(
        err,
        SafetyRuntimeError::ProviderWithoutCapability {
            provider: "empty".to_string()
        }
    );
}

// --- known CSAM confirmed ---

#[tokio::test]
async fn known_hash_match_is_excluded_and_emits_event_and_signal() {
    let provider =
        Arc::new(MockSafetyProvider::known_csam("known").with_known_hash_match("blob-1"));
    let orchestrator = SafetyOrchestrator::builder("node-1", clock(), ids())
        .provider(provider)
        .build()
        .unwrap();
    let report = orchestrator.scan_subject(&blob_request()).await;

    assert_eq!(report.verdict.action, SafetyAction::Exclude);
    assert_eq!(report.verdict.reason_code, ReasonCode::CsamConfirmed);
    assert!(report.verdict.critical);
    assert!(!report.verdict.is_indexable());

    let event = report.moderation_event.expect("event for excluded content");
    assert_eq!(event.id, "evt-0");
    assert_eq!(event.issuer_node_id, "node-1");
    assert_eq!(event.target_id, "blob-1");
    assert_eq!(event.created_at, SCANNED_AT);
    // confirmed は subscribed_nodes 以上が許される。
    assert_eq!(event.visibility, Visibility::SubscribedNodes);

    let signal = report.risk_signal.expect("signal for excluded content");
    assert_eq!(signal.target, RiskSignalTarget::BlobCid);
    assert_eq!(signal.category, SafetyCategory::Csam);
    assert_eq!(signal.visibility, Visibility::SubscribedNodes);
}

#[tokio::test]
async fn issuer_node_id_is_trimmed_for_events() {
    let provider =
        Arc::new(MockSafetyProvider::known_csam("known").with_known_hash_match("blob-1"));
    let orchestrator = SafetyOrchestrator::builder("  node-1  ", clock(), ids())
        .provider(provider)
        .build()
        .unwrap();
    let report = orchestrator.scan_subject(&blob_request()).await;
    assert_eq!(report.moderation_event.unwrap().issuer_node_id, "node-1");
}

// --- suspected unknown CSAM ---

#[tokio::test]
async fn suspected_unknown_csam_is_local_visibility() {
    let provider = Arc::new(
        MockSafetyProvider::with_capabilities(
            "classifier",
            vec![SafetyProviderCapability::NovelCsamImageClassifier],
        )
        .with_score(
            "blob-1",
            SafetyProviderCapability::NovelCsamImageClassifier,
            SafetyCategory::Csam,
            90,
        ),
    );
    // known CSAM provider が無いと require_known_csam で fail-closed になるため、
    // policy 側の require_known_csam を外して suspected 経路を検証する。
    let mut policy = SafetyPolicy::public_node_default();
    policy.require_known_csam = false;
    let orchestrator = SafetyOrchestrator::builder("node-1", clock(), ids())
        .policy(policy)
        .provider(provider)
        .build()
        .unwrap();
    let report = orchestrator.scan_subject(&blob_request()).await;

    assert_eq!(report.verdict.reason_code, ReasonCode::CsamSuspected);
    assert!(report.verdict.critical);
    assert!(!report.verdict.is_indexable());

    let signal = report.risk_signal.expect("signal for suspected content");
    // suspected は誤検知拡散防止のため local 既定。
    assert_eq!(signal.visibility, Visibility::Local);
}

#[tokio::test]
async fn cse_suspected_artifacts_do_not_use_first_noncritical_label() {
    let result = ProviderScanResult {
        provider: "cse-classifier".to_string(),
        capability: SafetyProviderCapability::CseTextClassifier,
        outcome: ScanOutcome::Completed,
        known_hash_match: false,
        score: Some(90),
        labels: vec![
            SafetyLabel::new(SafetyCategory::Nsfw).with_confidence(90),
            SafetyLabel::new(SafetyCategory::Cse).with_confidence(90),
        ],
    };
    let provider = Arc::new(StaticProvider {
        name: "cse-classifier",
        capabilities: vec![SafetyProviderCapability::CseTextClassifier],
        result,
    });
    let mut policy = SafetyPolicy::public_node_default();
    policy.require_known_csam = false;
    let orchestrator = SafetyOrchestrator::builder("node-1", clock(), ids())
        .policy(policy)
        .provider(provider)
        .build()
        .unwrap();
    let report = orchestrator.scan_subject(&blob_request()).await;

    assert_eq!(report.verdict.reason_code, ReasonCode::CseSuspected);
    let signal = report
        .risk_signal
        .expect("signal for CSE suspected content");
    assert_eq!(signal.category, SafetyCategory::Cse);
    assert_eq!(signal.visibility, Visibility::Local);
}

// --- scan failure / unavailable fail-closed ---

#[tokio::test]
async fn scan_failure_fails_closed_without_risk_signal() {
    // known CSAM provider が timeout → Failed に写像され fail-closed。
    let provider = Arc::new(
        MockSafetyProvider::known_csam("known")
            .default_error(ScanError::Timeout("deadline".into())),
    );
    let orchestrator = SafetyOrchestrator::builder("node-1", clock(), ids())
        .provider(provider)
        .build()
        .unwrap();
    let report = orchestrator.scan_subject(&blob_request()).await;

    assert_eq!(report.verdict.reason_code, ReasonCode::ScanFailed);
    assert_ne!(report.verdict.action, SafetyAction::Allow);
    assert!(!report.verdict.is_indexable());
    // scan failure は content category を示さないため risk signal は作らない。
    assert!(report.risk_signal.is_none());
    // fail-closed の moderation event は作る（target が揃っているため）。
    let event = report.moderation_event.expect("event for held content");
    assert_eq!(event.reason_code, ReasonCode::ScanFailed);
    assert_eq!(event.visibility, Visibility::Local);
}

#[tokio::test]
async fn provider_unavailable_fails_closed() {
    let provider = Arc::new(
        MockSafetyProvider::known_csam("known")
            .default_error(ScanError::Unavailable("down".into())),
    );
    let orchestrator = SafetyOrchestrator::builder("node-1", clock(), ids())
        .provider(provider)
        .build()
        .unwrap();
    let report = orchestrator.scan_subject(&blob_request()).await;

    assert_eq!(report.verdict.reason_code, ReasonCode::ProviderUnavailable);
    assert!(!report.verdict.is_indexable());
    let mapped = &report.scan_results[0];
    assert_eq!(mapped.outcome, ScanOutcome::Unavailable);
}

// --- 複数 provider 集約: confirmed を最優先 ---

#[tokio::test]
async fn confirmed_takes_priority_over_other_provider_failure() {
    let known = Arc::new(MockSafetyProvider::known_csam("known").with_known_hash_match("blob-1"));
    let classifier = Arc::new(
        MockSafetyProvider::with_capabilities(
            "classifier",
            vec![SafetyProviderCapability::NovelCsamImageClassifier],
        )
        .default_error(ScanError::Timeout("late".into())),
    );
    let orchestrator = SafetyOrchestrator::builder("node-1", clock(), ids())
        .provider(known)
        .provider(classifier)
        .build()
        .unwrap();
    let report = orchestrator.scan_subject(&blob_request()).await;

    // confirmed が最優先される。
    assert_eq!(report.verdict.action, SafetyAction::Exclude);
    assert_eq!(report.verdict.reason_code, ReasonCode::CsamConfirmed);
    // 2 provider 分の結果が集約される（失敗も除外せず保持）。
    assert_eq!(report.scan_results.len(), 2);
    assert!(
        report
            .scan_results
            .iter()
            .any(|r| r.outcome == ScanOutcome::Failed)
    );
}

// --- known CSAM provider 欠落 fail-closed ---

#[tokio::test]
async fn missing_known_csam_provider_fails_closed() {
    // general provider のみ、clean 相当を返す。require_known_csam=true（既定）。
    let provider = Arc::new(
        MockSafetyProvider::with_capabilities(
            "general",
            vec![SafetyProviderCapability::GeneralMediaModeration],
        )
        .with_no_known_match("blob-1"),
    );
    let orchestrator = SafetyOrchestrator::builder("node-1", clock(), ids())
        .provider(provider)
        .build()
        .unwrap();
    let report = orchestrator.scan_subject(&blob_request()).await;

    assert!(!report.verdict.is_indexable());
    assert_eq!(report.verdict.reason_code, ReasonCode::ProviderUnavailable);
}

// --- clean / indexable は artifact なし ---

#[tokio::test]
async fn clean_allow_emits_no_artifacts() {
    // known CSAM provider が NoKnownMatch を返し、policy が require_known_csam=false なら allow。
    let provider = Arc::new(MockSafetyProvider::known_csam("known").with_no_known_match("blob-1"));
    let mut policy = SafetyPolicy::public_node_default();
    policy.require_known_csam = false;
    let orchestrator = SafetyOrchestrator::builder("node-1", clock(), ids())
        .policy(policy)
        .provider(provider)
        .build()
        .unwrap();
    let report = orchestrator.scan_subject(&blob_request()).await;

    assert_eq!(report.verdict.action, SafetyAction::Allow);
    // NoKnownMatch は Clean ではない reason_code を維持する。
    assert_eq!(report.verdict.reason_code, ReasonCode::NoKnownMatch);
    assert!(report.verdict.is_indexable());
    assert!(report.moderation_event.is_none());
    assert!(report.risk_signal.is_none());
}

// --- target 欠落時は artifact を作らない ---

#[tokio::test]
async fn missing_target_emits_no_artifacts_but_still_fails_closed() {
    let provider = Arc::new(
        MockSafetyProvider::known_csam("known").default_error(ScanError::Timeout("x".into())),
    );
    let orchestrator = SafetyOrchestrator::builder("node-1", clock(), ids())
        .provider(provider)
        .build()
        .unwrap();
    // subject_id / subject_kind の無い request。
    let report = orchestrator.scan_subject(&ProviderScanRequest::new()).await;

    assert!(!report.verdict.is_indexable());
    assert!(report.moderation_event.is_none());
    assert!(report.risk_signal.is_none());
}

#[tokio::test]
async fn empty_target_id_emits_no_artifacts() {
    let provider = Arc::new(MockSafetyProvider::known_csam("known").with_known_hash_match(""));
    let orchestrator = SafetyOrchestrator::builder("node-1", clock(), ids())
        .provider(provider)
        .build()
        .unwrap();
    let request = ProviderScanRequest::for_subject(SubjectKind::Blob, "");
    let report = orchestrator.scan_subject(&request).await;

    assert!(!report.verdict.is_indexable());
    assert!(report.moderation_event.is_none());
    assert!(report.risk_signal.is_none());
}

// --- 決定論的 id / clock ---

#[tokio::test]
async fn event_ids_are_sequential_and_deterministic() {
    let provider =
        Arc::new(MockSafetyProvider::known_csam("known").with_known_hash_match("blob-1"));
    let orchestrator = SafetyOrchestrator::builder("node-1", clock(), ids())
        .provider(provider)
        .build()
        .unwrap();

    let first = orchestrator.scan_subject(&blob_request()).await;
    let second = orchestrator.scan_subject(&blob_request()).await;
    assert_eq!(first.moderation_event.unwrap().id, "evt-0");
    assert_eq!(second.moderation_event.unwrap().id, "evt-1");
}

// --- serde round-trip ---

#[tokio::test]
async fn report_round_trips_snake_case() {
    let provider =
        Arc::new(MockSafetyProvider::known_csam("known").with_known_hash_match("blob-1"));
    let orchestrator = SafetyOrchestrator::builder("node-1", clock(), ids())
        .provider(provider)
        .build()
        .unwrap();
    let report = orchestrator.scan_subject(&blob_request()).await;

    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(value["verdict"]["action"], "exclude");
    assert_eq!(value["moderation_event"]["action"], "exclude");
    assert_eq!(value["risk_signal"]["target"], "blob_cid");

    let back: kukuri_cn_safety_runtime::SafetyScanReport = serde_json::from_value(value).unwrap();
    assert_eq!(back, report);
}
