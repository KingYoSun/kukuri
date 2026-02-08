use anyhow::{anyhow, Result};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use cn_core::{
    config as env_config, db, health, http, logging, metrics, node_key, nostr, server,
    service_config, trust as trust_core,
};
use nostr_sdk::prelude::Keys;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{postgres::PgListener, Pool, Postgres, Row, Transaction};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

mod config;

const SERVICE_NAME: &str = "cn-trust";
const CONSUMER_NAME: &str = "trust-v1";
const OUTBOX_CHANNEL: &str = "cn_relay_outbox";
const GRAPH_NAME: &str = "kukuri_cn";

const JOB_REPORT_BASED: &str = "report_based";
const JOB_COMMUNICATION: &str = "communication_density";

#[derive(Clone)]
struct AppState {
    pool: Pool<Postgres>,
    config: service_config::ServiceConfigHandle,
    node_keys: Keys,
    health_targets: Arc<HashMap<String, String>>,
    health_client: reqwest::Client,
}

#[derive(Serialize)]
struct HealthStatus {
    status: String,
}

#[derive(Clone)]
pub struct TrustConfig {
    pub addr: SocketAddr,
    pub database_url: String,
    pub node_key_path: PathBuf,
    pub config_poll_seconds: u64,
}

#[derive(Deserialize)]
struct OutboxRow {
    seq: i64,
    op: String,
    event_id: String,
    topic_id: String,
    kind: i32,
    created_at: i64,
}

struct TrustEvent {
    raw: nostr::RawEvent,
    is_deleted: bool,
    is_current: bool,
    is_ephemeral: bool,
    expires_at: Option<i64>,
}

struct TrustJob {
    job_id: String,
    job_type: String,
    subject_pubkey: Option<String>,
}

struct TrustSchedule {
    job_type: String,
    interval_seconds: i64,
}

struct InteractionStats {
    edge_count: i64,
    peer_count: i64,
    weight_sum: f64,
}

pub fn load_config() -> Result<TrustConfig> {
    let addr = env_config::socket_addr_from_env("TRUST_ADDR", "0.0.0.0:8086")?;
    let database_url = env_config::required_env("DATABASE_URL")?;
    let node_key_path = node_key::key_path_from_env("NODE_KEY_PATH", "data/node_key.json")?;
    let config_poll_seconds = std::env::var("TRUST_CONFIG_POLL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(30);
    Ok(TrustConfig {
        addr,
        database_url,
        node_key_path,
        config_poll_seconds,
    })
}

pub async fn run(config: TrustConfig) -> Result<()> {
    logging::init(SERVICE_NAME);
    metrics::init(SERVICE_NAME);

    let pool = db::connect(&config.database_url).await?;
    let node_keys = node_key::load_or_generate(&config.node_key_path)?;

    let default_config = json!({
        "enabled": false,
        "consumer": { "batch_size": 200, "poll_interval_seconds": 5 },
        "report_based": {
            "window_days": 30,
            "report_weight": 1.0,
            "label_weight": 1.0,
            "score_normalization": 10.0
        },
        "communication_density": {
            "window_days": 30,
            "score_normalization": 20.0,
            "interaction_weights": { "1": 1.0, "6": 0.5, "7": 0.3 }
        },
        "attestation": { "exp_seconds": 86400 },
        "jobs": {
            "schedule_poll_seconds": 30,
            "report_based_interval_seconds": 86400,
            "communication_interval_seconds": 86400
        }
    });
    let config_handle = service_config::watch_service_config(
        pool.clone(),
        "trust",
        default_config,
        Duration::from_secs(config.config_poll_seconds),
    )
    .await?;
    let health_targets = Arc::new(health::parse_health_targets(
        "TRUST_HEALTH_TARGETS",
        &[
            ("relay", "RELAY_HEALTH_URL", "http://relay:8082/healthz"),
            (
                "moderation",
                "MODERATION_HEALTH_URL",
                "http://moderation:8085/healthz",
            ),
        ],
    ));
    let health_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let state = AppState {
        pool: pool.clone(),
        config: config_handle,
        node_keys,
        health_targets,
        health_client,
    };

    ensure_graph(&state.pool).await?;

    let snapshot = state.config.get().await;
    let runtime = config::TrustRuntimeConfig::from_json(&snapshot.config_json);
    ensure_job_schedules(&state.pool, &runtime).await?;

    spawn_outbox_consumer(state.clone());
    spawn_job_worker(state.clone());
    spawn_schedule_worker(state.clone());

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/metrics", get(metrics_endpoint))
        .with_state(state);

    let router = http::apply_standard_layers(router, SERVICE_NAME);
    server::serve(config.addr, router).await
}

async fn healthz(State(state): State<AppState>) -> impl IntoResponse {
    let ready = async {
        db::check_ready(&state.pool).await?;
        health::ensure_health_targets_ready(&state.health_client, &state.health_targets).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;

    match ready {
        Ok(_) => (
            StatusCode::OK,
            Json(HealthStatus {
                status: "ok".into(),
            }),
        ),
        Err(err) => {
            tracing::warn!(error = %err, "health check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(HealthStatus {
                    status: "unavailable".into(),
                }),
            )
        }
    }
}

async fn metrics_endpoint(State(state): State<AppState>) -> impl IntoResponse {
    if let Ok(max_seq) =
        sqlx::query_scalar::<_, i64>("SELECT COALESCE(MAX(seq), 0) FROM cn_relay.events_outbox")
            .fetch_one(&state.pool)
            .await
    {
        if let Ok(last_seq) = sqlx::query_scalar::<_, i64>(
            "SELECT last_seq FROM cn_relay.consumer_offsets WHERE consumer = $1",
        )
        .bind(CONSUMER_NAME)
        .fetch_optional(&state.pool)
        .await
        {
            if let Some(last_seq) = last_seq {
                let backlog = max_seq.saturating_sub(last_seq);
                metrics::set_outbox_backlog(SERVICE_NAME, CONSUMER_NAME, backlog);
            }
        }
    }

    metrics::metrics_response(SERVICE_NAME)
}

fn spawn_outbox_consumer(state: AppState) {
    tokio::spawn(async move {
        let mut last_seq = match load_last_seq(&state.pool).await {
            Ok(seq) => seq,
            Err(err) => {
                tracing::error!(error = %err, "failed to load consumer offset");
                return;
            }
        };

        let mut listener = match connect_listener(&state.pool, OUTBOX_CHANNEL).await {
            Ok(listener) => listener,
            Err(err) => {
                tracing::warn!(error = %err, "failed to listen outbox channel");
                return;
            }
        };

        loop {
            let snapshot = state.config.get().await;
            let runtime = config::TrustRuntimeConfig::from_json(&snapshot.config_json);
            if !runtime.enabled {
                tokio::time::sleep(Duration::from_secs(runtime.consumer_poll_seconds.max(5))).await;
                continue;
            }

            match fetch_outbox_batch(&state.pool, last_seq, runtime.consumer_batch_size).await {
                Ok(batch) if batch.is_empty() => {
                    if wait_for_notify(&mut listener, runtime.consumer_poll_seconds)
                        .await
                        .is_err()
                    {
                        match connect_listener(&state.pool, OUTBOX_CHANNEL).await {
                            Ok(new_listener) => listener = new_listener,
                            Err(err) => {
                                tracing::warn!(
                                    error = %err,
                                    "failed to reconnect outbox listener"
                                );
                                tokio::time::sleep(Duration::from_secs(2)).await;
                            }
                        }
                    }
                }
                Ok(batch) => {
                    let mut failed = false;
                    for row in &batch {
                        if let Err(err) = handle_outbox_row(&state, &runtime, row).await {
                            tracing::warn!(
                                error = %err,
                                seq = row.seq,
                                "outbox processing failed"
                            );
                            failed = true;
                            break;
                        }
                        last_seq = row.seq;
                    }
                    if failed {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                    if let Err(err) = commit_last_seq(&state.pool, last_seq).await {
                        tracing::warn!(error = %err, "failed to commit consumer offset");
                    }
                }
                Err(err) => {
                    tracing::warn!(error = %err, "outbox fetch failed");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });
}

fn spawn_job_worker(state: AppState) {
    tokio::spawn(async move {
        loop {
            let snapshot = state.config.get().await;
            let runtime = config::TrustRuntimeConfig::from_json(&snapshot.config_json);
            if !runtime.enabled {
                tokio::time::sleep(Duration::from_secs(runtime.consumer_poll_seconds.max(5))).await;
                continue;
            }

            match claim_job(&state.pool).await {
                Ok(Some(job)) => {
                    let result = process_job(&state, &runtime, &job).await;
                    if let Err(err) = &result {
                        tracing::warn!(error = %err, job_id = %job.job_id, "trust job failed");
                    }
                    finalize_job(
                        &state.pool,
                        &job,
                        result.map(|_| ()).map_err(|err| err.to_string()),
                    )
                    .await
                    .ok();
                }
                Ok(None) => {
                    tokio::time::sleep(Duration::from_secs(runtime.consumer_poll_seconds.max(1)))
                        .await;
                }
                Err(err) => {
                    tracing::warn!(error = %err, "trust job claim failed");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    });
}

fn spawn_schedule_worker(state: AppState) {
    tokio::spawn(async move {
        loop {
            let snapshot = state.config.get().await;
            let runtime = config::TrustRuntimeConfig::from_json(&snapshot.config_json);
            if !runtime.enabled {
                tokio::time::sleep(Duration::from_secs(runtime.schedule_poll_seconds.max(5))).await;
                continue;
            }

            if let Err(err) = ensure_job_schedules(&state.pool, &runtime).await {
                tracing::warn!(error = %err, "failed to ensure trust schedules");
            }

            match load_due_schedules(&state.pool).await {
                Ok(schedules) => {
                    for schedule in schedules {
                        if let Err(err) = enqueue_scheduled_job(&state.pool, &schedule).await {
                            tracing::warn!(
                                error = %err,
                                job_type = %schedule.job_type,
                                "failed to enqueue scheduled job"
                            );
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!(error = %err, "failed to load trust schedules");
                }
            }

            tokio::time::sleep(Duration::from_secs(runtime.schedule_poll_seconds.max(5))).await;
        }
    });
}

async fn connect_listener(pool: &Pool<Postgres>, channel: &str) -> Result<PgListener> {
    let mut listener = PgListener::connect_with(pool).await?;
    listener.listen(channel).await?;
    Ok(listener)
}

async fn wait_for_notify(listener: &mut PgListener, poll_seconds: u64) -> Result<()> {
    tokio::select! {
        notification = listener.recv() => {
            notification?;
            Ok(())
        }
        _ = tokio::time::sleep(Duration::from_secs(poll_seconds.max(1))) => Ok(())
    }
}

async fn load_last_seq(pool: &Pool<Postgres>) -> Result<i64> {
    let last_seq = sqlx::query_scalar::<_, i64>(
        "SELECT last_seq FROM cn_relay.consumer_offsets WHERE consumer = $1",
    )
    .bind(CONSUMER_NAME)
    .fetch_optional(pool)
    .await?;

    if let Some(last_seq) = last_seq {
        return Ok(last_seq);
    }

    sqlx::query("INSERT INTO cn_relay.consumer_offsets (consumer, last_seq) VALUES ($1, 0)")
        .bind(CONSUMER_NAME)
        .execute(pool)
        .await?;

    Ok(0)
}

async fn commit_last_seq(pool: &Pool<Postgres>, last_seq: i64) -> Result<()> {
    sqlx::query(
        "UPDATE cn_relay.consumer_offsets SET last_seq = $1, updated_at = NOW() WHERE consumer = $2",
    )
    .bind(last_seq)
    .bind(CONSUMER_NAME)
    .execute(pool)
    .await?;
    Ok(())
}

async fn fetch_outbox_batch(
    pool: &Pool<Postgres>,
    last_seq: i64,
    batch_size: i64,
) -> Result<Vec<OutboxRow>> {
    let rows = sqlx::query(
        "SELECT seq, op, event_id, topic_id, kind, created_at          FROM cn_relay.events_outbox          WHERE seq > $1          ORDER BY seq ASC          LIMIT $2",
    )
    .bind(last_seq)
    .bind(batch_size)
    .fetch_all(pool)
    .await?;

    let mut batch = Vec::new();
    for row in rows {
        batch.push(OutboxRow {
            seq: row.try_get("seq")?,
            op: row.try_get("op")?,
            event_id: row.try_get("event_id")?,
            topic_id: row.try_get("topic_id")?,
            kind: row.try_get("kind")?,
            created_at: row.try_get("created_at")?,
        });
    }
    Ok(batch)
}

async fn handle_outbox_row(
    state: &AppState,
    runtime: &config::TrustRuntimeConfig,
    row: &OutboxRow,
) -> Result<()> {
    if row.op != "upsert" {
        return Ok(());
    }

    let Some(event) = load_event(&state.pool, &row.event_id).await? else {
        return Ok(());
    };
    let now = cn_core::auth::unix_seconds()? as i64;
    if event.is_deleted
        || !event.is_current
        || event.is_ephemeral
        || event.expires_at.map(|exp| exp <= now).unwrap_or(false)
    {
        return Ok(());
    }

    match row.kind {
        39005 => {
            handle_report_event(state, runtime, row, &event, now).await?;
        }
        39006 => {
            handle_label_event(state, runtime, row, &event, now).await?;
        }
        _ => {
            if runtime.interaction_weights.contains_key(&row.kind) {
                handle_interaction_event(state, runtime, row, &event, now).await?;
            }
        }
    }

    Ok(())
}

async fn handle_report_event(
    state: &AppState,
    runtime: &config::TrustRuntimeConfig,
    row: &OutboxRow,
    event: &TrustEvent,
    now: i64,
) -> Result<()> {
    let Some(target) = event.raw.first_tag_value("target") else {
        return Ok(());
    };
    let reason = event.raw.first_tag_value("reason");

    let mut tx = state.pool.begin().await?;
    init_age_session(&mut tx).await?;

    let subject_pubkey = resolve_subject_pubkey(&mut tx, &target).await?;
    let Some(subject_pubkey) = subject_pubkey else {
        return Ok(());
    };
    if !is_hex_64(&subject_pubkey) || !is_hex_64(&event.raw.pubkey) || !is_hex_64(&event.raw.id) {
        return Ok(());
    }

    let inserted = insert_report_event(
        &mut tx,
        &event.raw.id,
        &subject_pubkey,
        Some(&event.raw.pubkey),
        &target,
        reason.as_deref(),
        None,
        None,
        None,
        row.kind,
        &row.topic_id,
        event.raw.created_at,
    )
    .await?;
    if !inserted {
        tx.commit().await?;
        return Ok(());
    }

    upsert_report_edge(
        &mut tx,
        &event.raw.pubkey,
        &subject_pubkey,
        &event.raw.id,
        row.kind,
        event.raw.created_at,
    )
    .await?;

    update_report_score(&mut tx, &state.node_keys, &subject_pubkey, runtime, now).await?;
    tx.commit().await?;
    Ok(())
}

async fn handle_label_event(
    state: &AppState,
    runtime: &config::TrustRuntimeConfig,
    row: &OutboxRow,
    event: &TrustEvent,
    now: i64,
) -> Result<()> {
    let Some(target) = event.raw.first_tag_value("target") else {
        return Ok(());
    };
    let Some(label) = event.raw.first_tag_value("label") else {
        return Ok(());
    };
    let confidence = event
        .raw
        .first_tag_value("confidence")
        .and_then(|value| value.parse::<f64>().ok());
    let label_exp = event
        .raw
        .first_tag_value("exp")
        .and_then(|value| value.parse::<i64>().ok());

    let mut tx = state.pool.begin().await?;
    init_age_session(&mut tx).await?;

    let subject_pubkey = resolve_subject_pubkey(&mut tx, &target).await?;
    let Some(subject_pubkey) = subject_pubkey else {
        return Ok(());
    };
    if !is_hex_64(&subject_pubkey) || !is_hex_64(&event.raw.pubkey) || !is_hex_64(&event.raw.id) {
        return Ok(());
    }

    let inserted = insert_report_event(
        &mut tx,
        &event.raw.id,
        &subject_pubkey,
        Some(&event.raw.pubkey),
        &target,
        None,
        Some(&label),
        confidence,
        label_exp,
        row.kind,
        &row.topic_id,
        event.raw.created_at,
    )
    .await?;
    if !inserted {
        tx.commit().await?;
        return Ok(());
    }

    upsert_report_edge(
        &mut tx,
        &event.raw.pubkey,
        &subject_pubkey,
        &event.raw.id,
        row.kind,
        event.raw.created_at,
    )
    .await?;

    update_report_score(&mut tx, &state.node_keys, &subject_pubkey, runtime, now).await?;
    tx.commit().await?;
    Ok(())
}

async fn handle_interaction_event(
    state: &AppState,
    runtime: &config::TrustRuntimeConfig,
    row: &OutboxRow,
    event: &TrustEvent,
    now: i64,
) -> Result<()> {
    let scope = event
        .raw
        .first_tag_value("scope")
        .unwrap_or_else(|| "public".into());
    if scope != "public" {
        return Ok(());
    }
    let weight = runtime
        .interaction_weights
        .get(&row.kind)
        .copied()
        .unwrap_or(0.0);
    if weight <= 0.0 {
        return Ok(());
    }

    let targets = event.raw.tag_values("p");
    if targets.is_empty() {
        return Ok(());
    }

    if !is_hex_64(&event.raw.pubkey) || !is_hex_64(&event.raw.id) {
        return Ok(());
    }

    let mut tx = state.pool.begin().await?;
    init_age_session(&mut tx).await?;

    let mut affected = HashSet::new();
    for target in targets {
        if !is_hex_64(&target) || target == event.raw.pubkey {
            continue;
        }
        let inserted = insert_interaction(
            &mut tx,
            &event.raw.id,
            &event.raw.pubkey,
            &target,
            weight,
            &row.topic_id,
            event.raw.created_at,
        )
        .await?;
        if !inserted {
            continue;
        }

        upsert_interaction_edge(
            &mut tx,
            &event.raw.pubkey,
            &target,
            &event.raw.id,
            weight,
            event.raw.created_at,
        )
        .await?;

        affected.insert(event.raw.pubkey.clone());
        affected.insert(target);
    }

    for pubkey in affected {
        update_communication_score(&mut tx, &state.node_keys, &pubkey, runtime, now).await?;
    }

    tx.commit().await?;
    Ok(())
}

async fn load_event(pool: &Pool<Postgres>, event_id: &str) -> Result<Option<TrustEvent>> {
    let row = sqlx::query(
        "SELECT raw_json, is_deleted, is_current, is_ephemeral, expires_at FROM cn_relay.events WHERE event_id = $1",
    )
    .bind(event_id)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let raw_json: serde_json::Value = row.try_get("raw_json")?;
    let raw = nostr::parse_event(&raw_json)?;

    Ok(Some(TrustEvent {
        raw,
        is_deleted: row.try_get("is_deleted")?,
        is_current: row.try_get("is_current")?,
        is_ephemeral: row.try_get("is_ephemeral")?,
        expires_at: row.try_get("expires_at")?,
    }))
}

async fn resolve_subject_pubkey(
    tx: &mut Transaction<'_, Postgres>,
    target: &str,
) -> Result<Option<String>> {
    let target = target.trim();
    if let Some(pubkey) = target.strip_prefix("pubkey:") {
        return Ok(Some(pubkey.to_string()));
    }
    if let Some(event_id) = target.strip_prefix("event:") {
        let pubkey = sqlx::query_scalar::<_, String>(
            "SELECT pubkey FROM cn_relay.events WHERE event_id = $1",
        )
        .bind(event_id)
        .fetch_optional(&mut **tx)
        .await?;
        return Ok(pubkey);
    }
    Ok(None)
}

async fn insert_report_event(
    tx: &mut Transaction<'_, Postgres>,
    event_id: &str,
    subject_pubkey: &str,
    reporter_pubkey: Option<&str>,
    target: &str,
    reason: Option<&str>,
    label: Option<&str>,
    confidence: Option<f64>,
    label_exp: Option<i64>,
    source_kind: i32,
    topic_id: &str,
    created_at: i64,
) -> Result<bool> {
    let result = sqlx::query(
        "INSERT INTO cn_trust.report_events          (event_id, subject_pubkey, reporter_pubkey, target, reason, label, confidence, label_exp, source_kind, topic_id, created_at)          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)          ON CONFLICT (event_id) DO NOTHING",
    )
    .bind(event_id)
    .bind(subject_pubkey)
    .bind(reporter_pubkey)
    .bind(target)
    .bind(reason)
    .bind(label)
    .bind(confidence)
    .bind(label_exp)
    .bind(source_kind)
    .bind(topic_id)
    .bind(created_at)
    .execute(&mut **tx)
    .await?;

    Ok(result.rows_affected() > 0)
}

async fn insert_interaction(
    tx: &mut Transaction<'_, Postgres>,
    event_id: &str,
    actor_pubkey: &str,
    target_pubkey: &str,
    weight: f64,
    topic_id: &str,
    created_at: i64,
) -> Result<bool> {
    let result = sqlx::query(
        "INSERT INTO cn_trust.interactions          (event_id, actor_pubkey, target_pubkey, weight, topic_id, created_at)          VALUES ($1, $2, $3, $4, $5, $6)          ON CONFLICT (event_id, target_pubkey) DO NOTHING",
    )
    .bind(event_id)
    .bind(actor_pubkey)
    .bind(target_pubkey)
    .bind(weight)
    .bind(topic_id)
    .bind(created_at)
    .execute(&mut **tx)
    .await?;

    Ok(result.rows_affected() > 0)
}

async fn update_report_score(
    tx: &mut Transaction<'_, Postgres>,
    node_keys: &Keys,
    subject_pubkey: &str,
    runtime: &config::TrustRuntimeConfig,
    now: i64,
) -> Result<()> {
    let since = now - runtime.report_window_days * 86400;
    let report_count = age_report_count(tx, subject_pubkey, since, 39005).await?;
    let label_count = age_report_count(tx, subject_pubkey, since, 39006).await?;

    let weighted =
        report_count as f64 * runtime.report_weight + label_count as f64 * runtime.label_weight;
    let score = (weighted / runtime.report_score_normalization).min(1.0);

    let mut attestation_id: Option<String> = None;
    let mut attestation_exp: Option<i64> = None;
    if report_count > 0 || label_count > 0 {
        let value = json!({
            "score": score,
            "reports": report_count,
            "labels": label_count,
            "window_days": runtime.report_window_days
        });
        let context = json!({
            "method": trust_core::METHOD_REPORT_BASED,
            "window_days": runtime.report_window_days
        });
        let exp = now + runtime.attestation_exp_seconds;
        let issued = issue_attestation(
            tx,
            node_keys,
            subject_pubkey,
            trust_core::CLAIM_REPORT_BASED,
            score,
            value,
            context,
            exp,
        )
        .await?;
        if let Some((id, exp)) = issued {
            attestation_id = Some(id);
            attestation_exp = Some(exp);
        }
    }

    sqlx::query(
        "INSERT INTO cn_trust.report_scores          (subject_pubkey, score, report_count, label_count, window_start, window_end, attestation_id, attestation_exp)          VALUES ($1, $2, $3, $4, $5, $6, $7, $8)          ON CONFLICT (subject_pubkey) DO UPDATE SET score = EXCLUDED.score,              report_count = EXCLUDED.report_count,              label_count = EXCLUDED.label_count,              window_start = EXCLUDED.window_start,              window_end = EXCLUDED.window_end,              attestation_id = COALESCE(EXCLUDED.attestation_id, cn_trust.report_scores.attestation_id),              attestation_exp = COALESCE(EXCLUDED.attestation_exp, cn_trust.report_scores.attestation_exp),              updated_at = NOW()",
    )
    .bind(subject_pubkey)
    .bind(score)
    .bind(report_count)
    .bind(label_count)
    .bind(since)
    .bind(now)
    .bind(attestation_id)
    .bind(attestation_exp)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn update_communication_score(
    tx: &mut Transaction<'_, Postgres>,
    node_keys: &Keys,
    subject_pubkey: &str,
    runtime: &config::TrustRuntimeConfig,
    now: i64,
) -> Result<()> {
    let since = now - runtime.communication_window_days * 86400;
    let stats = age_interaction_stats(tx, subject_pubkey, since).await?;
    let score = (stats.weight_sum / runtime.communication_score_normalization).min(1.0);

    let mut attestation_id: Option<String> = None;
    let mut attestation_exp: Option<i64> = None;
    if stats.edge_count > 0 {
        let value = json!({
            "score": score,
            "interactions": stats.edge_count,
            "peers": stats.peer_count,
            "window_days": runtime.communication_window_days
        });
        let context = json!({
            "method": trust_core::METHOD_COMMUNICATION_DENSITY,
            "window_days": runtime.communication_window_days
        });
        let exp = now + runtime.attestation_exp_seconds;
        let issued = issue_attestation(
            tx,
            node_keys,
            subject_pubkey,
            trust_core::CLAIM_COMMUNICATION_DENSITY,
            score,
            value,
            context,
            exp,
        )
        .await?;
        if let Some((id, exp)) = issued {
            attestation_id = Some(id);
            attestation_exp = Some(exp);
        }
    }

    sqlx::query(
        "INSERT INTO cn_trust.communication_scores          (subject_pubkey, score, interaction_count, peer_count, window_start, window_end, attestation_id, attestation_exp)          VALUES ($1, $2, $3, $4, $5, $6, $7, $8)          ON CONFLICT (subject_pubkey) DO UPDATE SET score = EXCLUDED.score,              interaction_count = EXCLUDED.interaction_count,              peer_count = EXCLUDED.peer_count,              window_start = EXCLUDED.window_start,              window_end = EXCLUDED.window_end,              attestation_id = COALESCE(EXCLUDED.attestation_id, cn_trust.communication_scores.attestation_id),              attestation_exp = COALESCE(EXCLUDED.attestation_exp, cn_trust.communication_scores.attestation_exp),              updated_at = NOW()",
    )
    .bind(subject_pubkey)
    .bind(score)
    .bind(stats.edge_count)
    .bind(stats.peer_count)
    .bind(since)
    .bind(now)
    .bind(attestation_id)
    .bind(attestation_exp)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn issue_attestation(
    tx: &mut Transaction<'_, Postgres>,
    node_keys: &Keys,
    subject_pubkey: &str,
    claim: &str,
    score: f64,
    value: serde_json::Value,
    context: serde_json::Value,
    exp: i64,
) -> Result<Option<(String, i64)>> {
    let input = trust_core::AttestationInput {
        subject: format!("pubkey:{subject_pubkey}"),
        claim: claim.to_string(),
        score,
        value,
        evidence: Vec::new(),
        context,
        exp,
        topic_id: None,
    };
    let event = trust_core::build_attestation_event(node_keys, &input)?;
    let event_json = serde_json::to_value(&event)?;

    sqlx::query(
        "INSERT INTO cn_trust.attestations          (attestation_id, subject, claim, score, exp, topic_id, issuer_pubkey, value_json, evidence_json, context_json, event_json)          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)          ON CONFLICT (attestation_id) DO NOTHING",
    )
    .bind(&event.id)
    .bind(&input.subject)
    .bind(&input.claim)
    .bind(score)
    .bind(exp)
    .bind::<Option<&str>>(None)
    .bind(&event.pubkey)
    .bind(&input.value)
    .bind(serde_json::json!([]))
    .bind(&input.context)
    .bind(event_json)
    .execute(&mut **tx)
    .await?;

    Ok(Some((event.id, exp)))
}

async fn ensure_graph(pool: &Pool<Postgres>) -> Result<()> {
    let mut tx = pool.begin().await?;
    init_age_session(&mut tx).await?;

    let exists = sqlx::query_scalar::<_, i64>("SELECT 1 FROM ag_catalog.ag_graph WHERE name = $1")
        .bind(GRAPH_NAME)
        .fetch_optional(&mut *tx)
        .await?
        .is_some();

    if !exists {
        sqlx::query("SELECT ag_catalog.create_graph($1)")
            .bind(GRAPH_NAME)
            .execute(&mut *tx)
            .await?;
    }

    let now = cn_core::auth::unix_seconds()? as i64;
    let marker_a = format!("__init__{}", Uuid::new_v4());
    let marker_b = format!("__init__{}", Uuid::new_v4());
    let marker_c = format!("__init__{}", Uuid::new_v4());
    let marker_d = format!("__init__{}", Uuid::new_v4());

    let create_report = format!(
        "CREATE (a:User {{pubkey: '{marker_a}'}})-[:REPORTED {{created_at: {now}, kind: 39005, event_id: '{marker_a}'}}]->(b:User {{pubkey: '{marker_b}'}})"
    );
    cypher_execute(&mut tx, &create_report).await?;
    let delete_report = format!(
        "MATCH (a:User {{pubkey: '{marker_a}'}})-[e:REPORTED]->(b:User {{pubkey: '{marker_b}'}}) DELETE e, a, b"
    );
    cypher_execute(&mut tx, &delete_report).await?;

    let create_interaction = format!(
        "CREATE (a:User {{pubkey: '{marker_c}'}})-[:INTERACTED {{created_at: {now}, weight: 0.0, event_id: '{marker_c}'}}]->(b:User {{pubkey: '{marker_d}'}})"
    );
    cypher_execute(&mut tx, &create_interaction).await?;
    let delete_interaction = format!(
        "MATCH (a:User {{pubkey: '{marker_c}'}})-[e:INTERACTED]->(b:User {{pubkey: '{marker_d}'}}) DELETE e, a, b"
    );
    cypher_execute(&mut tx, &delete_interaction).await?;

    tx.commit().await?;
    Ok(())
}

async fn init_age_session(tx: &mut Transaction<'_, Postgres>) -> Result<()> {
    sqlx::query("LOAD 'age'").execute(&mut **tx).await?;
    sqlx::query(r#"SET search_path = ag_catalog, "$user", public"#)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn upsert_report_edge(
    tx: &mut Transaction<'_, Postgres>,
    reporter_pubkey: &str,
    subject_pubkey: &str,
    event_id: &str,
    kind: i32,
    created_at: i64,
) -> Result<()> {
    if !is_hex_64(reporter_pubkey) || !is_hex_64(subject_pubkey) || !is_hex_64(event_id) {
        return Ok(());
    }
    let query = format!(
        "MERGE (reporter:User {{pubkey: '{reporter_pubkey}'}}) MERGE (subject:User {{pubkey: '{subject_pubkey}'}}) MERGE (reporter)-[e:REPORTED {{event_id: '{event_id}'}}]->(subject) ON CREATE SET e.kind = {kind}, e.created_at = {created_at}"
    );
    cypher_execute(tx, &query).await?;
    Ok(())
}

async fn upsert_interaction_edge(
    tx: &mut Transaction<'_, Postgres>,
    actor_pubkey: &str,
    target_pubkey: &str,
    event_id: &str,
    weight: f64,
    created_at: i64,
) -> Result<()> {
    if !is_hex_64(actor_pubkey) || !is_hex_64(target_pubkey) || !is_hex_64(event_id) {
        return Ok(());
    }
    let query = format!(
        "MERGE (actor:User {{pubkey: '{actor_pubkey}'}}) MERGE (target:User {{pubkey: '{target_pubkey}'}}) MERGE (actor)-[e:INTERACTED {{event_id: '{event_id}'}}]->(target) ON CREATE SET e.weight = {weight}, e.created_at = {created_at}"
    );
    cypher_execute(tx, &query).await?;
    Ok(())
}

async fn cypher_execute(tx: &mut Transaction<'_, Postgres>, query: &str) -> Result<()> {
    let _ = sqlx::query("SELECT * FROM cypher($1, $2) AS (v agtype)")
        .bind(GRAPH_NAME)
        .bind(query)
        .fetch_all(&mut **tx)
        .await?;
    Ok(())
}

async fn age_report_count(
    tx: &mut Transaction<'_, Postgres>,
    subject_pubkey: &str,
    since: i64,
    kind: i32,
) -> Result<i64> {
    if !is_hex_64(subject_pubkey) {
        return Ok(0);
    }
    let query = format!(
        "MATCH (:User)-[e:REPORTED]->(:User {{pubkey: '{subject_pubkey}'}}) WHERE e.created_at >= {since} AND e.kind = {kind} RETURN count(e) AS count_value"
    );
    let row = sqlx::query(
        "SELECT count_value::text AS count_value FROM cypher($1, $2) AS (count_value agtype)",
    )
    .bind(GRAPH_NAME)
    .bind(query)
    .fetch_optional(&mut **tx)
    .await?;

    let Some(row) = row else {
        return Ok(0);
    };
    let count_raw: String = row.try_get("count_value")?;
    Ok(parse_agtype_i64(&count_raw)?)
}

async fn age_interaction_stats(
    tx: &mut Transaction<'_, Postgres>,
    subject_pubkey: &str,
    since: i64,
) -> Result<InteractionStats> {
    if !is_hex_64(subject_pubkey) {
        return Ok(InteractionStats {
            edge_count: 0,
            peer_count: 0,
            weight_sum: 0.0,
        });
    }
    let query = format!(
        "MATCH (a:User {{pubkey: '{subject_pubkey}'}})-[e:INTERACTED]-(b:User) WHERE e.created_at >= {since} RETURN count(e) AS edge_count, count(DISTINCT b) AS peer_count, coalesce(sum(e.weight), 0.0) AS weight_sum"
    );
    let row = sqlx::query(
        "SELECT edge_count::text AS edge_count, peer_count::text AS peer_count, weight_sum::text AS weight_sum FROM cypher($1, $2) AS (edge_count agtype, peer_count agtype, weight_sum agtype)",
    )
    .bind(GRAPH_NAME)
    .bind(query)
    .fetch_optional(&mut **tx)
    .await?;

    let Some(row) = row else {
        return Ok(InteractionStats {
            edge_count: 0,
            peer_count: 0,
            weight_sum: 0.0,
        });
    };
    let edge_raw: String = row.try_get("edge_count")?;
    let peer_raw: String = row.try_get("peer_count")?;
    let weight_raw: String = row.try_get("weight_sum")?;

    Ok(InteractionStats {
        edge_count: parse_agtype_i64(&edge_raw)?,
        peer_count: parse_agtype_i64(&peer_raw)?,
        weight_sum: parse_agtype_f64(&weight_raw)?,
    })
}

async fn claim_job(pool: &Pool<Postgres>) -> Result<Option<TrustJob>> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query(
        "SELECT job_id, job_type, subject_pubkey FROM cn_trust.jobs          WHERE status = 'pending'          ORDER BY requested_at ASC          LIMIT 1          FOR UPDATE SKIP LOCKED",
    )
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = row else {
        tx.commit().await?;
        return Ok(None);
    };

    let job = TrustJob {
        job_id: row.try_get("job_id")?,
        job_type: row.try_get("job_type")?,
        subject_pubkey: row.try_get("subject_pubkey")?,
    };

    sqlx::query(
        "UPDATE cn_trust.jobs          SET status = 'running', started_at = NOW(), updated_at = NOW()          WHERE job_id = $1",
    )
    .bind(&job.job_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Some(job))
}

async fn finalize_job(
    pool: &Pool<Postgres>,
    job: &TrustJob,
    result: Result<(), String>,
) -> Result<()> {
    let (status, error_message) = match result {
        Ok(_) => ("succeeded", None),
        Err(err) => ("failed", Some(err)),
    };

    sqlx::query(
        "UPDATE cn_trust.jobs          SET status = $1, error_message = $2, completed_at = NOW(), updated_at = NOW()          WHERE job_id = $3",
    )
    .bind(status)
    .bind(error_message)
    .bind(&job.job_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn process_job(
    state: &AppState,
    runtime: &config::TrustRuntimeConfig,
    job: &TrustJob,
) -> Result<()> {
    let mut targets = if let Some(pubkey) = &job.subject_pubkey {
        vec![pubkey.clone()]
    } else if job.job_type == JOB_REPORT_BASED {
        list_report_subjects(&state.pool).await?
    } else if job.job_type == JOB_COMMUNICATION {
        list_communication_subjects(&state.pool).await?
    } else {
        return Err(anyhow!("unknown trust job type: {}", job.job_type));
    };

    targets.retain(|pubkey| is_hex_64(pubkey));
    let total = targets.len() as i64;
    update_job_progress(&state.pool, &job.job_id, total, 0).await?;

    let mut processed = 0_i64;
    for pubkey in targets {
        let mut tx = state.pool.begin().await?;
        init_age_session(&mut tx).await?;
        let now = cn_core::auth::unix_seconds()? as i64;
        match job.job_type.as_str() {
            JOB_REPORT_BASED => {
                update_report_score(&mut tx, &state.node_keys, &pubkey, runtime, now).await?
            }
            JOB_COMMUNICATION => {
                update_communication_score(&mut tx, &state.node_keys, &pubkey, runtime, now).await?
            }
            _ => return Err(anyhow!("unknown trust job type: {}", job.job_type)),
        }
        tx.commit().await?;
        processed += 1;
        if processed % 10 == 0 || processed == total {
            update_job_progress(&state.pool, &job.job_id, total, processed).await?;
        }
    }

    Ok(())
}

async fn update_job_progress(
    pool: &Pool<Postgres>,
    job_id: &str,
    total: i64,
    processed: i64,
) -> Result<()> {
    sqlx::query(
        "UPDATE cn_trust.jobs          SET total_targets = $1, processed_targets = $2, updated_at = NOW()          WHERE job_id = $3",
    )
    .bind(total)
    .bind(processed)
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn list_report_subjects(pool: &Pool<Postgres>) -> Result<Vec<String>> {
    let rows = sqlx::query(
        "SELECT DISTINCT subject_pubkey FROM (          SELECT subject_pubkey FROM cn_trust.report_events          UNION          SELECT subject_pubkey FROM cn_trust.report_scores      ) subjects",
    )
    .fetch_all(pool)
    .await?;
    let mut subjects = Vec::new();
    for row in rows {
        subjects.push(row.try_get("subject_pubkey")?);
    }
    Ok(subjects)
}

async fn list_communication_subjects(pool: &Pool<Postgres>) -> Result<Vec<String>> {
    let rows = sqlx::query(
        "SELECT DISTINCT subject_pubkey FROM (          SELECT actor_pubkey AS subject_pubkey FROM cn_trust.interactions          UNION          SELECT target_pubkey AS subject_pubkey FROM cn_trust.interactions          UNION          SELECT subject_pubkey FROM cn_trust.communication_scores      ) subjects",
    )
    .fetch_all(pool)
    .await?;
    let mut subjects = Vec::new();
    for row in rows {
        subjects.push(row.try_get("subject_pubkey")?);
    }
    Ok(subjects)
}

async fn ensure_job_schedules(
    pool: &Pool<Postgres>,
    runtime: &config::TrustRuntimeConfig,
) -> Result<()> {
    ensure_job_schedule(
        pool,
        JOB_REPORT_BASED,
        runtime.report_schedule_interval_seconds,
    )
    .await?;
    ensure_job_schedule(
        pool,
        JOB_COMMUNICATION,
        runtime.communication_schedule_interval_seconds,
    )
    .await?;
    Ok(())
}

async fn ensure_job_schedule(
    pool: &Pool<Postgres>,
    job_type: &str,
    interval_seconds: i64,
) -> Result<()> {
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT interval_seconds FROM cn_trust.job_schedules WHERE job_type = $1",
    )
    .bind(job_type)
    .fetch_optional(pool)
    .await?;

    if let Some(current_interval) = existing {
        if current_interval == interval_seconds {
            return Ok(());
        }
        sqlx::query(
            "UPDATE cn_trust.job_schedules              SET interval_seconds = $1,                  next_run_at = LEAST(next_run_at, NOW() + ($1 * INTERVAL '1 second')),                  updated_at = NOW()              WHERE job_type = $2",
        )
        .bind(interval_seconds)
        .bind(job_type)
        .execute(pool)
        .await?;
        return Ok(());
    }

    sqlx::query(
        "INSERT INTO cn_trust.job_schedules          (job_type, interval_seconds, next_run_at)          VALUES ($1, $2, NOW() + ($2 * INTERVAL '1 second'))",
    )
    .bind(job_type)
    .bind(interval_seconds)
    .execute(pool)
    .await?;
    Ok(())
}

async fn load_due_schedules(pool: &Pool<Postgres>) -> Result<Vec<TrustSchedule>> {
    let rows = sqlx::query(
        "SELECT job_type, interval_seconds FROM cn_trust.job_schedules          WHERE is_enabled = TRUE            AND next_run_at <= NOW()          ORDER BY next_run_at ASC",
    )
    .fetch_all(pool)
    .await?;

    let mut schedules = Vec::new();
    for row in rows {
        schedules.push(TrustSchedule {
            job_type: row.try_get("job_type")?,
            interval_seconds: row.try_get("interval_seconds")?,
        });
    }
    Ok(schedules)
}

async fn enqueue_scheduled_job(pool: &Pool<Postgres>, schedule: &TrustSchedule) -> Result<()> {
    let mut tx = pool.begin().await?;
    let pending = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_trust.jobs          WHERE job_type = $1            AND status IN ('pending', 'running')",
    )
    .bind(&schedule.job_type)
    .fetch_one(&mut *tx)
    .await?;

    if pending == 0 {
        let job_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO cn_trust.jobs              (job_id, job_type, status, requested_by)              VALUES ($1, $2, 'pending', $3)",
        )
        .bind(&job_id)
        .bind(&schedule.job_type)
        .bind("scheduler")
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        "UPDATE cn_trust.job_schedules          SET next_run_at = NOW() + ($1 * INTERVAL '1 second'), updated_at = NOW()          WHERE job_type = $2",
    )
    .bind(schedule.interval_seconds)
    .bind(&schedule.job_type)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

fn is_hex_64(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

fn parse_agtype_i64(value: &str) -> Result<i64> {
    let trimmed = value.trim_matches('"');
    trimmed
        .parse::<i64>()
        .map_err(|err| anyhow!("invalid agtype integer: {err}"))
}

fn parse_agtype_f64(value: &str) -> Result<f64> {
    let trimmed = value.trim_matches('"');
    trimmed
        .parse::<f64>()
        .map_err(|err| anyhow!("invalid agtype float: {err}"))
}

#[cfg(test)]
mod tests {
    use super::{is_hex_64, parse_agtype_f64, parse_agtype_i64};

    #[test]
    fn is_hex_64_accepts_valid_hex() {
        let value = "a1".repeat(32);
        assert!(is_hex_64(&value));
        assert!(!is_hex_64("xyz"));
    }

    #[test]
    fn parse_agtype_i64_handles_quotes() {
        assert_eq!(parse_agtype_i64("42").unwrap(), 42);
        assert_eq!(parse_agtype_i64("\"7\"").unwrap(), 7);
    }

    #[test]
    fn parse_agtype_f64_handles_quotes() {
        assert!((parse_agtype_f64("0.5").unwrap() - 0.5).abs() < f64::EPSILON);
        assert!((parse_agtype_f64("\"1.25\"").unwrap() - 1.25).abs() < f64::EPSILON);
    }
}
