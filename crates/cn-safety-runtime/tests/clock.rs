//! `SystemScanClock` の本番 clock contract テスト（#398）。
//!
//! 現在時刻そのものは固定せず、RFC3339 / UTC / 秒精度の契約と orchestrator への注入可能性だけを
//! 検証する。

use std::sync::Arc;

use chrono::DateTime;
use kukuri_cn_safety::provider::{ProviderScanRequest, SubjectKind};
use kukuri_cn_safety::verdict::SafetyAction;
use kukuri_cn_safety::{MockSafetyProvider, SafetyPolicy};
use kukuri_cn_safety_runtime::{EventIdGenerator, SafetyOrchestrator, ScanClock, SystemScanClock};

#[test]
fn system_clock_returns_parseable_rfc3339() {
    let clock = SystemScanClock::new();
    let now = clock.now_rfc3339();
    let parsed = DateTime::parse_from_rfc3339(&now)
        .unwrap_or_else(|err| panic!("now_rfc3339 must be RFC3339: {now} ({err})"));
    // UTC（offset 0 秒）であること。
    assert_eq!(
        parsed.offset().local_minus_utc(),
        0,
        "expected UTC offset: {now}"
    );
}

#[test]
fn system_clock_is_utc_z_suffixed_and_second_precision() {
    let now = SystemScanClock.now_rfc3339();
    // UTC は `Z` suffix で表す（`+00:00` ではない）。
    assert!(now.ends_with('Z'), "expected Z suffix: {now}");
    // 秒精度であり fractional seconds を含まない。
    assert!(!now.contains('.'), "expected no fractional seconds: {now}");
    // `YYYY-MM-DDTHH:MM:SSZ` は 20 文字。
    assert_eq!(now.len(), 20, "expected second precision length: {now}");
}

struct SequentialIdGenerator;
impl EventIdGenerator for SequentialIdGenerator {
    fn next_id(&self) -> String {
        "evt-0".to_string()
    }
}

#[tokio::test]
async fn system_clock_can_drive_orchestrator() {
    let clock: Arc<dyn ScanClock> = Arc::new(SystemScanClock::new());
    let ids: Arc<dyn EventIdGenerator> = Arc::new(SequentialIdGenerator);
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
    // orchestrator は clock の値を scanned_at / created_at に伝播する。
    DateTime::parse_from_rfc3339(&report.verdict.scanned_at)
        .expect("verdict.scanned_at must be RFC3339");
    DateTime::parse_from_rfc3339(&event.created_at).expect("event.created_at must be RFC3339");
    assert_eq!(report.verdict.scanned_at, event.created_at);
}
