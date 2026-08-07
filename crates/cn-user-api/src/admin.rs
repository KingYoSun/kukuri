//! IAP TCP forwarding 経由でのみ公開する、read-only の operator dashboard。
//!
//! public user API と route / listener を分離し、管理用 write は既存 `cn-cli` に残す。
//! browser write を追加する際は、先に actor を記録する append-only audit contract と
//! CSRF 防御を定義すること。

use axum::Router;
use axum::extract::State;
use axum::response::Html;
use axum::routing::get;
use kukuri_cn_core::{
    latest_readiness_activation, list_community_node_reports, list_supported_topics,
    load_admission_config,
};
use tower_http::trace::TraceLayer;

use crate::state::UserApiState;

pub(crate) fn admin_router(state: UserApiState) -> Router {
    Router::new()
        .route("/", get(dashboard))
        .route("/healthz", get(|| async { "ok" }))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

async fn dashboard(State(state): State<UserApiState>) -> Html<String> {
    match load_dashboard(&state).await {
        Ok(view) => Html(render_dashboard(&view)),
        Err(error) => {
            tracing::error!(error = %format!("{error:#}"), "admin dashboard data load failed");
            Html(render_error())
        }
    }
}

#[derive(Debug)]
struct DashboardView {
    admission_mode: String,
    readiness: String,
    topics: Vec<(String, String, String)>,
    reports: Vec<ReportView>,
    gcp_project: Option<String>,
}

#[derive(Debug)]
struct ReportView {
    created_at: String,
    id: String,
    subject: String,
    capability: String,
    reason: String,
    status: String,
}

async fn load_dashboard(state: &UserApiState) -> anyhow::Result<DashboardView> {
    let admission = load_admission_config(&state.pool).await?;
    let readiness = latest_readiness_activation(&state.pool)
        .await?
        .map(|activation| {
            format!(
                "{} · profile={} · {}",
                activation.activated_at.to_rfc3339(),
                activation.profile,
                if activation.revoked {
                    "revoked"
                } else {
                    "active"
                }
            )
        })
        .unwrap_or_else(|| "no activation recorded".to_string());
    let topics = list_supported_topics(&state.pool)
        .await?
        .into_iter()
        .map(|topic| {
            (
                topic.kind.as_str().to_string(),
                topic.id,
                topic.created_at.to_rfc3339(),
            )
        })
        .collect();
    let reports = list_community_node_reports(&state.pool, 50, 0)
        .await?
        .into_iter()
        .map(|report| ReportView {
            created_at: report.created_at.to_rfc3339(),
            id: report.id,
            subject: format!("{}/{}", report.subject_kind, report.subject_id),
            capability: report.capability,
            reason: report.reason,
            status: report.status,
        })
        .collect();

    Ok(DashboardView {
        admission_mode: admission.mode.as_str().to_string(),
        readiness,
        topics,
        reports,
        gcp_project: std::env::var("COMMUNITY_NODE_GCP_PROJECT")
            .ok()
            .filter(|value| !value.trim().is_empty()),
    })
}

fn render_dashboard(view: &DashboardView) -> String {
    let topics = if view.topics.is_empty() {
        table_empty("No supported topics configured")
    } else {
        view.topics
            .iter()
            .map(|(kind, id, created_at)| {
                format!(
                    "<tr><td>{}</td><td><code>{}</code></td><td>{}</td></tr>",
                    escape_html(kind),
                    escape_html(id),
                    escape_html(created_at)
                )
            })
            .collect()
    };
    let reports = if view.reports.is_empty() {
        table_empty("No reports received")
    } else {
        view.reports
            .iter()
            .map(|report| {
                format!(
                    "<tr><td>{}</td><td><code>{}</code></td><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    escape_html(&report.created_at),
                    escape_html(&report.id),
                    escape_html(&report.subject),
                    escape_html(&report.capability),
                    escape_html(&report.reason),
                    escape_html(&report.status)
                )
            })
            .collect()
    };
    let logs_link = view.gcp_project.as_ref().map_or_else(
        || "<p>Set <code>COMMUNITY_NODE_GCP_PROJECT</code> to show the Cloud Logging link.</p>".to_string(),
        |project| format!(
            "<p><a href=\"https://console.cloud.google.com/logs/query?project={}\" target=\"_blank\" rel=\"noreferrer\">Open Cloud Logging</a></p>",
            escape_html(project)
        ),
    );

    format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>kukuri Community Node admin</title><style>
:root{{color-scheme:light dark;font-family:system-ui,sans-serif}}body{{max-width:1180px;margin:0 auto;padding:24px;line-height:1.5}}header,section{{border:1px solid #7775;border-radius:12px;padding:18px;margin-bottom:18px}}.metrics{{display:grid;grid-template-columns:repeat(auto-fit,minmax(240px,1fr));gap:12px}}.metric{{background:#7771;border-radius:8px;padding:12px}}table{{border-collapse:collapse;width:100%;display:block;overflow:auto}}th,td{{border-bottom:1px solid #7774;padding:8px;text-align:left;vertical-align:top}}code{{overflow-wrap:anywhere}}.boundary{{border-left:4px solid #d59b16;padding-left:12px}}a{{color:#388bfd}}
</style></head><body>
<header><h1>Community Node admin</h1><p>Read-only operator view. Access is restricted by the GCP IAP TCP tunnel and IAM.</p></header>
<section><h2>Status</h2><div class="metrics"><div class="metric"><strong>User API</strong><br>running</div><div class="metric"><strong>Admission</strong><br>{}</div><div class="metric"><strong>Latest readiness</strong><br>{}</div></div></section>
<section><h2>Supported topics</h2><table><thead><tr><th>Kind</th><th>ID</th><th>Created</th></tr></thead><tbody>{}</tbody></table></section>
<section><h2>Recent reports</h2><p>Newest 50 reports. Full details and operator actions remain available through <code>cn-cli reports</code>.</p><table><thead><tr><th>Created</th><th>ID</th><th>Subject</th><th>Capability</th><th>Reason</th><th>Status</th></tr></thead><tbody>{}</tbody></table></section>
<section><h2>Audit and logs</h2><p>The readiness activation above is append-only audit state. Runtime logs remain in Cloud Logging and journald; this view does not copy secrets or raw logs into another store.</p>{}<p><code>sudo journalctl -u kukuri-readiness.service -n 100 --no-pager</code></p></section>
<section><h2>Safe changes</h2><p class="boundary">This first admin surface is intentionally read-only. Supported topic, admission, provider, and capability changes continue through reviewed <code>operator-config.yaml</code> / <code>cn-cli</code> workflows. Browser writes require an actor-aware append-only audit contract, validation preview, and CSRF protection before enablement.</p></section>
</body></html>"#,
        escape_html(&view.admission_mode),
        escape_html(&view.readiness),
        topics,
        reports,
        logs_link,
    )
}

fn table_empty(message: &str) -> String {
    format!("<tr><td colspan=\"6\">{}</td></tr>", escape_html(message))
}

fn render_error() -> String {
    "<!doctype html><html><head><meta charset=\"utf-8\"><title>kukuri admin unavailable</title></head><body><h1>Admin data unavailable</h1><p>Check cn-user-api logs through the IAP SSH runbook.</p></body></html>".to_string()
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_escapes_database_and_environment_values() {
        let html = render_dashboard(&DashboardView {
            admission_mode: "open".to_string(),
            readiness: "active".to_string(),
            topics: vec![(
                "public_topic".to_string(),
                "<script>alert(1)</script>".to_string(),
                "now".to_string(),
            )],
            reports: vec![ReportView {
                created_at: "now".to_string(),
                id: "report-1".to_string(),
                subject: "event/&danger".to_string(),
                capability: "report_endpoint".to_string(),
                reason: "\"quoted\"".to_string(),
                status: "received".to_string(),
            }],
            gcp_project: Some("project-id".to_string()),
        });

        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("event/&amp;danger"));
        assert!(html.contains("&quot;quoted&quot;"));
    }
}
