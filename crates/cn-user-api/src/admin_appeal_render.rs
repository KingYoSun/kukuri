//! 異議申し立て審査の一覧と確認画面の表示。

use kukuri_cn_core::{
    AppealReview, AppealReviewOperation, RiskSignalCorrection, RiskSignalMetadataEdit,
};

use crate::admin::{escape_html, option, render_simple_page};

pub(crate) fn render_appeal_reviews(
    reviews: &[AppealReview],
    write_enabled: bool,
    csrf_token: &str,
) -> String {
    if reviews.is_empty() {
        return "<p>審査中の異議申し立てはありません。</p>".to_string();
    }
    let boundary = if write_enabled {
        String::new()
    } else {
        "<p class=\"boundary\">現在は参照専用です。変更には <code>COMMUNITY_NODE_ADMIN_ACTOR</code> と <code>COMMUNITY_NODE_SAFETY_OPERATOR_REVIEW</code> の両方が必要です。</p>".to_string()
    };
    let items = reviews
        .iter()
        .map(|review| {
            let reports = review
                .reports
                .iter()
                .map(|report| {
                    format!(
                        "<li><code>{}</code>・{}・{}</li>",
                        escape_html(&report.id),
                        escape_html(&report.created_at.to_rfc3339()),
                        escape_html(report.details.as_deref().unwrap_or("説明なし")),
                    )
                })
                .collect::<String>();
            let actions = if write_enabled {
                render_appeal_action_forms(review, csrf_token)
            } else {
                "<p class=\"meta\">変更操作は無効です。</p>".to_string()
            };
            format!(
                r#"<article class="appeal-card"><h3>リスク判定 <code>{}</code></h3>
<dl class="appeal-grid"><dt>対象</dt><dd><code>{}/{}</code></dd><dt>発行元</dt><dd><code>{}</code></dd><dt>分類</dt><dd>{}</dd><dt>深刻度</dt><dd>{}</dd><dt>根拠</dt><dd>{}</dd><dt>確信度</dt><dd>{}</dd><dt>公開範囲</dt><dd>{}</dd><dt>失効時刻</dt><dd>{}</dd><dt>状態</dt><dd>{}</dd></dl>
<h4>申し立て内容</h4><ul>{}</ul>{}</article>"#,
                escape_html(&review.risk_signal_id),
                escape_html(&review.target),
                escape_html(&review.target_id),
                escape_html(&review.issuer_node_id),
                escape_html(&review.category),
                escape_html(&review.severity),
                escape_html(&review.basis),
                review
                    .confidence
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "なし".to_string()),
                escape_html(&review.visibility),
                escape_html(review.expires_at.as_deref().unwrap_or("なし")),
                escape_html(&review.appeal_status),
                reports,
                actions,
            )
        })
        .collect::<String>();
    format!("{boundary}{items}")
}

fn render_appeal_action_forms(review: &AppealReview, csrf_token: &str) -> String {
    let common = format!(
        r#"<input type="hidden" name="csrf_token" value="{}"><input type="hidden" name="target_id" value="{}">"#,
        escape_html(csrf_token),
        escape_html(&review.risk_signal_id),
    );
    let category = enum_options(
        &[
            "csam",
            "cse",
            "grooming",
            "nsfw",
            "spam",
            "malware",
            "phishing",
            "provider_test",
        ],
        &review.category,
    );
    let severity = enum_options(&["critical", "high", "medium", "low"], &review.severity);
    let visibility = enum_options(&["local", "subscribed_nodes", "public"], &review.visibility);
    let confidence = review
        .confidence
        .map(|value| value.to_string())
        .unwrap_or_default();
    let expires_at = review.expires_at.as_deref().unwrap_or_default();
    format!(
        r#"<div class="review-actions">
<form method="post" action="/actions/preview">{common}<input type="hidden" name="action" value="appeal.accept"><button type="submit">認容を確認</button></form>
<form method="post" action="/actions/preview">{common}<input type="hidden" name="action" value="appeal.reject"><button class="secondary" type="submit">棄却を確認</button></form>
<form method="post" action="/actions/preview" class="edit-form">{common}<input type="hidden" name="action" value="appeal.edit"><strong>検知情報を調整</strong><label>分類<select name="category">{category}</select></label><label>深刻度<select name="severity">{severity}</select></label><label>確信度<input name="confidence" type="number" min="0" max="100" value="{confidence}"></label><label>失効時刻<input name="expires_at" value="{expires_at}"></label><button class="secondary" type="submit">調整内容を確認</button></form>
<form method="post" action="/actions/preview" class="edit-form">{common}<input type="hidden" name="action" value="appeal.reissue"><strong>訂正版を再発行</strong><label>分類<select name="category">{category}</select></label><label>深刻度<select name="severity">{severity}</select></label><label>確信度<input name="confidence" type="number" min="0" max="100" value="{confidence}"></label><label>公開範囲<select name="visibility">{visibility}</select></label><button class="secondary" type="submit">再発行内容を確認</button></form>
</div>"#,
        common = common,
        category = category,
        severity = severity,
        confidence = escape_html(&confidence),
        expires_at = escape_html(expires_at),
        visibility = visibility,
    )
}

fn enum_options(values: &[&str], current: &str) -> String {
    values
        .iter()
        .map(|value| option(value, value, *value == current))
        .collect()
}

/// 審査操作の確認画面(#680 / #701)。
///
/// 表示値と隠し入力を同じ解析済み操作から生成し、確認画面に見えた値と適用時に
/// 再解析される値の出所を一致させる(ADR 0029 の「変更前後・影響・実行者を表示」)。
pub(crate) fn render_appeal_preview(
    csrf_token: &str,
    actor: &str,
    operation: &AppealReviewOperation,
    review: &AppealReview,
) -> String {
    let (action, title, impact, changes) = match operation {
        AppealReviewOperation::Accept { .. } => (
            "appeal.accept",
            "異議申し立てを認容しますか",
            "このリスク判定を Cleared にし、関連通報を処理済みにします。次回の信頼評価から、この判定の寄与は除外されます。",
            None,
        ),
        AppealReviewOperation::Reject { .. } => (
            "appeal.reject",
            "異議申し立てを棄却しますか",
            "このリスク判定を None に戻し、関連通報を棄却済みにします。信頼評価への寄与は残ります。",
            None,
        ),
        AppealReviewOperation::Edit { edit, .. } => (
            "appeal.edit",
            "検知情報を調整しますか",
            "署名済みモデレーション事象と利用者の正本状態は変更せず、このノードのリスク判定だけを調整します。異議申し立ては審査中のままです。",
            Some(render_change_list(&edit_changes(edit, review))),
        ),
        AppealReviewOperation::Reissue { correction, .. } => (
            "appeal.reissue",
            "訂正版を再発行しますか",
            "現在のリスク判定を認容(信頼評価への寄与なし)として終結させたまま根拠一覧に残し、指定した検知情報と公開範囲で新しいリスク判定を発行します。新しいリスク判定の失効時刻は未設定になります。関連通報は処理済みになります。",
            Some(render_change_list(&reissue_changes(correction, review))),
        ),
    };
    let expected = match operation {
        AppealReviewOperation::Accept { expected }
        | AppealReviewOperation::Reject { expected }
        | AppealReviewOperation::Edit { expected, .. }
        | AppealReviewOperation::Reissue { expected, .. } => expected,
    };
    let expected_state = serde_json::to_string(expected).expect("appeal review version serializes");
    let body = format!(
        r#"<div class="dialog"><p class="meta">適用前の確認</p><h1>{}</h1><dl><dt>運営者</dt><dd><code>{}</code></dd><dt>リスク判定</dt><dd><code>{}</code></dd><dt>対象</dt><dd><code>{}/{}</code></dd><dt>現在の状態</dt><dd>{}</dd>{}<dt>影響</dt><dd>{}</dd></dl><p class="boundary">適用時に現在値をもう一度取得し、確認後に変更されていた場合は拒否します。リスク判定、関連通報、操作記録は一つの取引で確定します。</p><form method="post" action="/actions/apply"><input type="hidden" name="csrf_token" value="{}"><input type="hidden" name="action" value="{}"><input type="hidden" name="target_id" value="{}"><input type="hidden" name="expected_state" value="{}">{}<button type="submit">確認して適用</button> <a href="/">取り消す</a></form></div>"#,
        escape_html(title),
        escape_html(actor),
        escape_html(&review.risk_signal_id),
        escape_html(&review.target),
        escape_html(&review.target_id),
        escape_html(&review.appeal_status),
        changes.unwrap_or_default(),
        escape_html(impact),
        escape_html(csrf_token),
        escape_html(action),
        escape_html(&review.risk_signal_id),
        escape_html(&expected_state),
        hidden_metadata_fields(operation),
    );
    render_simple_page("異議申し立て審査の確認", body.as_str())
}

/// 変更前後表示の 1 項目。`after` が `None`(未入力)の項目は現在値を維持する。
struct FieldChange {
    label: &'static str,
    before: String,
    after: Option<String>,
}

fn edit_changes(edit: &RiskSignalMetadataEdit, review: &AppealReview) -> Vec<FieldChange> {
    vec![
        FieldChange {
            label: "分類",
            before: review.category.clone(),
            after: edit.category.map(|value| enum_text(&value)),
        },
        FieldChange {
            label: "深刻度",
            before: review.severity.clone(),
            after: edit.severity.map(|value| enum_text(&value)),
        },
        FieldChange {
            label: "確信度",
            before: optional_number_text(review.confidence),
            after: edit.confidence.map(|value| value.to_string()),
        },
        FieldChange {
            label: "失効時刻",
            before: optional_text(review.expires_at.as_deref()),
            after: edit.expires_at.clone(),
        },
    ]
}

fn reissue_changes(correction: &RiskSignalCorrection, review: &AppealReview) -> Vec<FieldChange> {
    vec![
        FieldChange {
            label: "分類",
            before: review.category.clone(),
            after: correction.category.map(|value| enum_text(&value)),
        },
        FieldChange {
            label: "深刻度",
            before: review.severity.clone(),
            after: correction.severity.map(|value| enum_text(&value)),
        },
        FieldChange {
            label: "確信度",
            before: optional_number_text(review.confidence),
            after: correction.confidence.map(|value| value.to_string()),
        },
        FieldChange {
            label: "公開範囲",
            before: review.visibility.clone(),
            after: correction.visibility.map(|value| enum_text(&value)),
        },
    ]
}

/// 変更前後の一覧。調整フォームは現在値で事前入力されるため、入力があっても
/// 確認時点の現在値と同じなら「維持」と表示する(#701)。
fn render_change_list(changes: &[FieldChange]) -> String {
    let items = changes
        .iter()
        .map(|change| {
            match change
                .after
                .as_deref()
                .filter(|after| *after != change.before)
            {
                Some(after) => format!(
                    "<li>{}: <code>{}</code> → <code>{}</code></li>",
                    change.label,
                    escape_html(&change.before),
                    escape_html(after),
                ),
                None => format!(
                    "<li>{}: <code>{}</code>(維持)</li>",
                    change.label,
                    escape_html(&change.before),
                ),
            }
        })
        .collect::<String>();
    format!("<dt>変更内容(変更前は確認時点の審査情報)</dt><dd><ul>{items}</ul></dd>")
}

/// 適用フォームへ写す検知情報の隠し入力。解析済み操作から生成し、未変更(None)は
/// 空欄として写す(適用時の再解析で None に戻る)。
fn hidden_metadata_fields(operation: &AppealReviewOperation) -> String {
    let (category, severity, confidence, expires_at, visibility) = match operation {
        AppealReviewOperation::Edit { edit, .. } => (
            edit.category.map(|value| enum_text(&value)),
            edit.severity.map(|value| enum_text(&value)),
            edit.confidence.map(|value| value.to_string()),
            edit.expires_at.clone(),
            None,
        ),
        AppealReviewOperation::Reissue { correction, .. } => (
            correction.category.map(|value| enum_text(&value)),
            correction.severity.map(|value| enum_text(&value)),
            correction.confidence.map(|value| value.to_string()),
            None,
            correction.visibility.map(|value| enum_text(&value)),
        ),
        AppealReviewOperation::Accept { .. } | AppealReviewOperation::Reject { .. } => {
            (None, None, None, None, None)
        }
    };
    [
        ("category", category),
        ("severity", severity),
        ("confidence", confidence),
        ("expires_at", expires_at),
        ("visibility", visibility),
    ]
    .into_iter()
    .map(|(name, value)| {
        format!(
            "<input type=\"hidden\" name=\"{}\" value=\"{}\">",
            name,
            escape_html(value.as_deref().unwrap_or_default()),
        )
    })
    .collect()
}

/// snake_case enum の wire 表現(解析側 `parse_optional_enum` と対称の serde 経由)。
fn enum_text<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => text,
        _ => String::new(),
    }
}

fn optional_number_text(value: Option<u8>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "なし".to_string())
}

fn optional_text(value: Option<&str>) -> String {
    value.unwrap_or("なし").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    use kukuri_cn_core::{
        AppealReviewOperation, AppealReviewReport, RiskSignalCorrection, RiskSignalMetadataEdit,
    };
    use kukuri_cn_safety::{SafetyCategory, Severity, Visibility};

    fn review_fixture() -> AppealReview {
        AppealReview {
            risk_signal_id: "signal-1".to_string(),
            issuer_node_id: "issuer-node".to_string(),
            target: "post_id".to_string(),
            target_id: "post-1".to_string(),
            category: "nsfw".to_string(),
            severity: "high".to_string(),
            basis: "classifier_score".to_string(),
            confidence: Some(90),
            visibility: "local".to_string(),
            expires_at: None,
            appeal_status: "disputed".to_string(),
            reports: vec![AppealReviewReport {
                id: "report-1".to_string(),
                details: Some("説明".to_string()),
                status: "received".to_string(),
                created_at: "2026-08-15T00:00:00Z".parse().expect("timestamp"),
            }],
        }
    }

    // --- #701: 確認画面に変更前後の値を表示する契約 ---

    #[test]
    fn edit_preview_shows_before_and_after_from_parsed_operation() {
        let review = review_fixture();
        let operation = AppealReviewOperation::Edit {
            expected: review.version(),
            edit: RiskSignalMetadataEdit {
                category: Some(SafetyCategory::Spam),
                // 調整フォームは現在値で事前入力されるため、現在値と同じ入力は「維持」と表示する。
                severity: Some(Severity::High),
                confidence: Some(20),
                expires_at: None,
            },
        };
        let html = render_appeal_preview("csrf", "ops@kukuri.app", &operation, &review);

        // 変更前の出典を明示する。
        assert!(html.contains("確認時点の審査情報"), "{html}");
        // 変更する項目は変更前 → 変更後を表示する。
        assert!(
            html.contains("<li>分類: <code>nsfw</code> → <code>spam</code></li>"),
            "{html}"
        );
        assert!(
            html.contains("<li>確信度: <code>90</code> → <code>20</code></li>"),
            "{html}"
        );
        // 値を維持する項目は「維持」と区別して表示する(現在値と同じ入力・未入力の両方)。
        assert!(
            html.contains("<li>深刻度: <code>high</code>(維持)</li>"),
            "{html}"
        );
        assert!(
            html.contains("<li>失効時刻: <code>なし</code>(維持)</li>"),
            "{html}"
        );

        // 隠し入力は解析済み操作から生成する(表示値と適用値の出所を一致させる)。
        assert!(
            html.contains(r#"<input type="hidden" name="category" value="spam">"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<input type="hidden" name="severity" value="high">"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<input type="hidden" name="confidence" value="20">"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<input type="hidden" name="expires_at" value="">"#),
            "{html}"
        );
        assert!(html.contains(r#"name="expected_state""#), "{html}");
        // 実行者・対象・競合時の拒否方針は既存どおり表示する。
        assert!(html.contains("ops@kukuri.app"), "{html}");
        assert!(html.contains("signal-1"), "{html}");
        assert!(
            html.contains("確認後に変更されていた場合は拒否します"),
            "{html}"
        );
    }

    #[test]
    fn reissue_preview_shows_before_after_and_notes_empty_expiry() {
        let review = review_fixture();
        let operation = AppealReviewOperation::Reissue {
            expected: review.version(),
            correction: RiskSignalCorrection {
                category: None,
                severity: None,
                confidence: None,
                visibility: Some(Visibility::Public),
            },
        };
        let html = render_appeal_preview("csrf", "ops@kukuri.app", &operation, &review);

        assert!(
            html.contains("<li>公開範囲: <code>local</code> → <code>public</code></li>"),
            "{html}"
        );
        // 未入力(None)の項目は現在値の維持として表示する。
        assert!(
            html.contains("<li>分類: <code>nsfw</code>(維持)</li>"),
            "{html}"
        );
        assert!(
            html.contains("<li>深刻度: <code>high</code>(維持)</li>"),
            "{html}"
        );
        assert!(
            html.contains("<li>確信度: <code>90</code>(維持)</li>"),
            "{html}"
        );
        // 新しいリスク判定の失効時刻が未設定になる点を影響文で説明する。
        assert!(
            html.contains("新しいリスク判定の失効時刻は未設定になります"),
            "{html}"
        );
        // #710(案A): 再発行は旧判定を認容として終結させる(審査中のままにしない)。
        assert!(html.contains("認容"), "{html}");
        assert!(html.contains("終結"), "{html}");
        assert!(!html.contains("審査中のまま"), "{html}");
        assert!(
            html.contains(r#"<input type="hidden" name="visibility" value="public">"#),
            "{html}"
        );
    }

    #[test]
    fn accept_and_reject_previews_keep_transition_summary() {
        let review = review_fixture();
        let accept = render_appeal_preview(
            "csrf",
            "ops@kukuri.app",
            &AppealReviewOperation::Accept {
                expected: review.version(),
            },
            &review,
        );
        assert!(accept.contains("異議申し立てを認容しますか"), "{accept}");
        assert!(accept.contains("Cleared"), "{accept}");
        assert!(
            accept.contains("確認後に変更されていた場合は拒否します"),
            "{accept}"
        );

        let reject = render_appeal_preview(
            "csrf",
            "ops@kukuri.app",
            &AppealReviewOperation::Reject {
                expected: review.version(),
            },
            &review,
        );
        assert!(reject.contains("異議申し立てを棄却しますか"), "{reject}");
        assert!(reject.contains("None に戻し"), "{reject}");
    }

    #[test]
    fn preview_escapes_untrusted_values() {
        let mut review = review_fixture();
        review.category = "<script>alert(1)</script>".to_string();
        review.risk_signal_id = "signal-<script>".to_string();
        let operation = AppealReviewOperation::Edit {
            expected: review.version(),
            edit: RiskSignalMetadataEdit {
                category: Some(SafetyCategory::Spam),
                severity: None,
                confidence: None,
                expires_at: None,
            },
        };
        let html = render_appeal_preview("csrf", "ops<script>@kukuri.app", &operation, &review);
        assert!(!html.contains("<script>alert(1)</script>"), "{html}");
        assert!(
            html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"),
            "{html}"
        );
        assert!(!html.contains("signal-<script>"), "{html}");
        assert!(!html.contains("ops<script>"), "{html}");
    }
}
