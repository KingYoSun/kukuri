//! IAP operator UI for tester feedback (#802 / ADR 0039)。
//!
//! read-only の一覧のみを提供する(状態変更が無いため CSRF / preview→apply は不要)。
//! テスターフィードバックは operator が本文を読むことが目的のデータであるため、
//! 通報と異なり 3 つの自由記述を全文表示する(escape 済み。ADR 0039 §5)。

use axum::extract::State;
use axum::response::Html;
use kukuri_cn_core::{TesterFeedback, list_tester_feedback};

use crate::admin::{AdminState, escape_html};
use crate::admin_shell::render_admin_page;

pub(crate) async fn tester_feedback_page(State(state): State<AdminState>) -> Html<String> {
    let main = match list_tester_feedback(&state.runtime.pool, 50, 0).await {
        Ok(feedback) => render_tester_feedback_section(&feedback),
        Err(error) => format!(
            "<section><p>{}</p></section>",
            escape_html(&format!("{error:#}"))
        ),
    };
    Html(render_admin_page(
        "テスターフィードバック",
        "<nav class=\"crumbs\"><a href=\"/\">コミュニティノード運営</a><span>›</span><span aria-current=\"page\">テスターフィードバック</span></nav><h1>テスターフィードバック</h1>",
        &main,
    ))
}

fn render_tester_feedback_section(feedback: &[TesterFeedback]) -> String {
    let rows: String = if feedback.is_empty() {
        "<tr><td colspan=\"6\">テスターフィードバックはありません。</td></tr>".to_string()
    } else {
        feedback
            .iter()
            .map(|entry| {
                format!(
                    "<tr><td>{}</td><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code> / <code>{}</code></td></tr>",
                    escape_html(&entry.created_at.to_rfc3339()),
                    escape_html(&entry.id),
                    escape_html(&entry.what_attempted),
                    escape_html(&entry.what_happened),
                    escape_html(&entry.what_seemed_wrong),
                    escape_html(&entry.client_version),
                    escape_html(&entry.os),
                )
            })
            .collect()
    };
    format!(
        "<section><p>新しい順に 50 件を表示します。送信者の識別情報は保存していません。</p><table><thead><tr><th>受信時刻</th><th>識別子</th><th>やろうとしたこと</th><th>何が起きたか</th><th>何が変だと思ったか</th><th>クライアント版 / OS</th></tr></thead><tbody>{rows}</tbody></table></section>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tester_feedback_rows_render_all_fields_escaped() {
        let html = render_tester_feedback_section(&[TesterFeedback {
            id: "feedback-1".to_string(),
            what_attempted: "<script>alert(1)</script>".to_string(),
            what_happened: "送信ボタンを押しても反応がなかった".to_string(),
            what_seemed_wrong: "\"quoted\" & <b>bold</b>".to_string(),
            client_version: "0.1.7".to_string(),
            os: "linux".to_string(),
            created_at: "2026-08-28T00:00:00Z".parse().expect("timestamp"),
        }]);

        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("送信ボタンを押しても反応がなかった"));
        assert!(html.contains("&quot;quoted&quot; &amp; &lt;b&gt;bold&lt;/b&gt;"));
        assert!(html.contains("<code>0.1.7</code>"));
        assert!(html.contains("<code>linux</code>"));
        assert!(html.contains("feedback-1"));
    }

    #[test]
    fn tester_feedback_section_shows_empty_state() {
        let html = render_tester_feedback_section(&[]);
        assert!(html.contains("テスターフィードバックはありません。"));
    }
}
