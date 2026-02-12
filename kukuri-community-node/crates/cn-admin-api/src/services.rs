use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Postgres, QueryBuilder, Row};
use std::collections::HashMap;
use std::time::Duration;
use utoipa::ToSchema;

use crate::auth::require_admin;
use crate::{ApiError, ApiResult, AppState};

const SECRET_CONFIG_PREVIEW_LIMIT: usize = 3;
const RELAY_SERVICE_KEY: &str = "relay";

#[derive(Serialize, ToSchema)]
pub struct ServiceInfo {
    pub service: String,
    pub version: i64,
    pub config_json: Value,
    pub updated_at: i64,
    pub updated_by: String,
    pub health: Option<ServiceHealth>,
}

#[derive(Serialize, ToSchema)]
pub struct ServiceHealth {
    pub status: String,
    pub checked_at: i64,
    pub details: Option<Value>,
}

#[derive(Serialize, ToSchema)]
pub struct ServiceConfigResponse {
    pub service: String,
    pub version: i64,
    pub config_json: Value,
    pub updated_at: i64,
    pub updated_by: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateServiceConfigRequest {
    pub config_json: Value,
    pub expected_version: Option<i64>,
}

#[derive(Deserialize)]
pub struct AuditQuery {
    pub action: Option<String>,
    pub target: Option<String>,
    pub since: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct AuditLog {
    pub audit_id: i64,
    pub actor_admin_user_id: String,
    pub action: String,
    pub target: String,
    pub diff_json: Option<Value>,
    pub request_id: Option<String>,
    pub created_at: i64,
}

fn find_secret_config_paths(config_json: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    collect_secret_config_paths(config_json, "", &mut paths);
    paths.sort();
    paths.dedup();
    paths
}

fn collect_secret_config_paths(config_json: &Value, current_path: &str, paths: &mut Vec<String>) {
    match config_json {
        Value::Object(map) => {
            for (key, nested) in map {
                let escaped_key = escape_json_pointer_token(key);
                let next_path = if current_path.is_empty() {
                    format!("/{escaped_key}")
                } else {
                    format!("{current_path}/{escaped_key}")
                };

                if is_secret_like_key(key) {
                    paths.push(next_path.clone());
                }

                collect_secret_config_paths(nested, &next_path, paths);
            }
        }
        Value::Array(items) => {
            for (index, nested) in items.iter().enumerate() {
                let next_path = if current_path.is_empty() {
                    format!("/{index}")
                } else {
                    format!("{current_path}/{index}")
                };
                collect_secret_config_paths(nested, &next_path, paths);
            }
        }
        _ => {}
    }
}

fn escape_json_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

fn is_secret_like_key(key: &str) -> bool {
    let tokens = tokenize_key(key);
    if tokens.is_empty() {
        return false;
    }

    if tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "secret"
                | "password"
                | "passwd"
                | "pwd"
                | "apikey"
                | "secretkey"
                | "privatekey"
                | "masterkey"
                | "accesskey"
                | "clientsecret"
                | "jwtsecret"
                | "authsecret"
                | "signingkey"
                | "encryptionkey"
                | "hmacsecret"
        )
    }) {
        return true;
    }

    contains_token_pair(&tokens, "api", "key")
        || contains_token_pair(&tokens, "private", "key")
        || contains_token_pair(&tokens, "master", "key")
        || contains_token_pair(&tokens, "access", "key")
        || contains_token_pair(&tokens, "client", "secret")
        || contains_token_pair(&tokens, "jwt", "secret")
        || contains_token_pair(&tokens, "auth", "secret")
        || contains_token_pair(&tokens, "signing", "key")
        || contains_token_pair(&tokens, "encryption", "key")
        || contains_token_pair(&tokens, "hmac", "secret")
}

fn tokenize_key(key: &str) -> Vec<String> {
    let chars: Vec<char> = key.chars().collect();
    let mut tokens = Vec::new();
    let mut current = String::new();

    for (index, ch) in chars.iter().enumerate() {
        if ch.is_ascii_alphanumeric() {
            let prev = index.checked_sub(1).and_then(|idx| chars.get(idx));
            let next = chars.get(index + 1);
            let boundary = ch.is_ascii_uppercase()
                && !current.is_empty()
                && (prev.is_some_and(|c| c.is_ascii_lowercase())
                    || (prev.is_some_and(|c| c.is_ascii_uppercase())
                        && next.is_some_and(|c| c.is_ascii_lowercase())));

            if boundary {
                tokens.push(std::mem::take(&mut current));
            }

            current.push(ch.to_ascii_lowercase());
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn contains_token_pair(tokens: &[String], left: &str, right: &str) -> bool {
    let has_left = tokens.iter().any(|token| token == left);
    let has_right = tokens.iter().any(|token| token == right);
    has_left && has_right
}

fn build_secret_config_error_message(paths: &[String]) -> String {
    let preview = paths
        .iter()
        .take(SECRET_CONFIG_PREVIEW_LIMIT)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");

    if paths.len() > SECRET_CONFIG_PREVIEW_LIMIT {
        format!(
            "service config contains secret-like keys at {preview} and {} more. Use environment secrets instead.",
            paths.len() - SECRET_CONFIG_PREVIEW_LIMIT
        )
    } else {
        format!(
            "service config contains secret-like keys at {preview}. Use environment secrets instead."
        )
    }
}

pub async fn list_services(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
) -> ApiResult<Json<Vec<ServiceInfo>>> {
    require_admin(&state, &jar).await?;

    let rows = sqlx::query(
        "SELECT service, version, config_json, updated_at, updated_by FROM cn_admin.service_configs",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let health_rows = sqlx::query(
        "SELECT service, status, checked_at, details_json FROM cn_admin.service_health",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;
    let mut health_map = HashMap::new();
    for row in health_rows {
        let checked_at: chrono::DateTime<chrono::Utc> = row.try_get("checked_at")?;
        health_map.insert(
            row.try_get::<String, _>("service")?,
            ServiceHealth {
                status: row.try_get("status")?,
                checked_at: checked_at.timestamp(),
                details: row.try_get("details_json").ok(),
            },
        );
    }

    let mut services = Vec::new();
    for row in rows {
        let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
        let service: String = row.try_get("service")?;
        services.push(ServiceInfo {
            service: service.clone(),
            version: row.try_get("version")?,
            config_json: row.try_get("config_json")?,
            updated_at: updated_at.timestamp(),
            updated_by: row.try_get("updated_by")?,
            health: health_map.remove(&service),
        });
    }

    Ok(Json(services))
}

pub async fn get_service_config(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Path(service): Path<String>,
) -> ApiResult<Json<ServiceConfigResponse>> {
    require_admin(&state, &jar).await?;

    let row = sqlx::query(
        "SELECT service, version, config_json, updated_at, updated_by FROM cn_admin.service_configs WHERE service = $1",
    )
    .bind(&service)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    let Some(row) = row else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "service not found",
        ));
    };

    let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
    Ok(Json(ServiceConfigResponse {
        service: row.try_get("service")?,
        version: row.try_get("version")?,
        config_json: row.try_get("config_json")?,
        updated_at: updated_at.timestamp(),
        updated_by: row.try_get("updated_by")?,
    }))
}

pub async fn update_service_config(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Path(service): Path<String>,
    Json(payload): Json<UpdateServiceConfigRequest>,
) -> ApiResult<Json<ServiceConfigResponse>> {
    let admin = require_admin(&state, &jar).await?;
    let secret_paths = find_secret_config_paths(&payload.config_json);
    if !secret_paths.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "SECRET_CONFIG_FORBIDDEN",
            build_secret_config_error_message(&secret_paths),
        ));
    }

    let mut tx = state.pool.begin().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    let row = sqlx::query("SELECT version FROM cn_admin.service_configs WHERE service = $1")
        .bind(&service)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    let mut next_version = 1_i64;
    if let Some(row) = row {
        let current_version: i64 = row.try_get("version")?;
        if let Some(expected) = payload.expected_version {
            if expected != current_version {
                return Err(ApiError::new(
                    StatusCode::CONFLICT,
                    "VERSION_MISMATCH",
                    "service config version mismatch",
                ));
            }
        }
        next_version = current_version + 1;
        sqlx::query(
            "UPDATE cn_admin.service_configs              SET config_json = $1, version = $2, updated_at = NOW(), updated_by = $3              WHERE service = $4",
        )
        .bind(&payload.config_json)
        .bind(next_version)
        .bind(&admin.admin_user_id)
        .bind(&service)
        .execute(&mut *tx)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    } else {
        sqlx::query(
            "INSERT INTO cn_admin.service_configs              (service, version, config_json, updated_by)              VALUES ($1, 1, $2, $3)",
        )
        .bind(&service)
        .bind(&payload.config_json)
        .bind(&admin.admin_user_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    }

    crate::log_admin_audit_tx(
        &mut tx,
        &admin.admin_user_id,
        "service_config.update",
        &format!("service:{service}"),
        Some(payload.config_json.clone()),
        None,
    )
    .await?;

    sqlx::query("SELECT pg_notify('cn_admin_config', $1)")
        .bind(format!("{service}:{next_version}"))
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    let row = sqlx::query(
        "SELECT service, version, config_json, updated_at, updated_by FROM cn_admin.service_configs WHERE service = $1",
    )
    .bind(&service)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
    tx.commit().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    Ok(Json(ServiceConfigResponse {
        service: row.try_get("service")?,
        version: row.try_get("version")?,
        config_json: row.try_get("config_json")?,
        updated_at: updated_at.timestamp(),
        updated_by: row.try_get("updated_by")?,
    }))
}

pub async fn list_audit_logs(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Query(query): Query<AuditQuery>,
) -> ApiResult<Json<Vec<AuditLog>>> {
    require_admin(&state, &jar).await?;

    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT audit_id, actor_admin_user_id, action, target, diff_json, request_id, created_at FROM cn_admin.audit_logs WHERE 1=1",
    );
    if let Some(action) = query.action {
        builder.push(" AND action = ");
        builder.push_bind(action);
    }
    if let Some(target) = query.target {
        builder.push(" AND target = ");
        builder.push_bind(target);
    }
    if let Some(since) = query.since {
        builder.push(" AND created_at >= to_timestamp(");
        builder.push_bind(since);
        builder.push(")");
    }
    builder.push(" ORDER BY created_at DESC");
    let limit = query.limit.unwrap_or(200).clamp(1, 1000);
    builder.push(" LIMIT ");
    builder.push(limit.to_string());

    let rows = builder
        .build()
        .fetch_all(&state.pool)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    let mut logs = Vec::new();
    for row in rows {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
        logs.push(AuditLog {
            audit_id: row.try_get("audit_id")?,
            actor_admin_user_id: row.try_get("actor_admin_user_id")?,
            action: row.try_get("action")?,
            target: row.try_get("target")?,
            diff_json: row.try_get("diff_json").ok(),
            request_id: row.try_get("request_id").ok(),
            created_at: created_at.timestamp(),
        });
    }

    Ok(Json(logs))
}

pub fn spawn_health_poll(state: AppState, interval: Duration) {
    tokio::spawn(async move {
        loop {
            poll_health_once(&state).await;
            tokio::time::sleep(interval).await;
        }
    });
}

pub(crate) async fn poll_health_once(state: &AppState) {
    for (service, url) in state.health_targets.iter() {
        let result = state.health_client.get(url).send().await;
        let (status, details) = match result {
            Ok(resp) => {
                let mut details = serde_json::json!({ "status": resp.status().as_u16() });
                if service.eq_ignore_ascii_case(RELAY_SERVICE_KEY) {
                    if let Value::Object(details_map) = &mut details {
                        details_map.insert(
                            "auth_transition".to_string(),
                            collect_relay_auth_transition_details(&state.health_client, url).await,
                        );
                    }
                }

                if resp.status().is_success() {
                    ("healthy".to_string(), Some(details))
                } else {
                    ("degraded".to_string(), Some(details))
                }
            }
            Err(err) => (
                "unreachable".to_string(),
                Some(serde_json::json!({ "error": err.to_string() })),
            ),
        };

        let _ = sqlx::query(
            "INSERT INTO cn_admin.service_health              (service, status, checked_at, details_json)              VALUES ($1, $2, NOW(), $3)              ON CONFLICT (service) DO UPDATE SET status = EXCLUDED.status, checked_at = EXCLUDED.checked_at, details_json = EXCLUDED.details_json",
        )
        .bind(service)
        .bind(status)
        .bind(details)
        .execute(&state.pool)
        .await;
    }
}

async fn collect_relay_auth_transition_details(
    health_client: &reqwest::Client,
    health_url: &str,
) -> Value {
    let metrics_url = metrics_url_from_health_url(health_url);
    let response = match health_client.get(&metrics_url).send().await {
        Ok(response) => response,
        Err(err) => {
            return serde_json::json!({
                "metrics_url": metrics_url,
                "metrics_error": err.to_string(),
            });
        }
    };

    let metrics_status = response.status().as_u16();
    if !response.status().is_success() {
        return serde_json::json!({
            "metrics_url": metrics_url,
            "metrics_status": metrics_status,
            "metrics_error": format!("relay metrics status {}", metrics_status),
        });
    }

    let payload = match response.text().await {
        Ok(payload) => payload,
        Err(err) => {
            return serde_json::json!({
                "metrics_url": metrics_url,
                "metrics_status": metrics_status,
                "metrics_error": err.to_string(),
            });
        }
    };

    let ws_connections = parse_prometheus_metric_sum(&payload, "ws_connections", &[])
        .map(|value| value.round() as i64);
    let ws_unauthenticated_connections =
        parse_prometheus_metric_sum(&payload, "ws_unauthenticated_connections", &[])
            .map(|value| value.round() as i64);
    let ingest_rejected_auth_total =
        parse_prometheus_metric_sum(&payload, "ingest_rejected_total", &[("reason", "auth")])
            .map(|value| value.round() as i64);
    let ws_auth_disconnect_timeout_total = parse_prometheus_metric_sum(
        &payload,
        "ws_auth_disconnect_total",
        &[("reason", "timeout")],
    )
    .map(|value| value.round() as i64);
    let ws_auth_disconnect_deadline_total = parse_prometheus_metric_sum(
        &payload,
        "ws_auth_disconnect_total",
        &[("reason", "deadline")],
    )
    .map(|value| value.round() as i64);

    serde_json::json!({
        "metrics_url": metrics_url,
        "metrics_status": metrics_status,
        "ws_connections": ws_connections,
        "ws_unauthenticated_connections": ws_unauthenticated_connections,
        "ingest_rejected_auth_total": ingest_rejected_auth_total,
        "ws_auth_disconnect_timeout_total": ws_auth_disconnect_timeout_total,
        "ws_auth_disconnect_deadline_total": ws_auth_disconnect_deadline_total,
    })
}

fn metrics_url_from_health_url(health_url: &str) -> String {
    if let Some(prefix) = health_url.strip_suffix("/healthz") {
        return format!("{prefix}/metrics");
    }
    let trimmed = health_url.trim_end_matches('/');
    format!("{trimmed}/metrics")
}

fn parse_prometheus_metric_sum(
    payload: &str,
    metric_name: &str,
    required_labels: &[(&str, &str)],
) -> Option<f64> {
    let mut total = 0.0_f64;
    let mut found = false;

    for line in payload.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || !trimmed.starts_with(metric_name) {
            continue;
        }

        let boundary = trimmed.as_bytes().get(metric_name.len()).copied();
        if boundary != Some(b'{') && boundary != Some(b' ') {
            continue;
        }

        let (labels_section, value_section) = if boundary == Some(b'{') {
            let Some(end_index) = trimmed.find('}') else {
                continue;
            };
            let labels = &trimmed[metric_name.len() + 1..end_index];
            let value = trimmed[end_index + 1..].trim_start();
            (Some(labels), value)
        } else {
            (None, trimmed[metric_name.len()..].trim_start())
        };

        if !metric_line_matches_labels(labels_section, required_labels) {
            continue;
        }

        if let Some(value_token) = value_section.split_ascii_whitespace().next() {
            if let Ok(value) = value_token.parse::<f64>() {
                total += value;
                found = true;
            }
        }
    }

    if found {
        Some(total)
    } else {
        None
    }
}

fn metric_line_matches_labels(
    labels_section: Option<&str>,
    required_labels: &[(&str, &str)],
) -> bool {
    if required_labels.is_empty() {
        return true;
    }

    let Some(labels_section) = labels_section else {
        return false;
    };

    required_labels.iter().all(|(key, value)| {
        let token = format!("{key}=\"{value}\"");
        labels_section.contains(&token)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn find_secret_config_paths_detects_env_style_secret_keys() {
        let paths = find_secret_config_paths(&json!({
            "llm": {
                "OPENAI_API_KEY": "sk-test",
                "provider": "openai"
            }
        }));

        assert_eq!(paths, vec!["/llm/OPENAI_API_KEY"]);
    }

    #[test]
    fn find_secret_config_paths_detects_nested_camel_case_secret_keys() {
        let paths = find_secret_config_paths(&json!({
            "provider": [
                {
                    "credentials": {
                        "clientSecret": "top-secret"
                    }
                }
            ]
        }));

        assert_eq!(paths, vec!["/provider/0/credentials/clientSecret"]);
    }

    #[test]
    fn find_secret_config_paths_ignores_non_secret_keys() {
        let paths = find_secret_config_paths(&json!({
            "llm": {
                "max_tokens": 1024,
                "provider": "openai"
            },
            "retention": {
                "events_days": 30
            }
        }));

        assert!(paths.is_empty());
    }

    #[test]
    fn parse_prometheus_metric_sum_filters_labels_and_ignores_created_suffix() {
        let payload = r#"
# HELP ingest_rejected_total Total ingest messages rejected
# TYPE ingest_rejected_total counter
ingest_rejected_total{service="cn-relay",reason="auth"} 5
ingest_rejected_total{service="cn-relay",reason="ratelimit"} 7
ingest_rejected_total_created{service="cn-relay",reason="auth"} 1738809600
"#;

        let auth_total =
            parse_prometheus_metric_sum(payload, "ingest_rejected_total", &[("reason", "auth")]);
        let all_total = parse_prometheus_metric_sum(payload, "ingest_rejected_total", &[]);

        assert_eq!(auth_total, Some(5.0));
        assert_eq!(all_total, Some(12.0));
    }

    #[test]
    fn parse_prometheus_metric_sum_handles_metrics_without_labels() {
        let payload = r#"
ws_connections{service="cn-relay"} 3
ws_connections{service="cn-relay"} 4
"#;
        let total = parse_prometheus_metric_sum(payload, "ws_connections", &[]);
        assert_eq!(total, Some(7.0));
    }
}
