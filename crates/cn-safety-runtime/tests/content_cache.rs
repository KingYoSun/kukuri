use async_trait::async_trait;
use kukuri_cn_safety::{
    AppealStatus, ProviderScanRequest, ProviderScanResult, SafetyCategory, SafetyLabel,
    SafetyPolicy, SafetyProvider, SafetyProviderCapability, ScanError, ScanOutcome, SubjectKind,
};
use kukuri_cn_safety_runtime::{
    MemorySafetyArtifactStore, SafetyOrchestrator, SafetyScanService, SystemScanClock,
    UuidEventIdGenerator,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

#[derive(Default)]
struct Counter {
    calls: AtomicUsize,
    fail: AtomicBool,
    multiple: AtomicBool,
}
#[async_trait]
impl SafetyProvider for Counter {
    fn name(&self) -> &str {
        "content-only-test"
    }
    fn supports_content_reuse(&self) -> bool {
        true
    }
    fn capabilities(&self) -> &[SafetyProviderCapability] {
        &[SafetyProviderCapability::GeneralMediaModeration]
    }
    async fn scan(&self, _: &ProviderScanRequest) -> Result<ProviderScanResult, ScanError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(20)).await;
        let mut result = ProviderScanResult::completed(
            self.name(),
            SafetyProviderCapability::GeneralMediaModeration,
        );
        if self.fail.load(Ordering::SeqCst) {
            result.outcome = ScanOutcome::Failed;
        } else {
            result.score = Some(80);
            result.labels = vec![SafetyLabel::new(SafetyCategory::Nsfw).with_confidence(80)];
            if self.multiple.load(Ordering::SeqCst) {
                result
                    .labels
                    .push(SafetyLabel::new(SafetyCategory::Objectionable).with_confidence(23));
            }
        }
        Ok(result)
    }
}
fn service(store: Arc<MemorySafetyArtifactStore>, provider: Arc<Counter>) -> SafetyScanService {
    let mut policy = SafetyPolicy::public_node_default();
    policy.require_known_csam = false;
    let orchestrator = SafetyOrchestrator::builder(
        "node-a",
        Arc::new(SystemScanClock::new()),
        Arc::new(UuidEventIdGenerator::new()),
    )
    .policy(policy)
    .provider(provider)
    .build()
    .expect("orchestrator");
    SafetyScanService::builder(Arc::new(orchestrator), store)
        .without_signed_events("node-a")
        .build()
        .expect("service")
}
fn post(id: &str, text: &str) -> ProviderScanRequest {
    ProviderScanRequest::for_subject(SubjectKind::Post, id).with_text(text)
}

#[tokio::test]
async fn same_text_other_post_and_restart_reuses_content_but_not_author_or_appeal() {
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let provider = Arc::new(Counter::default());
    let scan = service(store.clone(), provider.clone());
    let first = scan
        .scan_or_reuse(&post("post-1", "same text"), Some("alice"), "state-1")
        .await
        .expect("first");
    assert!(store.set_signal_appeal_status(
        first.persisted_signal_id.as_deref().expect("signal"),
        AppealStatus::Cleared
    ));
    let second = scan
        .scan_or_reuse(&post("post-2", "same text"), Some("bob"), "state-2")
        .await
        .expect("second");
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        1,
        "same body must not call provider for every post"
    );
    assert_eq!(second.advisories[0].subject_id, "post-2");
    assert_ne!(first.persisted_signal_id, second.persisted_signal_id);
    let signals = store.signals_with_ids();
    assert_eq!(
        signals
            .iter()
            .find(|(_, _, s)| s.target_id == "post-2")
            .expect("second signal")
            .2
            .appeal_status,
        Some(AppealStatus::None)
    );
    let restarted_provider = Arc::new(Counter::default());
    let restarted = service(store.clone(), restarted_provider.clone());
    restarted
        .scan_or_reuse(&post("post-3", "same text"), Some("charlie"), "state-3")
        .await
        .expect("restart");
    assert_eq!(restarted_provider.calls.load(Ordering::SeqCst), 0);
    let authors = store.signal_subject_authors();
    assert!(
        authors
            .iter()
            .any(|(_, id, author)| id == "post-2" && author == "bob")
    );
    assert!(
        !authors
            .iter()
            .any(|(_, id, author)| id == "post-2" && author == "alice")
    );
}

#[tokio::test]
async fn concurrent_misses_share_one_complete_scan() {
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let provider = Arc::new(Counter::default());
    let a = service(store.clone(), provider.clone());
    let b = service(store.clone(), provider.clone());
    let ra = post("a", "shared");
    let rb = post("b", "shared");
    let (a, b) = tokio::join!(
        a.scan_or_reuse(&ra, Some("alice"), "state-a"),
        b.scan_or_reuse(&rb, Some("bob"), "state-b")
    );
    assert!(a.expect("first").report.verdict.is_labeled_allow());
    assert!(b.expect("second").report.verdict.is_labeled_allow());
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    assert_eq!(store.signals().len(), 2);
}

#[tokio::test]
async fn failed_content_is_not_cached_and_can_recover() {
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let provider = Arc::new(Counter::default());
    provider.fail.store(true, Ordering::SeqCst);
    let scan = service(store, provider.clone());
    let request = post("post", "recover");
    let failed = scan
        .scan_or_reuse(&request, Some("alice"), "state")
        .await
        .expect("failure outcome");
    assert!(!failed.report.verdict.is_indexable());
    provider.fail.store(false, Ordering::SeqCst);
    let success = scan
        .scan_or_reuse(&request, Some("alice"), "state")
        .await
        .expect("recovery");
    assert!(success.report.verdict.is_labeled_allow());
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn each_advisory_category_has_its_own_signal_and_confidence() {
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let provider = Arc::new(Counter::default());
    provider.multiple.store(true, Ordering::SeqCst);
    let scan = service(store.clone(), provider);
    let result = scan
        .scan_or_reuse(&post("both", "two categories"), Some("alice"), "state")
        .await
        .expect("scan");
    assert_eq!(
        store.signals().len(),
        2,
        "lookup must retain both advisory categories"
    );
    assert_eq!(result.advisories.len(), 2);
    assert_ne!(
        result.advisories[0].signal_id,
        result.advisories[1].signal_id
    );
    assert_eq!(
        store
            .signals()
            .iter()
            .find(|(_, s)| s.category == SafetyCategory::Objectionable)
            .expect("objectionable")
            .1
            .confidence,
        Some(23)
    );
}
