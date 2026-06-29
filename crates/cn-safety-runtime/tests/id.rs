//! `UuidEventIdGenerator` の本番 id contract テスト（#399）。
//!
//! 乱数 id の一意性そのものは確率的性質のため固定せず、UUID v4 として parse できること・
//! version が v4 であること・orchestrator へ注入できることを検証する。

use std::sync::Arc;

use kukuri_cn_safety::provider::{ProviderScanRequest, SubjectKind};
use kukuri_cn_safety::verdict::SafetyAction;
use kukuri_cn_safety::{MockSafetyProvider, SafetyPolicy};
use kukuri_cn_safety_runtime::{
    EventIdGenerator, SafetyOrchestrator, ScanClock, UuidEventIdGenerator,
};
use uuid::{Uuid, Version};

struct FixedClock;
impl ScanClock for FixedClock {
    fn now_rfc3339(&self) -> String {
        "2026-06-29T09:00:00Z".to_string()
    }
}

#[test]
fn next_id_is_uuid_v4() {
    let ids = UuidEventIdGenerator::new();
    let id = ids.next_id();
    let parsed = Uuid::parse_str(&id).unwrap_or_else(|err| panic!("id must be UUID: {id} ({err})"));
    assert_eq!(
        parsed.get_version(),
        Some(Version::Random),
        "expected v4: {id}"
    );
}

#[test]
fn next_id_is_unique_per_call() {
    let ids = UuidEventIdGenerator;
    assert_ne!(ids.next_id(), ids.next_id());
}

#[tokio::test]
async fn uuid_generator_can_drive_orchestrator() {
    let clock: Arc<dyn ScanClock> = Arc::new(FixedClock);
    let ids: Arc<dyn EventIdGenerator> = Arc::new(UuidEventIdGenerator::new());
    let provider =
        Arc::new(MockSafetyProvider::known_csam("known").with_known_hash_match("blob-1"));
    let orchestrator = SafetyOrchestrator::builder("node-1", clock, ids)
        .policy(SafetyPolicy::public_node_default())
        .provider(provider)
        .build()
        .unwrap();

    let request = ProviderScanRequest::for_subject(SubjectKind::Blob, "blob-1");
    let report = orchestrator.scan_subject(&request).await;

    assert_eq!(report.verdict.action, SafetyAction::Exclude);
    let event = report.moderation_event.expect("event for excluded content");
    Uuid::parse_str(&event.id)
        .unwrap_or_else(|err| panic!("event id must be UUID: {} ({err})", event.id));
}
