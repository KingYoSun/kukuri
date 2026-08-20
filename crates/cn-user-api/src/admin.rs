//! IAP TCP forwarding 経由でのみ公開する operator dashboard。
//!
//! public user API と route / listener を分離する。browser write は ADR 0029 に従い、
//! runtime DB 操作だけを preview / CSRF / deployment actor / append-only audit 付きで扱う。

use anyhow::{Context, bail};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use kukuri_cn_core::{
    AdminOperation, AdmissionMode, AppealReview, AppealReviewOperation, AppealReviewVersion,
    OperatorReportStatus, RiskSignalCorrection, RiskSignalMetadataEdit, apply_appeal_review_action,
    apply_operator_action, get_appeal_review, latest_readiness_activation, list_appeal_reviews,
    list_community_node_reports, list_operator_actions, list_supported_topics,
    load_admission_config, parse_bool_env, validate_admin_operation, validate_optional_confidence,
    validate_optional_expires_at,
};
use serde::Deserialize;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::admin_appeal_render::render_appeal_reviews;
use crate::state::UserApiState;

pub(crate) fn admin_router(state: UserApiState) -> Router {
    let actor = std::env::var("COMMUNITY_NODE_ADMIN_ACTOR")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_string());
    let csrf_token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let operator_review_enabled =
        parse_bool_env("COMMUNITY_NODE_SAFETY_OPERATOR_REVIEW", false).unwrap_or_else(|error| {
            tracing::warn!(error = %format!("{error:#}"), "運営者確認の有効化設定を解釈できないため無効化します");
            false
        });
    let state = AdminState {
        runtime: state,
        actor,
        csrf_token,
        operator_review_enabled,
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
    operator_review_enabled: bool,
}

#[derive(Debug, Default, Deserialize)]
struct AdminActionForm {
    csrf_token: String,
    action: String,
    #[serde(default)]
    target_id: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    expected_state: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    severity: String,
    #[serde(default)]
    confidence: String,
    #[serde(default)]
    expires_at: String,
    #[serde(default)]
    visibility: String,
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
            "COMMUNITY_NODE_ADMIN_ACTOR が未設定のため、変更操作は無効です。",
        );
    };
    if !csrf_matches(state.csrf_token.as_str(), form.csrf_token.as_str()) {
        return render_action_error(
            StatusCode::FORBIDDEN,
            "画面の確認情報が無効または期限切れです。画面を読み直してください。",
        );
    }
    if form.action.starts_with("appeal.") {
        return preview_appeal_action(&state, actor, &form).await;
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
            "COMMUNITY_NODE_ADMIN_ACTOR が未設定のため、変更操作は無効です。",
        );
    };
    if !csrf_matches(state.csrf_token.as_str(), form.csrf_token.as_str()) {
        return render_action_error(
            StatusCode::FORBIDDEN,
            "画面の確認情報が無効または期限切れです。画面を読み直してください。",
        );
    }
    if form.action.starts_with("appeal.") {
        return apply_appeal_action(&state, actor, &form).await;
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

async fn preview_appeal_action(
    state: &AdminState,
    actor: &str,
    form: &AdminActionForm,
) -> Response {
    if !state.operator_review_enabled {
        return render_action_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "このノードでは異議申し立ての運営者確認が無効です。",
        );
    }
    let review = match get_appeal_review(&state.runtime.pool, form.target_id.trim()).await {
        Ok(Some(review)) if review.appeal_status == "disputed" => review,
        Ok(Some(_)) => {
            return render_action_error(
                StatusCode::BAD_REQUEST,
                "この異議申し立てはすでに審査対象ではありません。",
            );
        }
        Ok(None) => {
            return render_action_error(
                StatusCode::NOT_FOUND,
                "審査対象のリスク判定が見つかりません。",
            );
        }
        Err(error) => {
            return render_action_error(StatusCode::BAD_REQUEST, format!("{error:#}").as_str());
        }
    };
    let expected = review.version();
    if let Err(error) = appeal_operation_from_form(form, expected.clone()) {
        return render_action_error(StatusCode::BAD_REQUEST, format!("{error:#}").as_str());
    }
    Html(render_appeal_preview(
        state.csrf_token.as_str(),
        actor,
        form,
        &review,
        &expected,
    ))
    .into_response()
}

async fn apply_appeal_action(state: &AdminState, actor: &str, form: &AdminActionForm) -> Response {
    let expected = match serde_json::from_str::<AppealReviewVersion>(&form.expected_state) {
        Ok(expected) => expected,
        Err(_) => {
            return render_action_error(
                StatusCode::BAD_REQUEST,
                "確認時の状態が欠けています。画面を読み直してもう一度確認してください。",
            );
        }
    };
    let operation = match appeal_operation_from_form(form, expected) {
        Ok(operation) => operation,
        Err(error) => {
            return render_action_error(StatusCode::BAD_REQUEST, format!("{error:#}").as_str());
        }
    };
    match apply_appeal_review_action(
        &state.runtime.pool,
        actor,
        form.target_id.trim(),
        &operation,
        state.operator_review_enabled,
    )
    .await
    {
        Ok(action) => Html(render_action_success(&action)).into_response(),
        Err(error) => {
            tracing::warn!(actor, error = %format!("{error:#}"), "異議申し立て審査を拒否しました");
            render_action_error(StatusCode::BAD_REQUEST, format!("{error:#}").as_str())
        }
    }
}

fn appeal_operation_from_form(
    form: &AdminActionForm,
    expected: AppealReviewVersion,
) -> anyhow::Result<AppealReviewOperation> {
    match form.action.as_str() {
        "appeal.accept" => Ok(AppealReviewOperation::Accept { expected }),
        "appeal.reject" => Ok(AppealReviewOperation::Reject { expected }),
        "appeal.edit" => Ok(AppealReviewOperation::Edit {
            expected,
            edit: RiskSignalMetadataEdit {
                category: parse_optional_enum("category", &form.category)?,
                severity: parse_optional_enum("severity", &form.severity)?,
                confidence: parse_optional_confidence(&form.confidence)?,
                expires_at: parse_optional_expires_at(&form.expires_at)?,
            },
        }),
        "appeal.reissue" => Ok(AppealReviewOperation::Reissue {
            expected,
            correction: RiskSignalCorrection {
                category: parse_optional_enum("category", &form.category)?,
                severity: parse_optional_enum("severity", &form.severity)?,
                confidence: parse_optional_confidence(&form.confidence)?,
                visibility: parse_optional_enum("visibility", &form.visibility)?,
            },
        }),
        _ => bail!("unsupported appeal review operation"),
    }
}

fn parse_optional_enum<T: serde::de::DeserializeOwned>(
    field: &str,
    value: &str,
) -> anyhow::Result<Option<T>> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    serde_json::from_value(serde_json::Value::String(value.to_string()))
        .with_context(|| format!("unsupported {field}"))
        .map(Some)
}

fn parse_optional_confidence(value: &str) -> anyhow::Result<Option<u8>> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let Ok(confidence) = value.parse::<u8>() else {
        bail!("確信度は 0 から 100 の整数で入力してください。");
    };
    if validate_optional_confidence(Some(confidence)).is_err() {
        bail!("確信度は 0 から 100 の整数で入力してください。");
    }
    Ok(Some(confidence))
}

fn parse_optional_expires_at(value: &str) -> anyhow::Result<Option<String>> {
    let Some(value) = optional_text(value) else {
        return Ok(None);
    };
    if validate_optional_expires_at(Some(value.as_str())).is_err() {
        bail!(
            "有効期限は RFC 3339 形式（例: 2026-08-20T09:00:00+09:00）で入力してください。過去時刻は失効として受理されます。"
        );
    }
    Ok(Some(value))
}

fn optional_text(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

#[derive(Debug)]
struct DashboardView {
    admission_mode: String,
    readiness: String,
    topics: Vec<(String, String, String)>,
    reports: Vec<ReportView>,
    appeals: Vec<AppealReview>,
    audit: Vec<AuditView>,
    gcp_project: Option<String>,
    actor: Option<String>,
    csrf_token: String,
    operator_review_enabled: bool,
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
    let appeals = list_appeal_reviews(&state.runtime.pool, 50, 0).await?;
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
        appeals,
        audit,
        gcp_project: std::env::var("COMMUNITY_NODE_GCP_PROJECT")
            .ok()
            .filter(|value| !value.trim().is_empty()),
        actor: state.actor.clone(),
        csrf_token: state.csrf_token.clone(),
        operator_review_enabled: state.operator_review_enabled,
    })
}

fn render_dashboard(view: &DashboardView) -> String {
    let write_enabled = view.actor.is_some();
    let admission_control = view.actor.as_ref().map_or_else(
        || "<p class=\"boundary\">現在は参照専用です。変更するには、確認済みの配備手順で <code>COMMUNITY_NODE_ADMIN_ACTOR</code> を設定してください。</p>".to_string(),
        |actor| {
            format!(
                r#"<form method="post" action="/actions/preview" class="inline-form">
<input type="hidden" name="csrf_token" value="{}"><input type="hidden" name="action" value="admission.set_mode">
<label>変更後の受け入れ方式 <select name="value">{}</select></label><button type="submit">変更内容を確認</button>
</form><p class="meta">操作記録に残る運営者: <code>{}</code></p>"#,
                escape_html(&view.csrf_token),
                admission_options(&view.admission_mode),
                escape_html(actor),
            )
        },
    );
    let topics = if view.topics.is_empty() {
        table_empty("対応トピックは設定されていません。", 4)
    } else {
        view.topics
            .iter()
            .map(|(kind, id, created_at)| {
                let action = if write_enabled && kind == "public_topic" {
                    format!(
                        r#"<form method="post" action="/actions/preview"><input type="hidden" name="csrf_token" value="{}"><input type="hidden" name="action" value="supported_topic.remove"><input type="hidden" name="target_id" value="{}"><button class="secondary" type="submit">削除内容を確認</button></form>"#,
                        escape_html(&view.csrf_token),
                        escape_html(id),
                    )
                } else {
                    "<span class=\"meta\">参照専用</span>".to_string()
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
            r#"<form method="post" action="/actions/preview" class="inline-form"><input type="hidden" name="csrf_token" value="{}"><input type="hidden" name="action" value="supported_topic.add"><label>公開トピック識別子 <input name="target_id" required placeholder="demo"></label><button type="submit">追加内容を確認</button></form>"#,
            escape_html(&view.csrf_token),
        )
    } else {
        String::new()
    };
    let reports = if view.reports.is_empty() {
        table_empty("受信済みの通報はありません。", 7)
    } else {
        view.reports
            .iter()
            .map(|report| {
                let action = if write_enabled {
                    format!(
                        r#"<form method="post" action="/actions/preview" class="compact-form"><input type="hidden" name="csrf_token" value="{}"><input type="hidden" name="action" value="report.set_status"><input type="hidden" name="target_id" value="{}"><select aria-label="変更後の状態" name="value">{}</select><button class="secondary" type="submit">変更内容を確認</button></form>"#,
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
    let appeals = render_appeal_reviews(
        &view.appeals,
        view.actor.is_some() && view.operator_review_enabled,
        &view.csrf_token,
    );
    let audit = if view.audit.is_empty() {
        table_empty("画面から行った操作記録はありません。", 6)
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
        || "<p><code>COMMUNITY_NODE_GCP_PROJECT</code> を設定するとログへのリンクを表示します。</p>".to_string(),
        |project| format!(
            "<p><a href=\"https://console.cloud.google.com/logs/query?project={}\" target=\"_blank\" rel=\"noreferrer\">運用ログを開く</a></p>",
            escape_html(project)
        ),
    );

    format!(
        r#"<!doctype html>
<html lang="ja"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>kukuri コミュニティノード運営</title><style>
	:root{{color-scheme:dark light;font-family:system-ui,sans-serif;--surface:#101923;--panel:#162231;--raised:#233241;--text:#f6f1e8;--muted:#cbbdae;--border:#39495a;--primary:#f59d62;--primary-text:#0e1b26;--focus:#00b3a4;--warning:#e6b066}}@media(prefers-color-scheme:light){{:root{{--surface:#f4efe6;--panel:#fff;--raised:#dfe6ec;--text:#21303b;--muted:#5f6c76;--border:#b7c2cb;--primary:#d77d45;--primary-text:#fff7ef;--focus:#0f8c82;--warning:#9a6e2a}}}}*{{box-sizing:border-box}}body{{max-width:1240px;margin:0 auto;padding:24px;line-height:1.5;background:var(--surface);color:var(--text)}}header,section,.dialog{{border:1px solid var(--border);border-radius:18px;padding:18px;margin-bottom:18px;background:var(--panel)}}.metrics{{display:grid;grid-template-columns:repeat(auto-fit,minmax(240px,1fr));gap:12px}}.metric{{background:var(--raised);border-radius:12px;padding:12px}}table{{border-collapse:collapse;width:100%;display:block;overflow:auto}}th,td{{border-bottom:1px solid var(--border);padding:9px;text-align:left;vertical-align:top}}code{{overflow-wrap:anywhere}}.boundary{{border-left:4px solid var(--warning);padding-left:12px}}a{{color:var(--focus)}}button{{min-height:44px;border:0;border-radius:999px;padding:8px 14px;background:var(--primary);color:var(--primary-text);font-weight:700;cursor:pointer}}button.secondary{{background:var(--raised);color:var(--text)}}input,select{{min-height:44px;border:1px solid var(--border);border-radius:10px;padding:7px 10px;background:var(--surface);color:var(--text)}}.inline-form,.compact-form,.review-actions{{display:flex;flex-wrap:wrap;align-items:end;gap:10px;margin:12px 0}}.compact-form{{min-width:270px}}label{{display:grid;gap:4px}}.meta{{color:var(--muted);font-size:.875rem}}.danger{{color:var(--warning)}}.appeal-card{{border:1px solid var(--border);border-radius:14px;padding:14px;margin:14px 0;background:var(--surface)}}.appeal-grid{{display:grid;grid-template-columns:minmax(100px,auto) minmax(0,1fr);gap:6px 12px}}.appeal-grid dt{{font-weight:700}}.appeal-grid dd{{margin:0}}.edit-form{{display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:10px;width:100%;padding:12px;border:1px solid var(--border);border-radius:12px}}@media(max-width:720px){{body{{padding:12px}}.appeal-grid{{grid-template-columns:1fr}}}}
</style></head><body>
<header><h1>コミュニティノード運営</h1><p>運営者専用の画面です。接続は <code>GCP IAP</code> の転送と権限管理で制限されます。</p></header>
<section><h2>稼働状態と受け入れ方式</h2><div class="metrics"><div class="metric"><strong>利用者向け接続先</strong><br>稼働中</div><div class="metric"><strong>受け入れ方式</strong><br>{}</div><div class="metric"><strong>直近の準備確認</strong><br>{}</div></div>{}</section>
<section><h2>対応トピック</h2><p>この画面で変更できるのは公開トピックだけです。非公開チャンネルの権限は変更できません。</p>{}<table><thead><tr><th>種類</th><th>識別子</th><th>作成時刻</th><th>操作</th></tr></thead><tbody>{}</tbody></table></section>
<section><h2>最近の通報</h2><p>新しい順に 50 件を表示します。補足説明と連絡先は、この一覧と操作記録には表示しません。</p><table><thead><tr><th>受信時刻</th><th>識別子</th><th>対象</th><th>機能</th><th>理由</th><th>状態</th><th>操作</th></tr></thead><tbody>{}</tbody></table></section>
	<section><h2>異議申し立ての審査</h2>{}</section>
	<section><h2>運営操作の記録</h2><p>画面から行った直近 50 件の操作です。データベースで追記専用に保護されています。</p><table><thead><tr><th>実行時刻</th><th>識別子</th><th>運営者</th><th>操作</th><th>対象</th><th>変更</th></tr></thead><tbody>{}</tbody></table></section>
<section><h2>運用ログ</h2><p>実行時ログは既存のログ基盤に残します。この画面が秘密情報や生ログを複製することはありません。</p>{}<p><code>sudo journalctl -u kukuri-readiness.service -n 100 --no-pager</code></p></section>
<section><h2>配備との責任境界</h2><p class="boundary">プロバイダーや言語モデルの認証情報、機能の有効化、配備版、非公開チャンネルの秘密、招待符号、許可一覧、禁止一覧はこの画面から変更できません。確認済みの <code>operator-config.yaml</code>、<code>Terraform</code>、秘密管理、<code>cn-cli readiness</code> を使ってください。画面からの変更は、内容確認と明示的な確定を必須にします。</p></section>
</body></html>"#,
        escape_html(&view.admission_mode),
        escape_html(&view.readiness),
        admission_control,
        add_topic,
        topics,
        reports,
        appeals,
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

pub(crate) fn option(value: &str, label: &str, selected: bool) -> String {
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
        r#"<div class="dialog"><p class="meta">適用前の確認</p><h1>{}</h1><dl><dt>運営者</dt><dd><code>{}</code></dd><dt>対象</dt><dd><code>{}</code></dd><dt>影響</dt><dd>{}</dd></dl><p class="boundary">適用時に現在値と入力をもう一度検証します。状態変更と操作記録は一つの取引で確定します。</p><form method="post" action="/actions/apply"><input type="hidden" name="csrf_token" value="{}"><input type="hidden" name="action" value="{}"><input type="hidden" name="target_id" value="{}"><input type="hidden" name="value" value="{}"><button type="submit">確認して適用</button> <a href="/">取り消す</a></form></div>"#,
        escape_html(title),
        escape_html(actor),
        escape_html(&target),
        escape_html(&impact),
        escape_html(csrf_token),
        escape_html(&form.action),
        escape_html(&form.target_id),
        escape_html(&form.value),
    );
    render_simple_page("運営操作の確認", body.as_str())
}

fn render_appeal_preview(
    csrf_token: &str,
    actor: &str,
    form: &AdminActionForm,
    review: &AppealReview,
    expected: &AppealReviewVersion,
) -> String {
    let (title, impact) = match form.action.as_str() {
        "appeal.accept" => (
            "異議申し立てを認容しますか",
            "このリスク判定を Cleared にし、関連通報を処理済みにします。次回の信頼評価から、この判定の寄与は除外されます。",
        ),
        "appeal.reject" => (
            "異議申し立てを棄却しますか",
            "このリスク判定を None に戻し、関連通報を棄却済みにします。信頼評価への寄与は残ります。",
        ),
        "appeal.edit" => (
            "検知情報を調整しますか",
            "署名済みモデレーション事象と利用者の正本状態は変更せず、このノードのリスク判定だけを調整します。異議申し立ては審査中のままです。",
        ),
        "appeal.reissue" => (
            "訂正版を再発行しますか",
            "現在のリスク判定を失効させ、指定した検知情報と公開範囲で新しいリスク判定を発行します。異議申し立ては審査中のままです。",
        ),
        _ => ("操作を確認しますか", "指定した審査操作を適用します。"),
    };
    let expected_state = serde_json::to_string(expected).expect("appeal review version serializes");
    let fields = [
        ("category", form.category.as_str()),
        ("severity", form.severity.as_str()),
        ("confidence", form.confidence.as_str()),
        ("expires_at", form.expires_at.as_str()),
        ("visibility", form.visibility.as_str()),
    ]
    .into_iter()
    .map(|(name, value)| {
        format!(
            "<input type=\"hidden\" name=\"{}\" value=\"{}\">",
            name,
            escape_html(value)
        )
    })
    .collect::<String>();
    let body = format!(
        r#"<div class="dialog"><p class="meta">適用前の確認</p><h1>{}</h1><dl><dt>運営者</dt><dd><code>{}</code></dd><dt>リスク判定</dt><dd><code>{}</code></dd><dt>対象</dt><dd><code>{}/{}</code></dd><dt>現在の状態</dt><dd>{}</dd><dt>影響</dt><dd>{}</dd></dl><p class="boundary">適用時に現在値をもう一度取得し、確認後に変更されていた場合は拒否します。リスク判定、関連通報、操作記録は一つの取引で確定します。</p><form method="post" action="/actions/apply"><input type="hidden" name="csrf_token" value="{}"><input type="hidden" name="action" value="{}"><input type="hidden" name="target_id" value="{}"><input type="hidden" name="expected_state" value="{}">{}<button type="submit">確認して適用</button> <a href="/">取り消す</a></form></div>"#,
        escape_html(title),
        escape_html(actor),
        escape_html(&review.risk_signal_id),
        escape_html(&review.target),
        escape_html(&review.target_id),
        escape_html(&review.appeal_status),
        escape_html(impact),
        escape_html(csrf_token),
        escape_html(&form.action),
        escape_html(&form.target_id),
        escape_html(&expected_state),
        fields,
    );
    render_simple_page("異議申し立て審査の確認", body.as_str())
}

fn operation_summary(operation: &AdminOperation) -> (&'static str, String, String) {
    match operation {
        AdminOperation::SetAdmissionMode { mode } => (
            "受け入れ方式を変更しますか",
            "community-node admission".to_string(),
            format!(
                "受け入れ方式を {} に変更します。現在接続中の利用者は維持されます。",
                mode.as_str()
            ),
        ),
        AdminOperation::AddSupportedPublicTopic { topic_id } => (
            "対応する公開トピックを追加しますか",
            topic_id.clone(),
            "この公開トピックの取り込みを許可します。安全性確認と準備確認は引き続き適用されます。"
                .to_string(),
        ),
        AdminOperation::RemoveSupportedPublicTopic { topic_id } => (
            "対応する公開トピックを削除しますか",
            topic_id.clone(),
            "この範囲の今後の取り込みを停止します。索引からの除去と処理状態の整合は非同期です。"
                .to_string(),
        ),
        AdminOperation::SetReportStatus { report_id, status } => (
            "通報の状態を変更しますか",
            report_id.clone(),
            format!(
                "このノード内の通報状態を {} に変更します。",
                status.as_str()
            ),
        ),
    }
}

fn render_action_success(action: &kukuri_cn_core::OperatorAction) -> String {
    let body = format!(
        r#"<div class="dialog"><p class="meta">操作完了</p><h1>運営操作を適用しました</h1><dl><dt>操作記録の識別子</dt><dd><code>{}</code></dd><dt>運営者</dt><dd><code>{}</code></dd><dt>操作</dt><dd>{}</dd><dt>対象</dt><dd><code>{}/{}</code></dd><dt>変更</dt><dd><code>{} -&gt; {}</code></dd></dl><p><a href="/">運営画面へ戻る</a></p></div>"#,
        escape_html(&action.id),
        escape_html(&action.actor),
        escape_html(&action.action),
        escape_html(&action.target_kind),
        escape_html(&action.target_id),
        escape_html(&action.before.to_string()),
        escape_html(&action.after.to_string()),
    );
    render_simple_page("運営操作を適用しました", body.as_str())
}

fn render_action_error(status: StatusCode, message: &str) -> Response {
    let body = format!(
        r#"<div class="dialog"><p class="meta">未適用</p><h1>運営操作を拒否しました</h1><p class="danger">{}</p><p>状態と操作記録は変更されていません。</p><p><a href="/">運営画面へ戻る</a></p></div>"#,
        escape_html(message)
    );
    (
        status,
        Html(render_simple_page("運営操作を拒否しました", body.as_str())),
    )
        .into_response()
}

fn render_simple_page(title: &str, body: &str) -> String {
    format!(
        r#"<!doctype html><html lang="ja"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{}</title><style>:root{{color-scheme:dark light;font-family:system-ui,sans-serif}}body{{max-width:760px;margin:0 auto;padding:24px;line-height:1.5}}.dialog{{border:1px solid currentColor;border-radius:18px;padding:20px}}code{{overflow-wrap:anywhere}}.meta{{opacity:.72}}.boundary{{border-left:4px solid #d59b16;padding-left:12px}}button{{min-height:44px;border:0;border-radius:999px;padding:10px 16px;font-weight:700}}dt{{font-weight:700;margin-top:10px}}dd{{margin-left:0}}</style></head><body>{}</body></html>"#,
        escape_html(title),
        body,
    )
}

fn render_error() -> String {
    "<!doctype html><html lang=\"ja\"><head><meta charset=\"utf-8\"><title>kukuri 運営画面を表示できません</title></head><body><h1>運営情報を取得できません</h1><p>運用手順に従い、IAP 経由で cn-user-api のログを確認してください。</p></body></html>".to_string()
}

pub(crate) fn escape_html(value: &str) -> String {
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
            appeals: vec![AppealReview {
                risk_signal_id: "signal-<script>".to_string(),
                issuer_node_id: "issuer<&>".to_string(),
                target: "post_id".to_string(),
                target_id: "target<script>".to_string(),
                category: "nsfw".to_string(),
                severity: "high".to_string(),
                basis: "classifier_score".to_string(),
                confidence: Some(90),
                visibility: "local".to_string(),
                expires_at: None,
                appeal_status: "disputed".to_string(),
                reports: vec![kukuri_cn_core::AppealReviewReport {
                    id: "report-appeal".to_string(),
                    details: Some("<img src=x onerror=alert(1)>".to_string()),
                    status: "received".to_string(),
                    created_at: "2026-08-15T00:00:00Z".parse().expect("timestamp"),
                }],
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
            operator_review_enabled: true,
        });

        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("event/&amp;danger"));
        assert!(html.contains("&quot;quoted&quot;"));
        assert!(!html.contains("<img src=x onerror=alert(1)>"));
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
    }

    #[test]
    fn admin_form_requires_csrf_and_maps_only_supported_runtime_operations() {
        let form = AdminActionForm {
            csrf_token: "token".to_string(),
            action: "admission.set_mode".to_string(),
            target_id: String::new(),
            value: "invite".to_string(),
            ..AdminActionForm::default()
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
                ..AdminActionForm::default()
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

    fn disputed_appeal_version() -> AppealReviewVersion {
        AppealReviewVersion {
            appeal_status: "disputed".to_string(),
            category: "nsfw".to_string(),
            severity: "high".to_string(),
            confidence: Some(90),
            visibility: "local".to_string(),
            expires_at: None,
            reports: vec![("report-1".to_string(), "received".to_string())],
        }
    }

    #[test]
    fn appeal_form_maps_only_supported_review_operations() {
        let expected = disputed_appeal_version();
        let form = AdminActionForm {
            action: "appeal.edit".to_string(),
            target_id: "signal-1".to_string(),
            category: "spam".to_string(),
            severity: "low".to_string(),
            confidence: "20".to_string(),
            ..AdminActionForm::default()
        };
        assert!(matches!(
            appeal_operation_from_form(&form, expected.clone()).expect("edit"),
            AppealReviewOperation::Edit { .. }
        ));
        let mut unsupported = form;
        unsupported.category = "not-a-category".to_string();
        assert!(appeal_operation_from_form(&unsupported, expected).is_err());
    }

    #[test]
    fn appeal_form_rejects_invalid_expires_at_with_japanese_message() {
        // #700: RFC 3339 でない有効期限は解析段階（確認・適用の両方が通る経路）で拒否する。
        let expected = disputed_appeal_version();
        let mut form = AdminActionForm {
            action: "appeal.edit".to_string(),
            target_id: "signal-1".to_string(),
            expires_at: "not-a-timestamp".to_string(),
            ..AdminActionForm::default()
        };
        let error = appeal_operation_from_form(&form, expected.clone()).unwrap_err();
        assert!(error.to_string().contains("RFC 3339"), "{error}");
        assert!(error.to_string().contains("有効期限"), "{error}");

        // 妥当な RFC 3339（時差付き・過去を含む）は受理される。
        for valid in [
            "2026-08-20T09:00:00Z",
            "2026-08-20T18:00:00+09:00",
            "2000-01-01T00:00:00Z",
        ] {
            form.expires_at = valid.to_string();
            assert!(
                appeal_operation_from_form(&form, expected.clone()).is_ok(),
                "{valid} は受理されるべき"
            );
        }
    }

    #[test]
    fn appeal_form_rejects_out_of_range_confidence_with_japanese_message() {
        let expected = disputed_appeal_version();
        // 再発行（適用時にも同じ解析を通る）でも範囲外・数値以外は拒否される。
        for invalid in ["101", "255", "abc", "-1"] {
            let form = AdminActionForm {
                action: "appeal.reissue".to_string(),
                target_id: "signal-1".to_string(),
                confidence: invalid.to_string(),
                ..AdminActionForm::default()
            };
            let error = appeal_operation_from_form(&form, expected.clone()).unwrap_err();
            assert!(
                error.to_string().contains("0 から 100"),
                "{invalid}: {error}"
            );
        }
    }
}
