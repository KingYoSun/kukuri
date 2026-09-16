//! #1060: provider category flags are decisions; confidence is not a second threshold.
use kukuri_cn_safety::{ProviderScanResult, SafetyAction, SafetyCategory, SafetyPolicy, route};

fn result(category: &str, confidence: u8, decision_basis: &str) -> ProviderScanResult {
    serde_json::from_value(serde_json::json!({
        "provider": "openai-moderation",
        "capability": "general_media_moderation",
        "outcome": "completed",
        "decision_basis": decision_basis,
        "labels": [{"category": category, "confidence": confidence}]
    }))
    .expect("provider result fixture")
}

fn policy() -> SafetyPolicy {
    let mut policy = SafetyPolicy::public_node_default();
    policy.require_known_csam = false;
    policy
}

#[test]
fn category_boolean_survives_low_confidence() {
    let verdict = route(&[result("nsfw", 2, "category_flags")], &policy(), "now");
    assert!(verdict.is_labeled_allow());
    assert_eq!(verdict.advisory_labels[0].category, SafetyCategory::Nsfw);
    assert_eq!(verdict.advisory_labels[0].confidence, Some(2));
    assert_eq!(verdict.confidence, Some(2));
}

#[test]
fn legacy_score_threshold_is_unchanged() {
    let verdict = route(&[result("nsfw", 2, "score_threshold")], &policy(), "now");
    assert!(verdict.is_indexable());
    assert!(verdict.advisory_labels.is_empty());
}

#[test]
fn boolean_critical_uses_suspected_route_without_confidence_inflation() {
    let verdict = route(&[result("cse", 2, "category_flags")], &policy(), "now");
    assert_eq!(verdict.action, SafetyAction::Quarantine);
    assert!(verdict.critical);
    assert_eq!(verdict.confidence, Some(2));
}

#[test]
fn boolean_detection_does_not_hide_failed_scan() {
    let mut failed = result("objectionable", 90, "category_flags");
    failed.outcome = kukuri_cn_safety::ScanOutcome::Failed;
    let verdict = route(
        &[result("nsfw", 2, "category_flags"), failed],
        &policy(),
        "now",
    );
    assert!(!verdict.is_indexable());
}

#[test]
fn category_flags_never_borrow_another_categorys_aggregate_score() {
    let mut general = result("nsfw", 2, "category_flags");
    general.score = Some(99);
    let verdict = route(&[general], &policy(), "now");
    assert_eq!(verdict.confidence, Some(2));
    let mut critical = result("cse", 2, "category_flags");
    critical.score = Some(99);
    let verdict = route(&[critical], &policy(), "now");
    assert_eq!(verdict.confidence, Some(2));
}
