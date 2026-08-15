//! 異議申し立て審査一覧の表示。

use kukuri_cn_core::AppealReview;

use crate::admin::{escape_html, option};

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
