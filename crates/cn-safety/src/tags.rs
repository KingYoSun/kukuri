//! VLM 派生の descriptive 検索タグ（ADR 0028 §2.6 / ADR 0025 §2.3、#420）。
//!
//! 同一 VLM scan が moderation verdict と検索タグの両方を生成する（二重スキャンしない）。
//! タグを index してよいのは **`allow` verdict の media のみ**。exclude / hold / quarantine と
//! すべての fail-closed 経路ではタグ化・index しない。critical 検知結果・Match Data・
//! 生スコアの機微はタグ / index に流さない。

use crate::provider::ProviderScanResult;
use crate::verdict::SafetyVerdict;

/// verdict と scan 結果群から、index に載せてよい検索タグを導く。
///
/// 規則（ADR 0028 §2.6）:
/// 1. `verdict.is_indexable()`（= `allow`）でなければ常に空。
/// 2. critical capability / critical label / `known_hash_match` を持つ scan 結果のタグは
///    収集しない（critical 検知結果と Match Data を index へ流さない）。
/// 3. タグは正規化（trim / 小文字化 / 重複除去）し、数値のみのトークンや score / confidence を
///    名乗るトークンを落とす（生スコアの混入をタグ語彙の面でも遮断する防御）。
pub fn derived_tags_for_index(
    verdict: &SafetyVerdict,
    scan_results: &[ProviderScanResult],
) -> Vec<String> {
    if !verdict.is_indexable() {
        return Vec::new();
    }
    let mut tags: Vec<String> = Vec::new();
    for result in scan_results {
        if is_sensitive_result(result) {
            continue;
        }
        for tag in &result.derived_tags {
            let Some(normalized) = normalize_tag(tag) else {
                continue;
            };
            if !tags.contains(&normalized) {
                tags.push(normalized);
            }
        }
    }
    tags
}

/// タグを収集してはならない scan 結果か（critical 検知 / Match Data）。
fn is_sensitive_result(result: &ProviderScanResult) -> bool {
    result.known_hash_match
        || result.capability.is_critical_safety()
        || result
            .labels
            .iter()
            .any(|l| l.category.is_critical_safety())
}

/// タグ 1 個の正規化。index に載せられないトークンは `None`。
fn normalize_tag(tag: &str) -> Option<String> {
    let normalized = tag.trim().to_lowercase();
    if normalized.is_empty() {
        return None;
    }
    // 数値のみ（"87" / "0.87" / "87%" 等）は生スコアの疑いがあるため落とす。
    if normalized
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '%'))
    {
        return None;
    }
    // score / confidence を名乗るトークン（"score=87" / "confidence: high" 等）も落とす。
    if normalized.contains("score") || normalized.contains("confidence") {
        return None;
    }
    Some(normalized)
}
