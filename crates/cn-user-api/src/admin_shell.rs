//! 管理画面の共通 view shell(#740)。
//!
//! dashboard と確認・完了・拒否ページが同じ color token・spacing・button 階層・header を
//! 共有するための document 枠と部品を置く。ADR 0029 の `preview -> confirm -> apply` 契約や
//! 隠し入力の出所には関与しない(view 層のみ)。

use crate::admin::escape_html;

/// 管理画面全体で共有する style。色値は `DESIGN.md` §9 のクイックリファレンスを正とする
/// (server-rendered HTML は `tokens.css` を参照できないため、ここが唯一の置き場)。
pub(crate) const ADMIN_STYLE: &str = r#":root{color-scheme:dark light;font-family:system-ui,sans-serif;--surface:#101923;--panel:#162231;--raised:#233241;--text:#f6f1e8;--muted:#cbbdae;--border:#39495a;--primary:#f59d62;--primary-text:#0e1b26;--focus:#00b3a4;--warning:#e6b066}@media(prefers-color-scheme:light){:root{--surface:#f4efe6;--panel:#fff;--raised:#dfe6ec;--text:#21303b;--muted:#5f6c76;--border:#b7c2cb;--primary:#d77d45;--primary-text:#fff7ef;--focus:#0f8c82;--warning:#9a6e2a}}*{box-sizing:border-box}body{max-width:1240px;margin:0 auto;padding:24px;line-height:1.5;background:var(--surface);color:var(--text)}header,section,.dialog{border:1px solid var(--border);border-radius:18px;padding:18px;margin-bottom:18px;background:var(--panel)}.dialog{max-width:840px}.metrics{display:grid;grid-template-columns:repeat(auto-fit,minmax(240px,1fr));gap:12px}.metric{background:var(--raised);border-radius:12px;padding:12px}table{border-collapse:collapse;width:100%;display:block;overflow:auto}th,td{border-bottom:1px solid var(--border);padding:9px;text-align:left;vertical-align:top}code{overflow-wrap:anywhere}.boundary{border-left:4px solid var(--warning);padding-left:12px}a{color:var(--focus)}:focus-visible{outline:3px solid var(--focus);outline-offset:2px}button,a.button{display:inline-flex;align-items:center;justify-content:center;min-height:44px;border:0;border-radius:999px;padding:8px 14px;background:var(--primary);color:var(--primary-text);font-weight:700;cursor:pointer;text-decoration:none;font-size:1rem}button.secondary,a.button.secondary{background:var(--raised);color:var(--text)}input,select{min-height:44px;border:1px solid var(--border);border-radius:10px;padding:7px 10px;background:var(--surface);color:var(--text)}.inline-form,.compact-form,.review-actions{display:flex;flex-wrap:wrap;align-items:end;gap:10px;margin:12px 0}.compact-form{min-width:270px}label{display:grid;gap:4px}.meta{color:var(--muted);font-size:.875rem}.danger{color:var(--warning)}.crumbs{display:flex;flex-wrap:wrap;gap:8px;margin:0 0 6px;color:var(--muted);font-size:.875rem}.crumbs [aria-current]{color:var(--text);font-weight:700}.facts{display:grid;grid-template-columns:minmax(120px,auto) minmax(0,1fr);gap:8px 14px;margin:14px 0}.facts dt{font-weight:700}.facts dd{margin:0}.facts ul{margin:0;padding-left:18px}.facts .meta{margin:0 0 4px}.actions{display:flex;flex-wrap:wrap;align-items:center;gap:10px;margin-top:16px}.appeal-card{border:1px solid var(--border);border-radius:14px;padding:14px;margin:14px 0;background:var(--surface)}.appeal-grid{display:grid;grid-template-columns:minmax(100px,auto) minmax(0,1fr);gap:6px 12px}.appeal-grid dt{font-weight:700}.appeal-grid dd{margin:0}.edit-form{display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:10px;width:100%;padding:12px;border:1px solid var(--border);border-radius:12px}@media(max-width:720px){body{padding:12px}.appeal-grid,.facts{grid-template-columns:1fr}}"#;

/// 管理画面の document 枠。`header` と `main` は生成済み HTML を受け取る。
pub(crate) fn render_admin_page(title: &str, header: &str, main: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="ja"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{}</title><style>{ADMIN_STYLE}</style></head><body>
<header>{header}</header><main>{main}</main>
</body></html>"#,
        escape_html(title),
    )
}

/// 確認・完了・拒否ページの header。ダッシュボードへ戻る link と現在の段階を示す。
pub(crate) fn action_header(stage: &str, heading: &str) -> String {
    format!(
        r#"<nav class="crumbs" aria-label="現在位置"><a href="/">コミュニティノード運営</a><span aria-hidden="true">›</span><span aria-current="page">{}</span></nav><h1>{}</h1>"#,
        escape_html(stage),
        escape_html(heading),
    )
}

/// 実行者・対象・変更前後・影響・取引境界などを並べる比較枠。値は生成済み HTML を受け取る。
pub(crate) fn facts(items: &[(&str, String)]) -> String {
    let mut html = String::from("<dl class=\"facts\">");
    for (label, value) in items {
        html.push_str("<dt>");
        html.push_str(&escape_html(label));
        html.push_str("</dt><dd>");
        html.push_str(value);
        html.push_str("</dd>");
    }
    html.push_str("</dl>");
    html
}

/// 見出しと本文の両方を無害化した比較枠の 1 項目(文字列値用)。
pub(crate) fn text_fact(label: &'static str, value: &str) -> (&'static str, String) {
    (label, escape_html(value))
}

/// 識別子などを等幅で示す比較枠の 1 項目。
pub(crate) fn code_fact(label: &'static str, value: &str) -> (&'static str, String) {
    (label, format!("<code>{}</code>", escape_html(value)))
}

/// ダッシュボードへ戻る link。`primary` は拒否ページの回復操作、`secondary` は取り消しに使う。
pub(crate) fn dashboard_link(label: &str, primary: bool) -> String {
    format!(
        "<a class=\"button {}\" href=\"/\">{}</a>",
        if primary { "primary" } else { "secondary" },
        escape_html(label),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_defines_theme_tokens_focus_visible_and_narrow_layout() {
        // dark / light の token 切り替え、focus-visible、狭幅での比較枠の縦並びを固定する。
        for needle in [
            "--primary:",
            "--focus:",
            "--surface:",
            "prefers-color-scheme:light",
            ":focus-visible",
            "@media(max-width:720px)",
            ".facts{display:grid",
            "a.button",
            ".secondary",
        ] {
            assert!(ADMIN_STYLE.contains(needle), "style に {needle} が無い");
        }
    }

    #[test]
    fn page_wraps_header_and_main_in_shared_document() {
        let html = render_admin_page("確認 <b>", "<h1>見出し</h1>", "<section>本文</section>");
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("<title>確認 &lt;b&gt;</title>"));
        assert!(html.contains(ADMIN_STYLE));
        assert!(html.contains("<header><h1>見出し</h1></header>"));
        assert!(html.contains("<main><section>本文</section></main>"));
        assert!(html.contains("color-scheme:dark light"));
    }

    #[test]
    fn action_header_links_back_to_dashboard_and_marks_current_stage() {
        let html = action_header("適用前の確認", "受け入れ方式を変更しますか <x>");
        assert!(html.contains("<nav class=\"crumbs\" aria-label=\"現在位置\">"));
        assert!(html.contains("<a href=\"/\">コミュニティノード運営</a>"));
        assert!(html.contains("<span aria-current=\"page\">適用前の確認</span>"));
        assert!(html.contains("<h1>受け入れ方式を変更しますか &lt;x&gt;</h1>"));
    }

    #[test]
    fn facts_render_definition_grid_with_escaped_labels() {
        let html = facts(&[
            text_fact("運営者 <a>", "ops<script>"),
            code_fact("対象", "topic<1>"),
        ]);
        assert!(html.starts_with("<dl class=\"facts\">"));
        assert!(html.contains("<dt>運営者 &lt;a&gt;</dt><dd>ops&lt;script&gt;</dd>"));
        assert!(html.contains("<dt>対象</dt><dd><code>topic&lt;1&gt;</code></dd>"));
        assert!(html.ends_with("</dl>"));
    }

    #[test]
    fn dashboard_link_uses_button_hierarchy() {
        assert_eq!(
            dashboard_link("取り消す", false),
            "<a class=\"button secondary\" href=\"/\">取り消す</a>"
        );
        assert_eq!(
            dashboard_link("ダッシュボードへ戻る", true),
            "<a class=\"button primary\" href=\"/\">ダッシュボードへ戻る</a>"
        );
    }
}
