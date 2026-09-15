//! 保存済み verdict の再利用と risk signal / event の重複停止（#1050）。
//!
//! `MemorySafetyArtifactStore` と呼び出し回数を数える provider で、`scan_or_reuse` が
//! 「内容と scan 構成が不変なら provider を呼ばず artifact も増やさない」ことを DB 非依存で固定する。

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use kukuri_cn_safety::provider::{
    ProviderScanRequest, ProviderScanResult, SafetyProvider, ScanError, ScanOutcome, SubjectKind,
};
use kukuri_cn_safety::verdict::{ReasonCode, SafetyAction};
use kukuri_cn_safety::{
    AdvisorySubjectKind, GeneralAction, MockSigner, RiskSignalTarget, SafetyCategory, SafetyLabel,
    SafetyPolicy, SafetyProviderCapability, SafetyVerdict, Severity,
};
use kukuri_cn_safety_runtime::{
    EventIdGenerator, MemorySafetyArtifactStore, RescanReason, ReuseDecision, ReuseInputs,
    SafetyArtifactStore, SafetyOrchestrator, SafetyScanService, ScanClock, ScanDisposition,
    StoredVerdictRecord, compute_scan_config_fingerprint, decide_verdict_reuse,
};

const ISSUER: &str = "issuer-node";
const SCANNED_AT: &str = "2026-09-15T10:00:00Z";

struct FixedClock;
impl ScanClock for FixedClock {
    fn now_rfc3339(&self) -> String {
        SCANNED_AT.to_string()
    }
}

#[derive(Default)]
struct SequentialIds(AtomicU64);
impl EventIdGenerator for SequentialIds {
    fn next_id(&self) -> String {
        format!("evt-{}", self.0.fetch_add(1, Ordering::SeqCst))
    }
}

/// 呼び出し回数を数え、返す結果を差し替えられる provider。
struct CountingProvider {
    name: String,
    capabilities: Vec<SafetyProviderCapability>,
    result: Mutex<ProviderScanResult>,
    calls: AtomicUsize,
}

impl CountingProvider {
    fn new(
        name: &str,
        capabilities: Vec<SafetyProviderCapability>,
        result: ProviderScanResult,
    ) -> Arc<Self> {
        Arc::new(Self {
            name: name.to_string(),
            capabilities,
            result: Mutex::new(result),
            calls: AtomicUsize::new(0),
        })
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn set_result(&self, result: ProviderScanResult) {
        *self.result.lock().expect("result mutex poisoned") = result;
    }
}

#[async_trait]
impl SafetyProvider for CountingProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &[SafetyProviderCapability] {
        &self.capabilities
    }

    async fn scan(&self, _request: &ProviderScanRequest) -> Result<ProviderScanResult, ScanError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.result.lock().expect("result mutex poisoned").clone())
    }
}

fn nsfw_result(provider: &str, score: u8) -> ProviderScanResult {
    ProviderScanResult {
        provider: provider.to_string(),
        capability: SafetyProviderCapability::GeneralMediaModeration,
        outcome: ScanOutcome::Completed,
        known_hash_match: false,
        score: Some(score),
        labels: vec![
            SafetyLabel::new(SafetyCategory::Nsfw)
                .with_confidence(score)
                .with_provider_capability(SafetyProviderCapability::GeneralMediaModeration),
        ],
        derived_tags: Vec::new(),
    }
}

fn clean_result(provider: &str, tags: &[&str]) -> ProviderScanResult {
    let mut result =
        ProviderScanResult::completed(provider, SafetyProviderCapability::GeneralMediaModeration);
    result.derived_tags = tags.iter().map(|tag| tag.to_string()).collect();
    result
}

fn unavailable_result(provider: &str) -> ProviderScanResult {
    let mut result =
        ProviderScanResult::completed(provider, SafetyProviderCapability::GeneralMediaModeration);
    result.outcome = ScanOutcome::Unavailable;
    result
}

/// 再利用 / 集約の契約は「非 allow の verdict」で固定する（nsfw を exclude に厳格化した node）。
/// 既定の label（advisory 付き allow）は `reused_labeled_allow_restores_advisories_without_artifacts`。
fn general_policy() -> SafetyPolicy {
    SafetyPolicy {
        require_known_csam: false,
        general_action: GeneralAction::Exclude,
        ..SafetyPolicy::public_node_default()
    }
}

fn service_with(
    provider: Arc<CountingProvider>,
    policy: SafetyPolicy,
    store: Arc<MemorySafetyArtifactStore>,
) -> SafetyScanService {
    let orchestrator = SafetyOrchestrator::builder(
        ISSUER,
        Arc::new(FixedClock),
        Arc::new(SequentialIds::default()),
    )
    .policy(policy)
    .provider(provider)
    .build()
    .expect("orchestrator");
    SafetyScanService::builder(Arc::new(orchestrator), store)
        .signer(Arc::new(MockSigner::new(ISSUER)))
        .build()
        .expect("service")
}

fn post_request(id: &str) -> ProviderScanRequest {
    ProviderScanRequest::for_subject(SubjectKind::Post, id).with_text("sexy test")
}

fn stored(
    verdict: SafetyVerdict,
    source: Option<&str>,
    config: Option<&str>,
) -> StoredVerdictRecord {
    StoredVerdictRecord {
        id: "verdict-1".to_string(),
        verdict,
        derived_tags: Vec::new(),
        advisories: Vec::new(),
        source_fingerprint: source.map(str::to_string),
        scan_config_fingerprint: config.map(str::to_string),
    }
}

fn verdict(action: SafetyAction, reason_code: ReasonCode) -> SafetyVerdict {
    SafetyVerdict {
        action,
        labels: Vec::new(),
        advisory_labels: Vec::new(),
        critical: false,
        reason_code,
        confidence: None,
        provider: None,
        provider_capability: None,
        policy_version: "v".to_string(),
        scanned_at: SCANNED_AT.to_string(),
    }
}

// --- 純関数: 再利用判定 ---

#[test]
fn reuse_decision_requires_equal_fingerprints_and_non_held_verdict() {
    let inputs = ReuseInputs {
        source_fingerprint: "src-a",
        scan_config_fingerprint: "cfg-a",
    };
    let allow = verdict(SafetyAction::Allow, ReasonCode::NoKnownMatch);

    assert_eq!(
        decide_verdict_reuse(None, &inputs),
        ReuseDecision::Rescan(RescanReason::NoStoredVerdict)
    );
    assert_eq!(
        decide_verdict_reuse(Some(&stored(allow.clone(), None, Some("cfg-a"))), &inputs),
        ReuseDecision::Rescan(RescanReason::MissingFingerprint)
    );
    assert_eq!(
        decide_verdict_reuse(
            Some(&stored(allow.clone(), Some("src-a"), Some("cfg-b"))),
            &inputs
        ),
        ReuseDecision::Rescan(RescanReason::ScanConfigChanged)
    );
    assert_eq!(
        decide_verdict_reuse(
            Some(&stored(allow.clone(), Some("src-b"), Some("cfg-a"))),
            &inputs
        ),
        ReuseDecision::Rescan(RescanReason::SourceChanged)
    );
    assert_eq!(
        decide_verdict_reuse(Some(&stored(allow, Some("src-a"), Some("cfg-a"))), &inputs),
        ReuseDecision::Reuse
    );

    // 確定した非 allow（exclude / quarantine）は再利用してよい。
    let exclude = verdict(SafetyAction::Exclude, ReasonCode::GeneralModeration);
    assert_eq!(
        decide_verdict_reuse(
            Some(&stored(exclude, Some("src-a"), Some("cfg-a"))),
            &inputs
        ),
        ReuseDecision::Reuse
    );

    // fail-closed（scan failure / provider unavailable / unscanned / hold）は必ず再 scan する。
    for held in [
        verdict(SafetyAction::Hold, ReasonCode::ScanFailed),
        verdict(SafetyAction::Hold, ReasonCode::ProviderUnavailable),
        verdict(SafetyAction::Hold, ReasonCode::Unscanned),
        verdict(SafetyAction::Hold, ReasonCode::CsamSuspected),
    ] {
        assert_eq!(
            decide_verdict_reuse(Some(&stored(held, Some("src-a"), Some("cfg-a"))), &inputs),
            ReuseDecision::Rescan(RescanReason::HeldVerdict)
        );
    }
}

#[test]
fn config_fingerprint_changes_with_policy_and_provider_identity() {
    let provider_a: Arc<dyn SafetyProvider> = CountingProvider::new(
        "general-a",
        vec![SafetyProviderCapability::GeneralMediaModeration],
        clean_result("general-a", &[]),
    );
    let provider_b: Arc<dyn SafetyProvider> = CountingProvider::new(
        "general-b",
        vec![SafetyProviderCapability::GeneralMediaModeration],
        clean_result("general-b", &[]),
    );
    let policy = general_policy();
    let mut stricter = general_policy();
    stricter.suspected_threshold = 50;

    let providers_a = vec![provider_a];
    let base = compute_scan_config_fingerprint(&policy, &providers_a);
    assert_eq!(base, compute_scan_config_fingerprint(&policy, &providers_a));
    assert_ne!(
        base,
        compute_scan_config_fingerprint(&stricter, &providers_a)
    );
    assert_ne!(
        base,
        compute_scan_config_fingerprint(&policy, &[provider_b])
    );
    assert_ne!(base, compute_scan_config_fingerprint(&policy, &[]));
}

// --- service: scan_or_reuse ---

#[tokio::test]
async fn scan_or_reuse_skips_provider_when_fingerprints_match() {
    let provider = CountingProvider::new(
        "general",
        vec![SafetyProviderCapability::GeneralMediaModeration],
        nsfw_result("general", 84),
    );
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let service = service_with(provider.clone(), general_policy(), store.clone());

    let first = service
        .scan_or_reuse(&post_request("post-1"), Some("author-a"), "state-hash-1")
        .await
        .expect("first scan");
    assert_eq!(first.disposition, ScanDisposition::Fresh);
    assert_eq!(first.report.verdict.action, SafetyAction::Exclude);
    assert!(first.signed_event.is_some());
    assert_eq!(provider.calls(), 1);
    assert_eq!(store.signals().len(), 1);
    assert_eq!(store.events().len(), 1);

    let second = service
        .scan_or_reuse(&post_request("post-1"), Some("author-a"), "state-hash-1")
        .await
        .expect("second pass");
    assert_eq!(second.disposition, ScanDisposition::Reused);
    assert_eq!(second.report.verdict.action, SafetyAction::Exclude);
    assert_eq!(
        second.report.verdict.reason_code,
        ReasonCode::GeneralModeration
    );
    assert_eq!(second.verdict_id, first.verdict_id);
    assert!(second.signed_event.is_none());
    assert!(second.persisted_signal_id.is_none());
    assert_eq!(
        provider.calls(),
        1,
        "unchanged content must not reach the provider"
    );
    assert_eq!(store.signals().len(), 1, "no duplicate risk signal");
    assert_eq!(store.events().len(), 1, "no duplicate moderation event");
}

#[tokio::test]
async fn scan_or_reuse_restores_derived_tags_for_allow_verdicts() {
    let provider = CountingProvider::new(
        "general",
        vec![SafetyProviderCapability::GeneralMediaModeration],
        clean_result("general", &["beach", "sunset"]),
    );
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let service = service_with(provider.clone(), general_policy(), store.clone());
    let request =
        ProviderScanRequest::for_subject(SubjectKind::Blob, "blob-1").with_media_hint("blob-1");

    let first = service
        .scan_or_reuse(&request, Some("author-a"), "blob-1")
        .await
        .expect("first");
    assert!(first.report.verdict.is_indexable());
    assert_eq!(first.report.derived_tags, vec!["beach", "sunset"]);

    let second = service
        .scan_or_reuse(&request, Some("author-a"), "blob-1")
        .await
        .expect("second");
    assert_eq!(second.disposition, ScanDisposition::Reused);
    assert_eq!(second.report.derived_tags, vec!["beach", "sunset"]);
    assert_eq!(provider.calls(), 1);
    assert!(
        store.signal_subject_authors().is_empty(),
        "allow verdicts attribute nothing"
    );
}

#[tokio::test]
async fn scan_or_reuse_rescans_when_source_fingerprint_changes() {
    let provider = CountingProvider::new(
        "general",
        vec![SafetyProviderCapability::GeneralMediaModeration],
        nsfw_result("general", 84),
    );
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let service = service_with(provider.clone(), general_policy(), store.clone());

    service
        .scan_or_reuse(&post_request("post-1"), None, "state-hash-1")
        .await
        .expect("first");
    let second = service
        .scan_or_reuse(&post_request("post-1"), None, "state-hash-2")
        .await
        .expect("second");
    assert_eq!(second.disposition, ScanDisposition::Fresh);
    assert_eq!(provider.calls(), 2);
}

#[tokio::test]
async fn scan_or_reuse_rescans_when_scan_config_fingerprint_changes() {
    let provider = CountingProvider::new(
        "general",
        vec![SafetyProviderCapability::GeneralMediaModeration],
        nsfw_result("general", 84),
    );
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let service = service_with(provider.clone(), general_policy(), store.clone());
    service
        .scan_or_reuse(&post_request("post-1"), None, "state-hash-1")
        .await
        .expect("first");

    // policy（閾値）を変えた別 service = scan 構成 fingerprint が変わる。
    let mut stricter = general_policy();
    stricter.suspected_threshold = 50;
    let restarted = service_with(provider.clone(), stricter, store.clone());
    let second = restarted
        .scan_or_reuse(&post_request("post-1"), None, "state-hash-1")
        .await
        .expect("second");
    assert_eq!(second.disposition, ScanDisposition::Fresh);
    assert_eq!(provider.calls(), 2);

    // 同じ構成での 3 回目は再利用される。
    let third = restarted
        .scan_or_reuse(&post_request("post-1"), None, "state-hash-1")
        .await
        .expect("third");
    assert_eq!(third.disposition, ScanDisposition::Reused);
    assert_eq!(provider.calls(), 2);
}

#[tokio::test]
async fn held_verdict_is_never_reused() {
    let provider = CountingProvider::new(
        "general",
        vec![SafetyProviderCapability::GeneralMediaModeration],
        unavailable_result("general"),
    );
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let service = service_with(provider.clone(), general_policy(), store.clone());

    let first = service
        .scan_or_reuse(&post_request("post-1"), None, "state-hash-1")
        .await
        .expect("first");
    assert_eq!(first.report.verdict.action, SafetyAction::Hold);
    assert_eq!(
        first.report.verdict.reason_code,
        ReasonCode::ProviderUnavailable
    );

    let second = service
        .scan_or_reuse(&post_request("post-1"), None, "state-hash-1")
        .await
        .expect("second");
    assert_eq!(
        second.disposition,
        ScanDisposition::Fresh,
        "hold is retried every pass"
    );
    assert_eq!(provider.calls(), 2);

    // provider が復旧すれば allow に更新され、以後は再利用される。
    provider.set_result(clean_result("general", &[]));
    let third = service
        .scan_or_reuse(&post_request("post-1"), None, "state-hash-1")
        .await
        .expect("third");
    assert_eq!(third.disposition, ScanDisposition::Fresh);
    assert!(third.report.verdict.is_indexable());
    let fourth = service
        .scan_or_reuse(&post_request("post-1"), None, "state-hash-1")
        .await
        .expect("fourth");
    assert_eq!(fourth.disposition, ScanDisposition::Reused);
    assert_eq!(provider.calls(), 3);
}

#[tokio::test]
async fn identical_rescan_emits_no_new_event_but_verdict_change_does() {
    let provider = CountingProvider::new(
        "general",
        vec![SafetyProviderCapability::GeneralMediaModeration],
        nsfw_result("general", 84),
    );
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let service = service_with(provider.clone(), general_policy(), store.clone());

    let first = service
        .scan_or_reuse(&post_request("post-1"), Some("author-a"), "state-hash-1")
        .await
        .expect("first");
    let first_signal_id = first.persisted_signal_id.clone().expect("signal id");
    assert_eq!(store.events().len(), 1);

    // 内容は変わったが判定は同じ（nsfw / exclude）→ signal は既存行の更新、event は増えない。
    provider.set_result(nsfw_result("general", 91));
    let second = service
        .scan_or_reuse(&post_request("post-1"), Some("author-a"), "state-hash-2")
        .await
        .expect("second");
    assert_eq!(second.disposition, ScanDisposition::Fresh);
    assert_eq!(
        second.persisted_signal_id.as_deref(),
        Some(first_signal_id.as_str())
    );
    assert!(second.signed_event.is_none());
    assert_eq!(store.signals().len(), 1);
    assert_eq!(
        store.signals()[0].1.confidence,
        Some(91),
        "existing signal is refreshed"
    );
    assert_eq!(store.events().len(), 1);

    // 同じ鍵（nsfw / classifier_score）のまま action が変わる（policy で exclude → hold）
    // → signal は同じ行、event は新規発行。
    let mut hold_policy = general_policy();
    hold_policy.general_action = GeneralAction::Hold;
    let restarted = service_with(provider.clone(), hold_policy, store.clone());
    let third = restarted
        .scan_or_reuse(&post_request("post-1"), Some("author-a"), "state-hash-2")
        .await
        .expect("third");
    assert_eq!(third.report.verdict.action, SafetyAction::Hold);
    assert_eq!(
        third.persisted_signal_id.as_deref(),
        Some(first_signal_id.as_str())
    );
    assert!(third.signed_event.is_some());
    assert_eq!(store.signals().len(), 1);
    assert_eq!(store.events().len(), 2);
}

#[tokio::test]
async fn reused_non_allow_verdict_still_attributes_author() {
    let provider = CountingProvider::new(
        "general",
        vec![SafetyProviderCapability::GeneralMediaModeration],
        nsfw_result("general", 84),
    );
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let service = service_with(provider.clone(), general_policy(), store.clone());
    let request = ProviderScanRequest::for_subject(SubjectKind::Blob, "blob-shared")
        .with_media_hint("blob-shared");

    service
        .scan_or_reuse(&request, Some("author-a"), "blob-shared")
        .await
        .expect("first");
    let second = service
        .scan_or_reuse(&request, Some("author-b"), "blob-shared")
        .await
        .expect("second");
    assert_eq!(second.disposition, ScanDisposition::Reused);
    assert_eq!(provider.calls(), 1);
    assert_eq!(store.signals().len(), 1);

    let authors = store.signal_subject_authors();
    assert!(authors.contains(&(
        RiskSignalTarget::BlobCid,
        "blob-shared".to_string(),
        "author-a".to_string()
    )));
    assert!(authors.contains(&(
        RiskSignalTarget::BlobCid,
        "blob-shared".to_string(),
        "author-b".to_string()
    )));
}

#[tokio::test]
async fn cleared_signal_with_same_key_is_not_resurrected_in_memory_store() {
    let provider = CountingProvider::new(
        "general",
        vec![SafetyProviderCapability::GeneralMediaModeration],
        nsfw_result("general", 84),
    );
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let service = service_with(provider.clone(), general_policy(), store.clone());

    let first = service
        .scan_or_reuse(&post_request("post-1"), None, "state-hash-1")
        .await
        .expect("first");
    let signal_id = first.persisted_signal_id.expect("signal id");
    assert!(store.set_signal_appeal_status(&signal_id, kukuri_cn_safety::AppealStatus::Cleared));

    // 内容変化で再 scan されても、cleared 済みの鍵に新しい signal は作らない。
    let second = service
        .scan_or_reuse(&post_request("post-1"), None, "state-hash-2")
        .await
        .expect("second");
    assert_eq!(second.disposition, ScanDisposition::Fresh);
    assert_eq!(
        second.persisted_signal_id.as_deref(),
        Some(signal_id.as_str())
    );
    assert_eq!(store.signals().len(), 1);
    assert_eq!(
        store.signals()[0].1.appeal_status,
        Some(kukuri_cn_safety::AppealStatus::Cleared)
    );
}

#[tokio::test]
async fn legacy_scan_and_record_never_reuses() {
    let provider = CountingProvider::new(
        "general",
        vec![SafetyProviderCapability::GeneralMediaModeration],
        clean_result("general", &[]),
    );
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let service = service_with(provider.clone(), general_policy(), store.clone());

    service
        .scan_and_record(&post_request("post-1"))
        .await
        .expect("first");
    let second = service
        .scan_and_record(&post_request("post-1"))
        .await
        .expect("second");
    assert_eq!(second.disposition, ScanDisposition::Fresh);
    assert_eq!(provider.calls(), 2);

    // 旧 API で保存した行は source fingerprint を持たないため、scan_or_reuse でも再 scan になる。
    let third = service
        .scan_or_reuse(&post_request("post-1"), None, "state-hash-1")
        .await
        .expect("third");
    assert_eq!(third.disposition, ScanDisposition::Fresh);
    assert_eq!(provider.calls(), 3);
}

// --- #1054: ラベル付き allow の advisory は再利用時も復元され、artifact は増えない（TR-10） ---

#[tokio::test]
async fn reused_labeled_allow_restores_advisories_without_artifacts() {
    let provider = CountingProvider::new(
        "general",
        vec![SafetyProviderCapability::GeneralMediaModeration],
        nsfw_result("general", 84),
    );
    let store = Arc::new(MemorySafetyArtifactStore::new());
    // 既定の general_action = label（advisory 付き allow）。
    let mut policy = general_policy();
    policy.general_action = GeneralAction::Label;
    let service = service_with(provider.clone(), policy.clone(), store.clone());

    let first = service
        .scan_or_reuse(&post_request("post-1"), Some("author-a"), "state-hash-1")
        .await
        .expect("first scan");
    assert_eq!(first.disposition, ScanDisposition::Fresh);
    assert!(first.report.verdict.is_labeled_allow());
    // signal が先に永続化され、advisory は signal id を持つ。
    let signal_id = first.persisted_signal_id.clone().expect("signal id");
    assert_eq!(first.advisories.len(), 1);
    assert_eq!(first.advisories[0].signal_id, signal_id);
    assert_eq!(
        first.advisories[0].subject_kind,
        AdvisorySubjectKind::PostId
    );
    assert_eq!(first.advisories[0].subject_id, "post-1");
    assert_eq!(first.advisories[0].label, "adult");
    assert!(first.signed_event.is_some());
    assert_eq!(store.signals().len(), 1);
    assert_eq!(store.signals()[0].1.severity, Severity::Low);
    assert_eq!(store.events().len(), 1);
    assert_eq!(
        store.events()[0].body.action,
        kukuri_cn_safety::ModerationAction::RiskLabel
    );
    // verdict 行に advisory が保存される。
    let stored = store
        .stored_verdict_for(SubjectKind::Post, "post-1")
        .expect("stored verdict");
    assert_eq!(stored.advisories, first.advisories);

    // 2 巡目: provider 呼び出し 0、artifact 増えず、advisory は復元される。共有 subject の
    // 2 人目の著者も（ラベル付き allow は signal を持つので）trust 入力へ関連付けられる。
    let second = service
        .scan_or_reuse(&post_request("post-1"), Some("author-b"), "state-hash-1")
        .await
        .expect("second pass");
    assert_eq!(second.disposition, ScanDisposition::Reused);
    assert!(second.report.verdict.is_indexable());
    assert_eq!(second.advisories, first.advisories);
    assert!(second.signed_event.is_none());
    assert_eq!(provider.calls(), 1);
    assert_eq!(store.signals().len(), 1);
    assert_eq!(store.events().len(), 1);
    let authors = store.signal_subject_authors();
    for author in ["author-a", "author-b"] {
        assert!(
            authors.contains(&(
                RiskSignalTarget::PostId,
                "post-1".to_string(),
                author.to_string()
            )),
            "{author}: {authors:?}"
        );
    }

    // indexer が post 行へ blob 分を含む和集合を確定させても、自 subject 分だけが復元される。
    let blob_advisory = kukuri_cn_safety::ContentAdvisory {
        issuer_node_id: ISSUER.to_string(),
        subject_kind: AdvisorySubjectKind::BlobCid,
        subject_id: "blob-1".to_string(),
        category: SafetyCategory::Objectionable,
        label: "sensitive".to_string(),
        confidence: Some(80),
        signal_id: "memory-signal-9".to_string(),
        basis: kukuri_cn_safety::Basis::ClassifierScore,
    };
    let union = vec![first.advisories[0].clone(), blob_advisory.clone()];
    store
        .persist_advisories(SubjectKind::Post, "post-1", &union)
        .await
        .expect("persist advisories");
    assert_eq!(
        store
            .stored_verdict_for(SubjectKind::Post, "post-1")
            .expect("stored")
            .advisories,
        union
    );
    let third = service
        .scan_or_reuse(&post_request("post-1"), Some("author-a"), "state-hash-1")
        .await
        .expect("third pass");
    assert_eq!(third.disposition, ScanDisposition::Reused);
    assert_eq!(third.advisories, first.advisories);

    // 内容が変わって再 scan しても、同じ signal を再利用し event は増えない（verdict 不変）。
    let fourth = service
        .scan_or_reuse(&post_request("post-1"), Some("author-a"), "state-hash-2")
        .await
        .expect("fourth pass");
    assert_eq!(fourth.disposition, ScanDisposition::Fresh);
    assert_eq!(
        fourth.persisted_signal_id.as_deref(),
        Some(signal_id.as_str())
    );
    assert!(fourth.signed_event.is_none());
    assert_eq!(fourth.advisories[0].signal_id, signal_id);
    assert_eq!(store.signals().len(), 1);
    assert_eq!(store.events().len(), 1);
}
