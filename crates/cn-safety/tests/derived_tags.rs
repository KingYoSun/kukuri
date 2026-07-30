//! VLM 派生検索タグの contract（ADR 0028 §2.6 / ADR 0025 §2.3、#420）。
//!
//! 受け入れ条件:
//! - タグを index するのは `allow` verdict の media のみ（exclude / hold / quarantine /
//!   fail-closed はタグ化しない）
//! - critical 検知結果 / Match Data / 生スコアの機微をタグ / index に流さない

use kukuri_cn_safety::provider::{ProviderScanResult, ScanOutcome};
use kukuri_cn_safety::verdict::{SafetyCategory, SafetyLabel};
use kukuri_cn_safety::{SafetyPolicy, SafetyProviderCapability, derived_tags_for_index, route};

const SCANNED_AT: &str = "2026-07-30T00:00:00Z";

fn known_csam_no_match() -> ProviderScanResult {
    let mut result =
        ProviderScanResult::completed("known-csam", SafetyProviderCapability::KnownCsamHashMatch);
    result.outcome = ScanOutcome::NoKnownMatch;
    result
}

fn tagged_general_result(tags: &[&str]) -> ProviderScanResult {
    let mut result = ProviderScanResult::completed(
        "vlm-general",
        SafetyProviderCapability::GeneralMediaModeration,
    );
    result.derived_tags = tags.iter().map(|t| t.to_string()).collect();
    result
}

#[test]
fn derived_tags_only_for_allow_media() {
    let policy = SafetyPolicy::public_node_default();

    // allow verdict → タグが index に載る。
    let outcomes = [
        known_csam_no_match(),
        tagged_general_result(&["sunset", "beach"]),
    ];
    let verdict = route(&outcomes, &policy, SCANNED_AT);
    assert!(verdict.is_indexable());
    assert_eq!(
        derived_tags_for_index(&verdict, &outcomes),
        vec!["sunset".to_string(), "beach".to_string()]
    );

    // 非 allow（general suspected → exclude）→ タグは空。
    let mut flagged = tagged_general_result(&["sunset", "beach"]);
    flagged.score = Some(95);
    flagged.labels = vec![SafetyLabel::new(SafetyCategory::Nsfw).with_confidence(95)];
    let outcomes = [known_csam_no_match(), flagged];
    let verdict = route(&outcomes, &policy, SCANNED_AT);
    assert!(!verdict.is_indexable());
    assert!(derived_tags_for_index(&verdict, &outcomes).is_empty());

    // fail-closed（provider unavailable）→ タグは空。
    let mut unavailable = tagged_general_result(&["sunset"]);
    unavailable.outcome = ScanOutcome::Unavailable;
    let outcomes = [known_csam_no_match(), unavailable];
    let verdict = route(&outcomes, &policy, SCANNED_AT);
    assert!(!verdict.is_indexable());
    assert!(derived_tags_for_index(&verdict, &outcomes).is_empty());
}

#[test]
fn derived_tags_exclude_critical_and_match_data() {
    let policy = SafetyPolicy::public_node_default();

    // critical capability の scan 結果のタグは、verdict が allow でも収集しない。
    // （閾値未満の critical 検知は verdict 自体が fail-closed になるため、ここでは
    //   「critical capability 由来のタグは源から遮断される」ことを直接検証する。）
    let mut critical_tagged = ProviderScanResult::completed(
        "vlm-unknown-csam",
        SafetyProviderCapability::NovelCsamImageClassifier,
    );
    critical_tagged.derived_tags = vec!["leaked-critical-tag".to_string()];
    let outcomes = [
        known_csam_no_match(),
        tagged_general_result(&["harbor"]),
        critical_tagged,
    ];
    // critical 検知（Completed + critical capability）が混ざると verdict は fail-closed。
    let verdict = route(&outcomes, &policy, SCANNED_AT);
    assert!(!verdict.is_indexable());
    assert!(derived_tags_for_index(&verdict, &outcomes).is_empty());

    // Match Data（known_hash_match=true）の結果からもタグを流さない。
    let mut match_data =
        ProviderScanResult::completed("known-csam", SafetyProviderCapability::KnownCsamHashMatch);
    match_data.known_hash_match = true;
    match_data.derived_tags = vec!["match-data-leak".to_string()];
    let outcomes = [match_data, tagged_general_result(&["harbor"])];
    let verdict = route(&outcomes, &policy, SCANNED_AT);
    assert!(!verdict.is_indexable());
    assert!(derived_tags_for_index(&verdict, &outcomes).is_empty());

    // critical label 付きの結果は、たとえ allow verdict の合成対象に紛れても収集しない
    // （is_sensitive_result の防御を、フィルタ関数単体で確認する）。
    let mut critical_label = tagged_general_result(&["grooming-context-leak"]);
    critical_label.labels = vec![SafetyLabel::new(SafetyCategory::Grooming)];
    let allow_outcomes = [known_csam_no_match(), tagged_general_result(&["harbor"])];
    let allow_verdict = route(&allow_outcomes, &policy, SCANNED_AT);
    assert!(allow_verdict.is_indexable());
    let mixed = [
        known_csam_no_match(),
        tagged_general_result(&["harbor"]),
        critical_label,
    ];
    assert_eq!(
        derived_tags_for_index(&allow_verdict, &mixed),
        vec!["harbor".to_string()]
    );

    // 生スコア風トークン（数値のみ / score / confidence）はタグ語彙の面でも遮断する。
    let outcomes = [
        known_csam_no_match(),
        tagged_general_result(&[
            "  Sunset ",
            "87",
            "0.87",
            "score=87",
            "Confidence: high",
            "sunset",
        ]),
    ];
    let verdict = route(&outcomes, &policy, SCANNED_AT);
    assert!(verdict.is_indexable());
    assert_eq!(
        derived_tags_for_index(&verdict, &outcomes),
        vec!["sunset".to_string()]
    );
}
