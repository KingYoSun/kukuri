//! IAP TCP forwarding 経由でのみ公開する operator dashboard。
//!
//! public user API と route / listener を分離する。browser write は ADR 0029 に従い、
//! runtime DB 操作だけを preview / CSRF / deployment actor / append-only audit 付きで扱う。

use anyhow::bail;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use kukuri_cn_core::{
    AdminOperation, AdmissionMode, OperatorReportStatus, apply_operator_action,
    latest_readiness_activation, list_community_node_reports, list_operator_actions,
    list_supported_topics, load_admission_config, validate_admin_operation,
};
use serde::Deserialize;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::state::UserApiState;

pub(crate) fn admin_router(state: UserApiState) -> Router {
    let actor = std::env::var("COMMUNITY_NODE_ADMIN_ACTOR")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_string());
    let csrf_token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let state = AdminState {
        runtime: state,
        actor,
        csrf_token,
    };
    Router::new()
        .route("/", get(dashboard))
        .route("/healthz", get(|| async { "ok" }))
        .route("/actions/preview", post(preview_action))
        .route("/actions/apply", post(apply_action))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

#[derive(Clone)]
struct AdminState {
    runtime: UserApiState,
    actor: Option<String>,
    csrf_token: String,
}

#[derive(Debug, Deserialize)]
struct AdminActionForm {
    csrf_token: String,
    action: String,
    #[serde(default)]
    target_id: String,
    #[serde(default)]
    value: String,
}

async fn dashboard(State(state): State<AdminState>) -> Html<String> {
    match load_dashboard(&state).await {
        Ok(view) => Html(render_dashboard(&view)),
        Err(error) => {
            tracing::error!(error = %format!("{error:#}"), "admin dashboard data load failed");
            Html(render_error())
        }
    }
}

async fn preview_action(
    State(state): State<AdminState>,
    Form(form): Form<AdminActionForm>,
) -> Response {
    let Some(actor) = state.actor.as_deref() else {
        return render_action_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Browser writes are disabled because COMMUNITY_NODE_ADMIN_ACTOR is not configured.",
        );
    };
    if !csrf_matches(state.csrf_token.as_str(), form.csrf_token.as_str()) {
        return render_action_error(
            StatusCode::FORBIDDEN,
            "The CSRF token is invalid or expired.",
        );
    }
    match operation_from_form(&form) {
        Ok(operation) => Html(render_preview(
            state.csrf_token.as_str(),
            actor,
            &form,
            &operation,
        ))
        .into_response(),
        Err(error) => render_action_error(StatusCode::BAD_REQUEST, format!("{error:#}").as_str()),
    }
}

async fn apply_action(
    State(state): State<AdminState>,
    Form(form): Form<AdminActionForm>,
) -> Response {
    let Some(actor) = state.actor.as_deref() else {
        return render_action_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Browser writes are disabled because COMMUNITY_NODE_ADMIN_ACTOR is not configured.",
        );
    };
    if !csrf_matches(state.csrf_token.as_str(), form.csrf_token.as_str()) {
        return render_action_error(
            StatusCode::FORBIDDEN,
            "The CSRF token is invalid or expired.",
        );
    }
    let operation = match operation_from_form(&form) {
        Ok(operation) => operation,
        Err(error) => {
            return render_action_error(StatusCode::BAD_REQUEST, format!("{error:#}").as_str());
        }
    };
    match apply_operator_action(&state.runtime.pool, actor, &operation).await {
        Ok(action) => {
            tracing::info!(
                action_id = %action.id,
                actor = %action.actor,
                action = %action.action,
                target_kind = %action.target_kind,
                target_id = %action.target_id,
                "admin operation applied"
            );
            Html(render_action_success(&action)).into_response()
        }
        Err(error) => {
            tracing::warn!(
                actor,
                error = %format!("{error:#}"),
                "admin operation rejected"
            );
            render_action_error(StatusCode::BAD_REQUEST, format!("{error:#}").as_str())
        }
    }
}

#[derive(Debug)]
struct DashboardView {
    admission_mode: String,
    readiness: String,
    topics: Vec<(String, String, String)>,
    reports: Vec<ReportView>,
    audit: Vec<AuditView>,
    gcp_project: Option<String>,
    actor: Option<String>,
    csrf_token: String,
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

#[derive(Debug)]
struct AuditView {
    occurred_at: String,
    id: String,
    actor: String,
    action: String,
    target: String,
    change: String,
}

async fn load_dashboard(state: &AdminState) -> anyhow::Result<DashboardView> {
    let admission = load_admission_config(&state.runtime.pool).await?;
    let readiness = latest_readiness_activation(&state.runtime.pool)
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
    let topics = list_supported_topics(&state.runtime.pool)
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
    let reports = list_community_node_reports(&state.runtime.pool, 50, 0)
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
    let audit = list_operator_actions(&state.runtime.pool, 50, 0)
        .await?
        .into_iter()
        .map(|entry| AuditView {
            occurred_at: entry.occurred_at.to_rfc3339(),
            id: entry.id,
            actor: entry.actor,
            action: entry.action,
            target: format!("{}/{}", entry.target_kind, entry.target_id),
            change: format!("{} -> {}", entry.before, entry.after),
        })
        .collect();

    Ok(DashboardView {
        admission_mode: admission.mode.as_str().to_string(),
        readiness,
        topics,
        reports,
        audit,
        gcp_project: std::env::var("COMMUNITY_NODE_GCP_PROJECT")
            .ok()
            .filter(|value| !value.trim().is_empty()),
        actor: state.actor.clone(),
        csrf_token: state.csrf_token.clone(),
    })
}

fn render_dashboard(view: &DashboardView) -> String {
    let write_enabled = view.actor.is_some();
    let admission_control = view.actor.as_ref().map_or_else(
        || "<p class=\"boundary\">Browser writes are disabled. Set <code>COMMUNITY_NODE_ADMIN_ACTOR</code> through the reviewed deployment workflow to enable them.</p>".to_string(),
        |actor| {
            format!(
                r#"<form method="post" action="/actions/preview" class="inline-form">
<input type="hidden" name="csrf_token" value="{}"><input type="hidden" name="action" value="admission.set_mode">
<label>Next mode <select name="value">{}</select></label><button type="submit">Preview admission change</button>
</form><p class="meta">Audited deployment actor: <code>{}</code></p>"#,
                escape_html(&view.csrf_token),
                admission_options(&view.admission_mode),
                escape_html(actor),
            )
        },
    );
    let topics = if view.topics.is_empty() {
        table_empty("No supported topics configured", 4)
    } else {
        view.topics
            .iter()
            .map(|(kind, id, created_at)| {
                let action = if write_enabled && kind == "public_topic" {
                    format!(
                        r#"<form method="post" action="/actions/preview"><input type="hidden" name="csrf_token" value="{}"><input type="hidden" name="action" value="supported_topic.remove"><input type="hidden" name="target_id" value="{}"><button class="secondary" type="submit">Preview removal</button></form>"#,
                        escape_html(&view.csrf_token),
                        escape_html(id),
                    )
                } else {
                    "<span class=\"meta\">read-only</span>".to_string()
                };
                format!(
                    "<tr><td>{}</td><td><code>{}</code></td><td>{}</td><td>{}</td></tr>",
                    escape_html(kind),
                    escape_html(id),
                    escape_html(created_at),
                    action,
                )
            })
            .collect()
    };
    let add_topic = if write_enabled {
        format!(
            r#"<form method="post" action="/actions/preview" class="inline-form"><input type="hidden" name="csrf_token" value="{}"><input type="hidden" name="action" value="supported_topic.add"><label>Public topic ID <input name="target_id" required placeholder="demo"></label><button type="submit">Preview topic addition</button></form>"#,
            escape_html(&view.csrf_token),
        )
    } else {
        String::new()
    };
    let reports = if view.reports.is_empty() {
        table_empty("No reports received", 7)
    } else {
        view.reports
            .iter()
            .map(|report| {
                let action = if write_enabled {
                    format!(
                        r#"<form method="post" action="/actions/preview" class="compact-form"><input type="hidden" name="csrf_token" value="{}"><input type="hidden" name="action" value="report.set_status"><input type="hidden" name="target_id" value="{}"><select aria-label="Next status" name="value">{}</select><button class="secondary" type="submit">Preview</button></form>"#,
                        escape_html(&view.csrf_token),
                        escape_html(&report.id),
                        report_status_options(&report.status),
                    )
                } else {
                    escape_html(&report.status)
                };
                format!(
                    "<tr><td>{}</td><td><code>{}</code></td><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    escape_html(&report.created_at),
                    escape_html(&report.id),
                    escape_html(&report.subject),
                    escape_html(&report.capability),
                    escape_html(&report.reason),
                    escape_html(&report.status),
                    action,
                )
            })
            .collect()
    };
    let audit = if view.audit.is_empty() {
        table_empty("No browser operator actions recorded", 6)
    } else {
        view.audit
            .iter()
            .map(|entry| {
                format!(
                    "<tr><td>{}</td><td><code>{}</code></td><td>{}</td><td>{}</td><td><code>{}</code></td><td><code>{}</code></td></tr>",
                    escape_html(&entry.occurred_at),
                    escape_html(&entry.id),
                    escape_html(&entry.actor),
                    escape_html(&entry.action),
                    escape_html(&entry.target),
                    escape_html(&entry.change),
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
:root{{color-scheme:dark light;font-family:system-ui,sans-serif;--surface:#101923;--panel:#162231;--raised:#233241;--text:#f6f1e8;--muted:#cbbdae;--border:#39495a;--primary:#f59d62;--primary-text:#0e1b26;--focus:#00b3a4;--warning:#e6b066}}@media(prefers-color-scheme:light){{:root{{--surface:#f4efe6;--panel:#fff;--raised:#dfe6ec;--text:#21303b;--muted:#5f6c76;--border:#b7c2cb;--primary:#d77d45;--primary-text:#fff7ef;--focus:#0f8c82;--warning:#9a6e2a}}}}*{{box-sizing:border-box}}body{{max-width:1240px;margin:0 auto;padding:24px;line-height:1.5;background:var(--surface);color:var(--text)}}header,section,.dialog{{border:1px solid var(--border);border-radius:18px;padding:18px;margin-bottom:18px;background:var(--panel)}}.metrics{{display:grid;grid-template-columns:repeat(auto-fit,minmax(240px,1fr));gap:12px}}.metric{{background:var(--raised);border-radius:12px;padding:12px}}table{{border-collapse:collapse;width:100%;display:block;overflow:auto}}th,td{{border-bottom:1px solid var(--border);padding:9px;text-align:left;vertical-align:top}}code{{overflow-wrap:anywhere}}.boundary{{border-left:4px solid var(--warning);padding-left:12px}}a{{color:var(--focus)}}button{{min-height:40px;border:0;border-radius:999px;padding:8px 14px;background:var(--primary);color:var(--primary-text);font-weight:700;cursor:pointer}}button.secondary{{background:var(--raised);color:var(--text)}}input,select{{min-height:40px;border:1px solid var(--border);border-radius:10px;padding:7px 10px;background:var(--surface);color:var(--text)}}.inline-form,.compact-form{{display:flex;flex-wrap:wrap;align-items:end;gap:10px;margin:12px 0}}.compact-form{{min-width:270px}}label{{display:grid;gap:4px}}.meta{{color:var(--muted);font-size:.875rem}}.danger{{color:var(--warning)}}
</style></head><body>
<header><h1>Community Node admin</h1><p>Operator-only surface. Access is restricted by the GCP IAP TCP tunnel and IAM.</p></header>
<section><h2>Status and admission</h2><div class="metrics"><div class="metric"><strong>User API</strong><br>running</div><div class="metric"><strong>Admission</strong><br>{}</div><div class="metric"><strong>Latest readiness</strong><br>{}</div></div>{}</section>
<section><h2>Supported topics</h2><p>Only public topics can be changed here. Private channel capability remains outside browser writes.</p>{}<table><thead><tr><th>Kind</th><th>ID</th><th>Created</th><th>Action</th></tr></thead><tbody>{}</tbody></table></section>
<section><h2>Recent reports</h2><p>Newest 50 reports. Details and reporter contact are intentionally omitted from this surface and audit.</p><table><thead><tr><th>Created</th><th>ID</th><th>Subject</th><th>Capability</th><th>Reason</th><th>Status</th><th>Action</th></tr></thead><tbody>{}</tbody></table></section>
<section><h2>Operator action audit</h2><p>Newest 50 browser actions. This table is append-only at the database layer.</p><table><thead><tr><th>Occurred</th><th>ID</th><th>Actor</th><th>Action</th><th>Target</th><th>Change</th></tr></thead><tbody>{}</tbody></table></section>
<section><h2>Logs</h2><p>Runtime logs remain in Cloud Logging and journald; this view does not copy secrets or raw logs into another store.</p>{}<p><code>sudo journalctl -u kukuri-readiness.service -n 100 --no-pager</code></p></section>
<section><h2>Deployment boundary</h2><p class="boundary">Provider/LLM credentials, capability availability, image revision, private channel secrets, invite codes, allowlist, and bans are not browser writes. Continue to use reviewed <code>operator-config.yaml</code>, Terraform, secret management, and <code>cn-cli readiness</code>. Every enabled browser action requires preview and explicit confirmation.</p></section>
</body></html>"#,
        escape_html(&view.admission_mode),
        escape_html(&view.readiness),
        admission_control,
        add_topic,
        topics,
        reports,
        audit,
        logs_link,
    )
}

fn table_empty(message: &str, colspan: usize) -> String {
    format!(
        "<tr><td colspan=\"{colspan}\">{}</td></tr>",
        escape_html(message)
    )
}

fn admission_options(current: &str) -> String {
    ["open", "invite", "whitelist"]
        .into_iter()
        .map(|value| option(value, value, value == current))
        .collect()
}

fn report_status_options(current: &str) -> String {
    ["received", "reviewing", "actioned", "dismissed"]
        .into_iter()
        .map(|value| option(value, value, value == current))
        .collect()
}

fn option(value: &str, label: &str, selected: bool) -> String {
    format!(
        "<option value=\"{}\"{}>{}</option>",
        escape_html(value),
        if selected { " selected" } else { "" },
        escape_html(label)
    )
}

fn operation_from_form(form: &AdminActionForm) -> anyhow::Result<AdminOperation> {
    let operation = match form.action.as_str() {
        "admission.set_mode" => {
            let mode = match form.value.trim() {
                "open" => AdmissionMode::Open,
                "invite" => AdmissionMode::Invite,
                "whitelist" => AdmissionMode::Whitelist,
                _ => bail!("unsupported admission mode"),
            };
            AdminOperation::SetAdmissionMode { mode }
        }
        "supported_topic.add" => AdminOperation::AddSupportedPublicTopic {
            topic_id: form.target_id.trim().to_string(),
        },
        "supported_topic.remove" => AdminOperation::RemoveSupportedPublicTopic {
            topic_id: form.target_id.trim().to_string(),
        },
        "report.set_status" => {
            let status = match form.value.trim() {
                "received" => OperatorReportStatus::Received,
                "reviewing" => OperatorReportStatus::Reviewing,
                "actioned" => OperatorReportStatus::Actioned,
                "dismissed" => OperatorReportStatus::Dismissed,
                _ => bail!("unsupported report status"),
            };
            AdminOperation::SetReportStatus {
                report_id: form.target_id.trim().to_string(),
                status,
            }
        }
        _ => bail!("unsupported admin operation"),
    };
    validate_admin_operation(&operation)?;
    Ok(operation)
}

fn csrf_matches(expected: &str, supplied: &str) -> bool {
    if expected.len() != supplied.len() || expected.is_empty() {
        return false;
    }
    expected
        .bytes()
        .zip(supplied.bytes())
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

fn render_preview(
    csrf_token: &str,
    actor: &str,
    form: &AdminActionForm,
    operation: &AdminOperation,
) -> String {
    let (title, target, impact) = operation_summary(operation);
    let body = format!(
        r#"<div class="dialog"><p class="meta">Validation preview</p><h1>{}</h1><dl><dt>Actor</dt><dd><code>{}</code></dd><dt>Target</dt><dd><code>{}</code></dd><dt>Impact</dt><dd>{}</dd></dl><p class="boundary">The current state will be loaded and validated again during apply. The state change and audit row commit in one transaction.</p><form method="post" action="/actions/apply"><input type="hidden" name="csrf_token" value="{}"><input type="hidden" name="action" value="{}"><input type="hidden" name="target_id" value="{}"><input type="hidden" name="value" value="{}"><button type="submit">Confirm and apply</button> <a href="/">Cancel</a></form></div>"#,
        escape_html(title),
        escape_html(actor),
        escape_html(&target),
        escape_html(&impact),
        escape_html(csrf_token),
        escape_html(&form.action),
        escape_html(&form.target_id),
        escape_html(&form.value),
    );
    render_simple_page("Confirm admin operation", body.as_str())
}

fn operation_summary(operation: &AdminOperation) -> (&'static str, String, String) {
    match operation {
        AdminOperation::SetAdmissionMode { mode } => (
            "Change admission mode",
            "community-node admission".to_string(),
            format!(
                "Set mode to {}. Existing active subscribers remain admitted.",
                mode.as_str()
            ),
        ),
        AdminOperation::AddSupportedPublicTopic { topic_id } => (
            "Add supported public topic",
            topic_id.clone(),
            "Allow the indexer to ingest this public topic. Provider and readiness gates still apply."
                .to_string(),
        ),
        AdminOperation::RemoveSupportedPublicTopic { topic_id } => (
            "Remove supported public topic",
            topic_id.clone(),
            "Stop future ingest for this scope. De-index and worker reconciliation are asynchronous."
                .to_string(),
        ),
        AdminOperation::SetReportStatus { report_id, status } => (
            "Change report status",
            report_id.clone(),
            format!("Set operator-local report status to {}.", status.as_str()),
        ),
    }
}

fn render_action_success(action: &kukuri_cn_core::OperatorAction) -> String {
    let body = format!(
        r#"<div class="dialog"><p class="meta">Operation completed</p><h1>Admin operation applied</h1><dl><dt>Audit ID</dt><dd><code>{}</code></dd><dt>Actor</dt><dd><code>{}</code></dd><dt>Action</dt><dd>{}</dd><dt>Target</dt><dd><code>{}/{}</code></dd><dt>Change</dt><dd><code>{} -&gt; {}</code></dd></dl><p><a href="/">Return to dashboard</a></p></div>"#,
        escape_html(&action.id),
        escape_html(&action.actor),
        escape_html(&action.action),
        escape_html(&action.target_kind),
        escape_html(&action.target_id),
        escape_html(&action.before.to_string()),
        escape_html(&action.after.to_string()),
    );
    render_simple_page("Admin operation applied", body.as_str())
}

fn render_action_error(status: StatusCode, message: &str) -> Response {
    let body = format!(
        r#"<div class="dialog"><p class="meta">Operation not applied</p><h1>Admin operation rejected</h1><p class="danger">{}</p><p>No state or audit row was committed.</p><p><a href="/">Return to dashboard</a></p></div>"#,
        escape_html(message)
    );
    (
        status,
        Html(render_simple_page(
            "Admin operation rejected",
            body.as_str(),
        )),
    )
        .into_response()
}

fn render_simple_page(title: &str, body: &str) -> String {
    format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{}</title><style>:root{{color-scheme:dark light;font-family:system-ui,sans-serif}}body{{max-width:760px;margin:0 auto;padding:24px;line-height:1.5}}.dialog{{border:1px solid currentColor;border-radius:18px;padding:20px}}code{{overflow-wrap:anywhere}}.meta{{opacity:.72}}.boundary{{border-left:4px solid #d59b16;padding-left:12px}}button{{min-height:44px;border:0;border-radius:999px;padding:10px 16px;font-weight:700}}dt{{font-weight:700;margin-top:10px}}dd{{margin-left:0}}</style></head><body>{}</body></html>"#,
        escape_html(title),
        body,
    )
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
            audit: vec![AuditView {
                occurred_at: "now".to_string(),
                id: "audit-1".to_string(),
                actor: "ops@example.com".to_string(),
                action: "report.set_status".to_string(),
                target: "report/report-1".to_string(),
                change: "received -> reviewing".to_string(),
            }],
            gcp_project: Some("project-id".to_string()),
            actor: Some("ops@example.com".to_string()),
            csrf_token: "csrf-token".to_string(),
        });

        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("event/&amp;danger"));
        assert!(html.contains("&quot;quoted&quot;"));
    }

    #[test]
    fn admin_form_requires_csrf_and_maps_only_supported_runtime_operations() {
        let form = AdminActionForm {
            csrf_token: "token".to_string(),
            action: "admission.set_mode".to_string(),
            target_id: String::new(),
            value: "invite".to_string(),
        };
        assert!(csrf_matches("token", form.csrf_token.as_str()));
        assert!(!csrf_matches("token", "wrong"));
        assert!(matches!(
            operation_from_form(&form).unwrap(),
            kukuri_cn_core::AdminOperation::SetAdmissionMode {
                mode: kukuri_cn_core::AdmissionMode::Invite
            }
        ));

        let mut unsupported = form;
        unsupported.action = "provider.set_api_key".to_string();
        assert!(operation_from_form(&unsupported).is_err());
    }

    #[test]
    fn preview_escapes_actor_target_and_values() {
        let html = render_preview(
            "csrf",
            "ops<script>@kukuri.app",
            &AdminActionForm {
                csrf_token: "csrf".to_string(),
                action: "supported_topic.add".to_string(),
                target_id: "kukuri:topic:<script>".to_string(),
                value: String::new(),
            },
            &kukuri_cn_core::AdminOperation::AddSupportedPublicTopic {
                topic_id: "kukuri:topic:<script>".to_string(),
            },
        );
        assert!(!html.contains("ops<script>"));
        assert!(!html.contains("kukuri:topic:<script>"));
        assert!(html.contains("ops&lt;script&gt;@kukuri.app"));
        assert!(html.contains("kukuri:topic:&lt;script&gt;"));
    }
}
