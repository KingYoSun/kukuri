use axum::extract::State;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use serde::Serialize;
use sqlx::Row;
use utoipa::ToSchema;

use crate::auth::require_admin;
use crate::{ApiResult, AppState};

const OUTBOX_BACKLOG_ALERT_THRESHOLD: i64 = 1_000;
const REJECT_SURGE_ALERT_PER_MINUTE: f64 = 30.0;
const DB_DISK_SOFT_LIMIT_BYTES: i64 = 10 * 1024 * 1024 * 1024;
const DB_CONNECTION_ALERT_RATIO: f64 = 0.85;
const DB_LOCK_WAIT_ALERT_THRESHOLD: i64 = 3;

#[derive(Debug, Default)]
pub(crate) struct DashboardCache {
    pub reject_sample: Option<RejectSample>,
}

#[derive(Debug, Clone)]
pub(crate) struct RejectSample {
    pub total: i64,
    pub collected_at: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DashboardSnapshot {
    pub collected_at: i64,
    pub outbox_backlog: OutboxBacklogSignal,
    pub reject_surge: RejectSurgeSignal,
    pub db_pressure: DbPressureSignal,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OutboxBacklogSignal {
    pub max_seq: i64,
    pub total_backlog: i64,
    pub max_backlog: i64,
    pub threshold: i64,
    pub alert: bool,
    pub consumers: Vec<OutboxConsumerBacklog>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OutboxConsumerBacklog {
    pub consumer: String,
    pub last_seq: i64,
    pub backlog: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RejectSurgeSignal {
    pub source_status: String,
    pub source_error: Option<String>,
    pub current_total: Option<i64>,
    pub previous_total: Option<i64>,
    pub delta: Option<i64>,
    pub per_minute: Option<f64>,
    pub threshold_per_minute: f64,
    pub alert: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DbPressureSignal {
    pub db_size_bytes: i64,
    pub disk_soft_limit_bytes: i64,
    pub disk_utilization: f64,
    pub active_connections: i64,
    pub max_connections: i64,
    pub connection_utilization: f64,
    pub lock_waiters: i64,
    pub connection_threshold: f64,
    pub lock_waiter_threshold: i64,
    pub alert: bool,
    pub alerts: Vec<String>,
}

pub async fn get_dashboard_snapshot(
    State(state): State<AppState>,
    jar: CookieJar,
) -> ApiResult<Json<DashboardSnapshot>> {
    require_admin(&state, &jar).await?;

    let collected_at = chrono::Utc::now().timestamp();
    let outbox_backlog = collect_outbox_backlog(&state).await?;
    let reject_surge = collect_reject_surge(&state, collected_at).await;
    let db_pressure = collect_db_pressure(&state).await?;

    Ok(Json(DashboardSnapshot {
        collected_at,
        outbox_backlog,
        reject_surge,
        db_pressure,
    }))
}

async fn collect_outbox_backlog(state: &AppState) -> ApiResult<OutboxBacklogSignal> {
    let max_seq =
        sqlx::query_scalar::<_, i64>("SELECT COALESCE(MAX(seq), 0) FROM cn_relay.events_outbox")
            .fetch_one(&state.pool)
            .await?;

    let rows = sqlx::query("SELECT consumer, last_seq FROM cn_relay.consumer_offsets")
        .fetch_all(&state.pool)
        .await?;

    let mut consumers = Vec::with_capacity(rows.len());
    for row in rows {
        let consumer: String = row.try_get("consumer")?;
        let last_seq: i64 = row.try_get("last_seq")?;
        let backlog = max_seq.saturating_sub(last_seq);
        consumers.push(OutboxConsumerBacklog {
            consumer,
            last_seq,
            backlog,
        });
    }

    consumers.sort_by(|left, right| {
        right
            .backlog
            .cmp(&left.backlog)
            .then_with(|| left.consumer.cmp(&right.consumer))
    });

    let total_backlog = consumers.iter().map(|row| row.backlog).sum();
    let max_backlog = consumers.iter().map(|row| row.backlog).max().unwrap_or(0);
    let threshold = OUTBOX_BACKLOG_ALERT_THRESHOLD;

    Ok(OutboxBacklogSignal {
        max_seq,
        total_backlog,
        max_backlog,
        threshold,
        alert: max_backlog >= threshold,
        consumers,
    })
}

async fn collect_reject_surge(state: &AppState, collected_at: i64) -> RejectSurgeSignal {
    let threshold_per_minute = REJECT_SURGE_ALERT_PER_MINUTE;
    let mut source_status = "unavailable".to_string();
    let mut source_error = None;
    let mut current_total = None;

    if let Some(metrics_url) = relay_metrics_url(state) {
        match state.health_client.get(metrics_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.text().await {
                        Ok(metrics_payload) => {
                            current_total = parse_prometheus_counter_sum(
                                &metrics_payload,
                                "ingest_rejected_total",
                            )
                            .map(|value| value.round() as i64);
                            source_status = if current_total.is_some() {
                                "ok".to_string()
                            } else {
                                source_error = Some(
                                    "ingest_rejected_total was not found in relay metrics"
                                        .to_string(),
                                );
                                "partial".to_string()
                            };
                        }
                        Err(err) => {
                            source_status = "error".to_string();
                            source_error = Some(err.to_string());
                        }
                    }
                } else {
                    source_status = "error".to_string();
                    source_error = Some(format!(
                        "relay metrics status {}",
                        response.status().as_u16()
                    ));
                }
            }
            Err(err) => {
                source_status = "unreachable".to_string();
                source_error = Some(err.to_string());
            }
        }
    } else {
        source_error = Some("relay health target is not configured".to_string());
    }

    let mut previous_total = None;
    let mut delta = None;
    let mut per_minute = None;

    if let Some(total) = current_total {
        let mut cache = state.dashboard_cache.lock().await;
        if let Some(previous) = cache.reject_sample.as_ref() {
            previous_total = Some(previous.total);
            let diff = total.saturating_sub(previous.total);
            delta = Some(diff);
            let elapsed = collected_at.saturating_sub(previous.collected_at);
            if elapsed > 0 {
                per_minute = Some((diff as f64) * 60.0 / (elapsed as f64));
            }
        }
        cache.reject_sample = Some(RejectSample {
            total,
            collected_at,
        });
    }

    let alert = per_minute
        .map(|value| value >= threshold_per_minute)
        .unwrap_or(false);

    RejectSurgeSignal {
        source_status,
        source_error,
        current_total,
        previous_total,
        delta,
        per_minute,
        threshold_per_minute,
        alert,
    }
}

async fn collect_db_pressure(state: &AppState) -> ApiResult<DbPressureSignal> {
    let db_size_bytes = sqlx::query_scalar::<_, i64>("SELECT pg_database_size(current_database())")
        .fetch_one(&state.pool)
        .await?;
    let active_connections = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::bigint FROM pg_stat_activity WHERE datname = current_database()",
    )
    .fetch_one(&state.pool)
    .await?;
    let max_connections = sqlx::query_scalar::<_, i64>(
        "SELECT setting::bigint FROM pg_settings WHERE name = 'max_connections'",
    )
    .fetch_one(&state.pool)
    .await?;
    let lock_waiters = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::bigint          FROM pg_stat_activity          WHERE datname = current_database()            AND wait_event_type = 'Lock'",
    )
    .fetch_one(&state.pool)
    .await?;

    let disk_soft_limit_bytes = DB_DISK_SOFT_LIMIT_BYTES;
    let connection_threshold = DB_CONNECTION_ALERT_RATIO;
    let lock_waiter_threshold = DB_LOCK_WAIT_ALERT_THRESHOLD;
    let disk_utilization = if disk_soft_limit_bytes > 0 {
        (db_size_bytes as f64) / (disk_soft_limit_bytes as f64)
    } else {
        0.0
    };
    let connection_utilization = if max_connections > 0 {
        (active_connections as f64) / (max_connections as f64)
    } else {
        0.0
    };

    let mut alerts = Vec::new();
    if db_size_bytes >= disk_soft_limit_bytes {
        alerts.push("disk_soft_limit_exceeded".to_string());
    }
    if connection_utilization >= connection_threshold {
        alerts.push("connections_near_capacity".to_string());
    }
    if lock_waiters >= lock_waiter_threshold {
        alerts.push("lock_waiters_high".to_string());
    }

    Ok(DbPressureSignal {
        db_size_bytes,
        disk_soft_limit_bytes,
        disk_utilization,
        active_connections,
        max_connections,
        connection_utilization,
        lock_waiters,
        connection_threshold,
        lock_waiter_threshold,
        alert: !alerts.is_empty(),
        alerts,
    })
}

fn relay_metrics_url(state: &AppState) -> Option<String> {
    let health_url = state.health_targets.get("relay")?;
    if let Some(prefix) = health_url.strip_suffix("/healthz") {
        return Some(format!("{prefix}/metrics"));
    }

    let trimmed = health_url.trim_end_matches('/');
    Some(format!("{trimmed}/metrics"))
}

fn parse_prometheus_counter_sum(payload: &str, metric_name: &str) -> Option<f64> {
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

        if let Some(value_token) = trimmed.split_ascii_whitespace().nth(1) {
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

#[cfg(test)]
mod tests {
    use super::parse_prometheus_counter_sum;

    #[test]
    fn parse_prometheus_counter_sum_ignores_comments_and_created_suffix() {
        let payload = r#"
# HELP ingest_rejected_total Total ingest messages rejected
# TYPE ingest_rejected_total counter
ingest_rejected_total{service="cn-relay",reason="auth"} 5
ingest_rejected_total{service="cn-relay",reason="ratelimit"} 7
ingest_rejected_total_created{service="cn-relay",reason="auth"} 1738809600
"#;

        let total = parse_prometheus_counter_sum(payload, "ingest_rejected_total");
        assert_eq!(total, Some(12.0));
    }
}
