use anyhow::{anyhow, Result};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use cn_core::{
    config as env_config, db, health, http, logging, metrics, moderation as moderation_core,
    node_key, nostr, server, service_config,
};
use nostr_sdk::prelude::Keys;
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{postgres::PgListener, Pool, Postgres, Row};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

mod config;
mod llm;

const SERVICE_NAME: &str = "cn-moderation";
const CONSUMER_NAME: &str = "moderation-v1";
const OUTBOX_CHANNEL: &str = "cn_relay_outbox";
const LLM_POLICY_URL: &str = "https://example.com/policies/llm-moderation-v1";
const LLM_POLICY_REF: &str = "moderation-llm-v1";
const LLM_LABEL_EXP_SECONDS: i64 = 24 * 60 * 60;
const LLM_BUDGET_LOCK_KEY: i64 = 3900601;
const LLM_INFLIGHT_TTL_SECONDS: i64 = 60;
const LLM_COST_PER_1K_CHARS: f64 = 0.0001;

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
pub struct ModerationConfig {
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
}

#[derive(Debug)]
struct ModerationJob {
    job_id: String,
    event_id: String,
    topic_id: String,
    attempts: i32,
    max_attempts: i32,
}

struct ModerationEvent {
    raw: nostr::RawEvent,
    is_deleted: bool,
    is_current: bool,
    is_ephemeral: bool,
    expires_at: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
enum LlmSkipReason {
    RequestsPerDay,
    CostPerDay,
    Concurrency,
}

impl LlmSkipReason {
    fn as_str(self) -> &'static str {
        match self {
            LlmSkipReason::RequestsPerDay => "max_requests_per_day",
            LlmSkipReason::CostPerDay => "max_cost_per_day",
            LlmSkipReason::Concurrency => "max_concurrency",
        }
    }
}

#[derive(Debug, Clone)]
struct LlmUsageSnapshot {
    usage_day: String,
    requests_today: i64,
    cost_today: f64,
    inflight_requests: i64,
}

#[derive(Debug)]
struct LlmExecutionPermit {
    request_id: String,
}

#[derive(Debug)]
enum LlmExecutionGate {
    Acquired(LlmExecutionPermit),
    Skipped {
        reason: LlmSkipReason,
        usage: LlmUsageSnapshot,
        estimated_cost: f64,
    },
}

pub fn load_config() -> Result<ModerationConfig> {
    let addr = env_config::socket_addr_from_env("MODERATION_ADDR", "0.0.0.0:8085")?;
    let database_url = env_config::required_env("DATABASE_URL")?;
    let node_key_path = node_key::key_path_from_env("NODE_KEY_PATH", "data/node_key.json")?;
    let config_poll_seconds = std::env::var("MODERATION_CONFIG_POLL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(30);
    Ok(ModerationConfig {
        addr,
        database_url,
        node_key_path,
        config_poll_seconds,
    })
}

pub async fn run(config: ModerationConfig) -> Result<()> {
    logging::init(SERVICE_NAME);
    metrics::init(SERVICE_NAME);

    let pool = db::connect(&config.database_url).await?;
    let node_keys = node_key::load_or_generate(&config.node_key_path)?;

    let default_config = json!({
        "enabled": false,
        "consumer": { "batch_size": 200, "poll_interval_seconds": 5 },
        "queue": { "max_attempts": 3, "retry_delay_seconds": 30 },
        "rules": { "max_labels_per_event": 5 },
        "llm": {
            "enabled": false,
            "provider": "disabled",
            "external_send_enabled": false,
            "truncate_chars": 2000,
            "mask_pii": true,
            "max_requests_per_day": 0,
            "max_cost_per_day": 0.0,
            "max_concurrency": 1
        }
    });
    let config_handle = service_config::watch_service_config(
        pool.clone(),
        "moderation",
        default_config,
        Duration::from_secs(config.config_poll_seconds),
    )
    .await?;
    let health_targets = Arc::new(health::parse_health_targets(
        "MODERATION_HEALTH_TARGETS",
        &[("relay", "RELAY_HEALTH_URL", "http://relay:8082/healthz")],
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

    spawn_outbox_consumer(state.clone());
    spawn_job_worker(state.clone());

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
        let snapshot = state.config.get().await;
        let runtime = config::ModerationRuntimeConfig::from_json(&snapshot.config_json);
        ensure_llm_dependency_ready(&state.health_client, &runtime.llm).await?;
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

async fn ensure_llm_dependency_ready(
    client: &reqwest::Client,
    llm: &config::LlmRuntimeConfig,
) -> Result<()> {
    if !llm.enabled || llm.provider == "disabled" {
        return Ok(());
    }

    match llm.provider.as_str() {
        "openai" => {
            if !llm.external_send_enabled {
                return Ok(());
            }
            let api_key = std::env::var("OPENAI_API_KEY")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow!("OPENAI_API_KEY is required when provider=openai"))?;
            let endpoint = std::env::var("OPENAI_MODERATION_ENDPOINT")
                .unwrap_or_else(|_| "https://api.openai.com/v1/moderations".to_string());
            health::ensure_endpoint_reachable(client, "llm:openai", &endpoint).await?;
            if api_key.len() < 10 {
                return Err(anyhow!("OPENAI_API_KEY appears invalid"));
            }
            Ok(())
        }
        "local" => {
            let endpoint = std::env::var("LLM_LOCAL_ENDPOINT")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow!("LLM_LOCAL_ENDPOINT is required when provider=local"))?;
            health::ensure_endpoint_reachable(client, "llm:local", &endpoint).await
        }
        other => Err(anyhow!("unsupported llm provider `{other}`")),
    }
}

async fn metrics_endpoint(State(state): State<AppState>) -> impl IntoResponse {
    if let Ok(max_seq) =
        sqlx::query_scalar::<_, i64>("SELECT COALESCE(MAX(seq), 0) FROM cn_relay.events_outbox")
            .fetch_one(&state.pool)
            .await
    {
        if let Ok(Some(last_seq)) = sqlx::query_scalar::<_, i64>(
            "SELECT last_seq FROM cn_relay.consumer_offsets WHERE consumer = $1",
        )
        .bind(CONSUMER_NAME)
        .fetch_optional(&state.pool)
        .await
        {
            let backlog = max_seq.saturating_sub(last_seq);
            metrics::set_outbox_backlog(SERVICE_NAME, CONSUMER_NAME, backlog);
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
            let runtime = config::ModerationRuntimeConfig::from_json(&snapshot.config_json);
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
                                tracing::warn!(error = %err, "failed to reconnect outbox listener");
                                tokio::time::sleep(Duration::from_secs(2)).await;
                            }
                        }
                    }
                }
                Ok(batch) => {
                    let batch_started_at = Instant::now();
                    let batch_size = batch.len();
                    metrics::observe_outbox_consumer_batch_size(
                        SERVICE_NAME,
                        CONSUMER_NAME,
                        batch_size,
                    );
                    let mut failed = false;
                    for row in &batch {
                        if row.op == "upsert" {
                            if let Err(err) =
                                enqueue_job(&state.pool, row, runtime.queue_max_attempts).await
                            {
                                tracing::warn!(
                                    error = %err,
                                    seq = row.seq,
                                    "failed to enqueue moderation job"
                                );
                                failed = true;
                                break;
                            }
                        }
                        last_seq = row.seq;
                    }
                    if failed {
                        metrics::inc_outbox_consumer_batch_total(
                            SERVICE_NAME,
                            CONSUMER_NAME,
                            metrics::OUTBOX_CONSUMER_RESULT_ERROR,
                        );
                        metrics::observe_outbox_consumer_processing_duration(
                            SERVICE_NAME,
                            CONSUMER_NAME,
                            metrics::OUTBOX_CONSUMER_RESULT_ERROR,
                            batch_started_at.elapsed(),
                        );
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                    if let Err(err) = commit_last_seq(&state.pool, last_seq).await {
                        tracing::warn!(error = %err, "failed to commit consumer offset");
                        metrics::inc_outbox_consumer_batch_total(
                            SERVICE_NAME,
                            CONSUMER_NAME,
                            metrics::OUTBOX_CONSUMER_RESULT_ERROR,
                        );
                        metrics::observe_outbox_consumer_processing_duration(
                            SERVICE_NAME,
                            CONSUMER_NAME,
                            metrics::OUTBOX_CONSUMER_RESULT_ERROR,
                            batch_started_at.elapsed(),
                        );
                    } else {
                        metrics::inc_outbox_consumer_batch_total(
                            SERVICE_NAME,
                            CONSUMER_NAME,
                            metrics::OUTBOX_CONSUMER_RESULT_SUCCESS,
                        );
                        metrics::observe_outbox_consumer_processing_duration(
                            SERVICE_NAME,
                            CONSUMER_NAME,
                            metrics::OUTBOX_CONSUMER_RESULT_SUCCESS,
                            batch_started_at.elapsed(),
                        );
                    }
                }
                Err(err) => {
                    tracing::warn!(error = %err, "outbox fetch failed");
                    metrics::inc_outbox_consumer_batch_total(
                        SERVICE_NAME,
                        CONSUMER_NAME,
                        metrics::OUTBOX_CONSUMER_RESULT_ERROR,
                    );
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
            let runtime = config::ModerationRuntimeConfig::from_json(&snapshot.config_json);
            if !runtime.enabled {
                tokio::time::sleep(Duration::from_secs(runtime.consumer_poll_seconds.max(5))).await;
                continue;
            }

            match claim_job(&state.pool).await {
                Ok(Some(job)) => {
                    let result = process_job(&state, &runtime, &job).await;
                    if let Err(err) = &result {
                        tracing::warn!(error = %err, job_id = %job.job_id, "moderation job failed");
                    }
                    finalize_job(
                        &state.pool,
                        &job,
                        &runtime,
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
                    tracing::warn!(error = %err, "moderation job claim failed");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
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
        "SELECT seq, op, event_id, topic_id          FROM cn_relay.events_outbox          WHERE seq > $1          ORDER BY seq ASC          LIMIT $2",
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
        });
    }
    Ok(batch)
}

async fn enqueue_job(pool: &Pool<Postgres>, row: &OutboxRow, max_attempts: i32) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_moderation.jobs          (job_id, event_id, topic_id, source, status, max_attempts)          VALUES ($1, $2, $3, $4, 'pending', $5)          ON CONFLICT (event_id, topic_id) DO NOTHING",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&row.event_id)
    .bind(&row.topic_id)
    .bind("outbox")
    .bind(max_attempts)
    .execute(pool)
    .await?;
    Ok(())
}

async fn claim_job(pool: &Pool<Postgres>) -> Result<Option<ModerationJob>> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query(
        "SELECT job_id, event_id, topic_id, attempts, max_attempts          FROM cn_moderation.jobs          WHERE status = 'pending'            AND next_run_at <= NOW()          ORDER BY next_run_at ASC, created_at ASC          LIMIT 1          FOR UPDATE SKIP LOCKED",
    )
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = row else {
        tx.commit().await?;
        return Ok(None);
    };

    let job_id: String = row.try_get("job_id")?;
    let attempts: i32 = row.try_get("attempts")?;
    let max_attempts: i32 = row.try_get("max_attempts")?;

    sqlx::query(
        "UPDATE cn_moderation.jobs          SET status = 'running', attempts = $1, started_at = NOW(), updated_at = NOW()          WHERE job_id = $2",
    )
    .bind(attempts + 1)
    .bind(&job_id)
    .execute(&mut *tx)
    .await?;

    let job = ModerationJob {
        job_id,
        event_id: row.try_get("event_id")?,
        topic_id: row.try_get("topic_id")?,
        attempts: attempts + 1,
        max_attempts,
    };
    tx.commit().await?;
    Ok(Some(job))
}

async fn finalize_job(
    pool: &Pool<Postgres>,
    job: &ModerationJob,
    runtime: &config::ModerationRuntimeConfig,
    result: Result<(), String>,
) -> Result<()> {
    match result {
        Ok(_) => {
            sqlx::query(
                "UPDATE cn_moderation.jobs                  SET status = 'succeeded', completed_at = NOW(), updated_at = NOW(), last_error = NULL                  WHERE job_id = $1",
            )
            .bind(&job.job_id)
            .execute(pool)
            .await?;
        }
        Err(err) => {
            if job.attempts >= job.max_attempts {
                sqlx::query(
                    "UPDATE cn_moderation.jobs                      SET status = 'failed', completed_at = NOW(), updated_at = NOW(), last_error = $1                      WHERE job_id = $2",
                )
                .bind(&err)
                .bind(&job.job_id)
                .execute(pool)
                .await?;
            } else {
                sqlx::query(
                    "UPDATE cn_moderation.jobs                      SET status = 'pending', next_run_at = NOW() + ($1 * INTERVAL '1 second'),                          updated_at = NOW(), last_error = $2                      WHERE job_id = $3",
                )
                .bind(runtime.queue_retry_delay_seconds)
                .bind(&err)
                .bind(&job.job_id)
                .execute(pool)
                .await?;
            }
        }
    }
    Ok(())
}

async fn process_job(
    state: &AppState,
    runtime: &config::ModerationRuntimeConfig,
    job: &ModerationJob,
) -> Result<usize> {
    let Some(event) = load_event(&state.pool, &job.event_id).await? else {
        return Ok(0);
    };

    let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
    if event.is_deleted
        || !event.is_current
        || event.is_ephemeral
        || event.expires_at.map(|exp| exp <= now).unwrap_or(false)
    {
        return Ok(0);
    }

    let rules = load_active_rules(&state.pool).await?;

    let mut issued = 0usize;
    for rule in rules {
        if issued >= runtime.rules_max_labels_per_event {
            break;
        }
        if !rule_matches(&rule.conditions, &event.raw) {
            continue;
        }
        let exp = now.saturating_add(rule.action.exp_seconds);
        let input = moderation_core::LabelInput {
            target: format!("event:{}", event.raw.id),
            label: rule.action.label.clone(),
            confidence: rule.action.confidence,
            exp,
            policy_url: rule.action.policy_url.clone(),
            policy_ref: rule.action.policy_ref.clone(),
            topic_id: Some(job.topic_id.clone()),
        };
        let label_event = moderation_core::build_label_event(&state.node_keys, &input)?;
        let inserted = insert_label(
            &state.pool,
            &label_event,
            &input,
            Some(&event.raw.id),
            Some(&rule.rule_id),
            "rule",
        )
        .await?;
        if inserted {
            issued += 1;
        }
    }

    if runtime.llm.enabled {
        let provider = llm::build_provider(&runtime.llm);
        let request = llm::LlmRequest {
            event_id: event.raw.id.clone(),
            content: prepare_llm_input(&event.raw.content, &runtime.llm),
        };
        if !request.content.trim().is_empty()
            && !matches!(&provider, llm::LlmProviderKind::Disabled)
        {
            match acquire_llm_execution_gate(
                &state.pool,
                &runtime.llm,
                job,
                &event.raw.id,
                provider.source(),
                &request.content,
            )
            .await
            {
                Ok(LlmExecutionGate::Acquired(permit)) => {
                    let classification = provider.classify(&request).await;
                    if let Err(err) = release_llm_execution_gate(&state.pool, &permit).await {
                        tracing::warn!(
                            error = %err,
                            request_id = %permit.request_id,
                            job_id = %job.job_id,
                            event_id = %event.raw.id,
                            provider = provider.source(),
                            "failed to release llm execution gate"
                        );
                    }

                    match classification {
                        Ok(Some(prediction)) => {
                            match apply_llm_label(
                                state,
                                job,
                                &event.raw.id,
                                &provider,
                                &prediction,
                                now,
                            )
                            .await
                            {
                                Ok(true) => issued += 1,
                                Ok(false) => {}
                                Err(err) => {
                                    tracing::warn!(
                                        error = %err,
                                        job_id = %job.job_id,
                                        event_id = %event.raw.id,
                                        provider = provider.source(),
                                        "failed to apply llm label"
                                    );
                                }
                            }
                        }
                        Ok(None) => {}
                        Err(err) => {
                            tracing::warn!(
                                error = %err,
                                job_id = %job.job_id,
                                event_id = %event.raw.id,
                                provider = provider.source(),
                                "llm classification failed"
                            );
                        }
                    }
                }
                Ok(LlmExecutionGate::Skipped {
                    reason,
                    usage,
                    estimated_cost,
                }) => {
                    tracing::info!(
                        job_id = %job.job_id,
                        event_id = %event.raw.id,
                        provider = provider.source(),
                        skip_reason = reason.as_str(),
                        usage_day = usage.usage_day,
                        requests_today = usage.requests_today,
                        cost_today = usage.cost_today,
                        inflight_requests = usage.inflight_requests,
                        estimated_cost,
                        "llm classification skipped by runtime limits"
                    );
                    if let Err(err) = log_llm_skip_audit(
                        state,
                        job,
                        &event.raw.id,
                        reason,
                        &usage,
                        estimated_cost,
                        &runtime.llm,
                    )
                    .await
                    {
                        tracing::warn!(
                            error = %err,
                            job_id = %job.job_id,
                            event_id = %event.raw.id,
                            provider = provider.source(),
                            "failed to record llm skip audit log"
                        );
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        error = %err,
                        job_id = %job.job_id,
                        event_id = %event.raw.id,
                        provider = provider.source(),
                        "failed to evaluate llm runtime limits; skipping llm classification"
                    );
                }
            }
        }
    }

    Ok(issued)
}

async fn load_event(pool: &Pool<Postgres>, event_id: &str) -> Result<Option<ModerationEvent>> {
    let row = sqlx::query(
        "SELECT raw_json, is_deleted, is_current, is_ephemeral, expires_at          FROM cn_relay.events          WHERE event_id = $1",
    )
    .bind(event_id)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let raw_json: serde_json::Value = row.try_get("raw_json")?;
    let raw: nostr::RawEvent = serde_json::from_value(raw_json)?;
    Ok(Some(ModerationEvent {
        raw,
        is_deleted: row.try_get("is_deleted")?,
        is_current: row.try_get("is_current")?,
        is_ephemeral: row.try_get("is_ephemeral")?,
        expires_at: row.try_get("expires_at")?,
    }))
}

async fn load_active_rules(pool: &Pool<Postgres>) -> Result<Vec<moderation_core::ModerationRule>> {
    let rows = sqlx::query(
        "SELECT rule_id, name, description, is_enabled, priority, conditions_json, action_json          FROM cn_moderation.rules          WHERE is_enabled = TRUE          ORDER BY priority DESC, updated_at DESC",
    )
    .fetch_all(pool)
    .await?;

    let mut rules = Vec::new();
    for row in rows {
        let conditions_json: serde_json::Value = row.try_get("conditions_json")?;
        let action_json: serde_json::Value = row.try_get("action_json")?;
        let conditions: moderation_core::RuleCondition = serde_json::from_value(conditions_json)?;
        let action: moderation_core::RuleAction = serde_json::from_value(action_json)?;
        if conditions.validate().is_err() || action.validate().is_err() {
            tracing::warn!(
                rule_id = row.try_get::<String, _>("rule_id").unwrap_or_default(),
                "invalid moderation rule configuration"
            );
            continue;
        }
        rules.push(moderation_core::ModerationRule {
            rule_id: row.try_get("rule_id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            is_enabled: row.try_get("is_enabled")?,
            priority: row.try_get("priority")?,
            conditions,
            action,
        });
    }
    Ok(rules)
}

fn rule_matches(condition: &moderation_core::RuleCondition, raw: &nostr::RawEvent) -> bool {
    if let Some(kinds) = &condition.kinds {
        if !kinds.contains(&(raw.kind as i32)) {
            return false;
        }
    }
    if let Some(authors) = &condition.author_pubkeys {
        if !authors.iter().any(|author| author == &raw.pubkey) {
            return false;
        }
    }
    if let Some(keywords) = &condition.content_keywords {
        let content = raw.content.to_lowercase();
        if !keywords
            .iter()
            .any(|keyword| content.contains(&keyword.to_lowercase()))
        {
            return false;
        }
    }
    if let Some(pattern) = &condition.content_regex {
        let Ok(regex) = RegexBuilder::new(pattern).case_insensitive(true).build() else {
            tracing::warn!(pattern = pattern.as_str(), "invalid content regex");
            return false;
        };
        if !regex.is_match(&raw.content) {
            return false;
        }
    }
    if let Some(filters) = &condition.tag_filters {
        for (tag, values) in filters {
            let tag_values = raw.tag_values(tag);
            if tag_values.is_empty() {
                return false;
            }
            if !values.is_empty() && !tag_values.iter().any(|value| values.contains(value)) {
                return false;
            }
        }
    }
    true
}

async fn insert_label(
    pool: &Pool<Postgres>,
    label_event: &nostr::RawEvent,
    input: &moderation_core::LabelInput,
    source_event_id: Option<&str>,
    rule_id: Option<&str>,
    source: &str,
) -> Result<bool> {
    let label_json = serde_json::to_value(label_event)?;
    let result = sqlx::query(
        "INSERT INTO cn_moderation.labels          (label_id, source_event_id, target, topic_id, label, confidence, policy_url, policy_ref, exp, issuer_pubkey, rule_id, source, label_event_json, review_status)          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 'active')          ON CONFLICT (source_event_id, rule_id)          WHERE source_event_id IS NOT NULL AND rule_id IS NOT NULL AND review_status = 'active'          DO NOTHING",
    )
    .bind(&label_event.id)
    .bind(source_event_id)
    .bind(&input.target)
    .bind(&input.topic_id)
    .bind(&input.label)
    .bind(input.confidence)
    .bind(&input.policy_url)
    .bind(&input.policy_ref)
    .bind(input.exp)
    .bind(&label_event.pubkey)
    .bind(rule_id)
    .bind(source)
    .bind(label_json)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

async fn llm_label_exists(
    pool: &Pool<Postgres>,
    source_event_id: &str,
    source: &str,
    label: &str,
    now: i64,
) -> Result<bool> {
    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM cn_moderation.labels          WHERE source_event_id = $1            AND source = $2            AND label = $3            AND review_status = 'active'            AND exp > $4          LIMIT 1",
    )
    .bind(source_event_id)
    .bind(source)
    .bind(label)
    .bind(now)
    .fetch_optional(pool)
    .await?;
    Ok(exists.is_some())
}

async fn apply_llm_label(
    state: &AppState,
    job: &ModerationJob,
    event_id: &str,
    provider: &llm::LlmProviderKind,
    prediction: &llm::LlmLabel,
    now: i64,
) -> Result<bool> {
    if llm_label_exists(
        &state.pool,
        event_id,
        provider.source(),
        &prediction.label,
        now,
    )
    .await?
    {
        return Ok(false);
    }

    let input = moderation_core::LabelInput {
        target: format!("event:{event_id}"),
        label: prediction.label.clone(),
        confidence: prediction.confidence,
        exp: now.saturating_add(LLM_LABEL_EXP_SECONDS),
        policy_url: LLM_POLICY_URL.to_string(),
        policy_ref: LLM_POLICY_REF.to_string(),
        topic_id: Some(job.topic_id.clone()),
    };
    let label_event = moderation_core::build_label_event(&state.node_keys, &input)?;
    insert_label(
        &state.pool,
        &label_event,
        &input,
        Some(event_id),
        None,
        provider.source(),
    )
    .await
}

async fn acquire_llm_execution_gate(
    pool: &Pool<Postgres>,
    runtime: &config::LlmRuntimeConfig,
    job: &ModerationJob,
    event_id: &str,
    provider: &str,
    content: &str,
) -> Result<LlmExecutionGate> {
    let estimated_cost = estimate_llm_cost(content);
    let mut tx = pool.begin().await?;

    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(LLM_BUDGET_LOCK_KEY)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM cn_moderation.llm_inflight WHERE expires_at <= NOW()")
        .execute(&mut *tx)
        .await?;

    let usage_day: String =
        sqlx::query_scalar("SELECT (NOW() AT TIME ZONE 'UTC')::date::text AS usage_day")
            .fetch_one(&mut *tx)
            .await?;

    let usage_row = sqlx::query(
        "SELECT requests_count, estimated_cost          FROM cn_moderation.llm_daily_usage          WHERE usage_day = (NOW() AT TIME ZONE 'UTC')::date          FOR UPDATE",
    )
    .fetch_optional(&mut *tx)
    .await?;
    let (requests_today, cost_today) = if let Some(row) = usage_row {
        (
            row.try_get("requests_count")?,
            row.try_get("estimated_cost")?,
        )
    } else {
        (0_i64, 0.0_f64)
    };

    let inflight_requests =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM cn_moderation.llm_inflight")
            .fetch_one(&mut *tx)
            .await?;

    let usage = LlmUsageSnapshot {
        usage_day,
        requests_today,
        cost_today,
        inflight_requests,
    };

    if runtime.max_requests_per_day > 0 && usage.requests_today >= runtime.max_requests_per_day {
        tx.rollback().await?;
        return Ok(LlmExecutionGate::Skipped {
            reason: LlmSkipReason::RequestsPerDay,
            usage,
            estimated_cost,
        });
    }

    if runtime.max_cost_per_day > 0.0
        && estimated_cost > 0.0
        && usage.cost_today + estimated_cost > runtime.max_cost_per_day
    {
        tx.rollback().await?;
        return Ok(LlmExecutionGate::Skipped {
            reason: LlmSkipReason::CostPerDay,
            usage,
            estimated_cost,
        });
    }

    let max_concurrency = i64::try_from(runtime.max_concurrency.max(1)).unwrap_or(i64::MAX);
    if usage.inflight_requests >= max_concurrency {
        tx.rollback().await?;
        return Ok(LlmExecutionGate::Skipped {
            reason: LlmSkipReason::Concurrency,
            usage,
            estimated_cost,
        });
    }

    sqlx::query(
        "INSERT INTO cn_moderation.llm_daily_usage (usage_day, requests_count, estimated_cost, updated_at)          VALUES ((NOW() AT TIME ZONE 'UTC')::date, 1, $1, NOW())          ON CONFLICT (usage_day)          DO UPDATE SET requests_count = cn_moderation.llm_daily_usage.requests_count + 1,              estimated_cost = cn_moderation.llm_daily_usage.estimated_cost + EXCLUDED.estimated_cost,              updated_at = NOW()",
    )
    .bind(estimated_cost)
    .execute(&mut *tx)
    .await?;

    let request_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO cn_moderation.llm_inflight          (request_id, job_id, event_id, provider, estimated_cost, started_at, expires_at)          VALUES ($1, $2, $3, $4, $5, NOW(), NOW() + ($6 * INTERVAL '1 second'))",
    )
    .bind(&request_id)
    .bind(&job.job_id)
    .bind(event_id)
    .bind(provider)
    .bind(estimated_cost)
    .bind(LLM_INFLIGHT_TTL_SECONDS)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(LlmExecutionGate::Acquired(LlmExecutionPermit {
        request_id,
    }))
}

async fn release_llm_execution_gate(
    pool: &Pool<Postgres>,
    permit: &LlmExecutionPermit,
) -> Result<()> {
    sqlx::query("DELETE FROM cn_moderation.llm_inflight WHERE request_id = $1")
        .bind(&permit.request_id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn log_llm_skip_audit(
    state: &AppState,
    job: &ModerationJob,
    event_id: &str,
    reason: LlmSkipReason,
    usage: &LlmUsageSnapshot,
    estimated_cost: f64,
    runtime: &config::LlmRuntimeConfig,
) -> Result<()> {
    cn_core::admin::log_audit(
        &state.pool,
        "system",
        "moderation.llm.skip",
        &format!("event:{event_id}"),
        Some(json!({
            "job_id": job.job_id,
            "topic_id": job.topic_id,
            "event_id": event_id,
            "provider": runtime.provider,
            "skip_reason": reason.as_str(),
            "estimated_cost": estimated_cost,
            "usage": {
                "usage_day": usage.usage_day,
                "requests_today": usage.requests_today,
                "cost_today": usage.cost_today,
                "inflight_requests": usage.inflight_requests
            },
            "limits": {
                "max_requests_per_day": runtime.max_requests_per_day,
                "max_cost_per_day": runtime.max_cost_per_day,
                "max_concurrency": runtime.max_concurrency.max(1)
            }
        })),
        Some(&job.job_id),
    )
    .await
}

fn estimate_llm_cost(content: &str) -> f64 {
    let chars = content.chars().count();
    if chars == 0 {
        return 0.0;
    }
    let blocks = ((chars as f64) / 1000.0).ceil();
    blocks * LLM_COST_PER_1K_CHARS
}

fn prepare_llm_input(content: &str, config: &config::LlmRuntimeConfig) -> String {
    let mut result = if config.truncate_chars == 0 {
        String::new()
    } else {
        truncate_chars(content, config.truncate_chars)
    };
    if config.mask_pii {
        result = mask_pii(&result);
    }
    result
}

fn truncate_chars(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    value.chars().take(max).collect()
}

fn mask_pii(value: &str) -> String {
    let mut masked = value.to_string();
    let email = RegexBuilder::new(r"[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}")
        .case_insensitive(true)
        .build()
        .ok();
    if let Some(regex) = email {
        masked = regex.replace_all(&masked, "[redacted-email]").to_string();
    }
    let url = RegexBuilder::new(r"https?://[^\s]+")
        .case_insensitive(true)
        .build()
        .ok();
    if let Some(regex) = url {
        masked = regex.replace_all(&masked, "[redacted-url]").to_string();
    }
    masked
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::{header, StatusCode};
    use axum::response::IntoResponse;
    use axum::{
        extract::Json as AxumJson,
        routing::{get, post},
        Router,
    };
    use cn_core::moderation::RuleCondition;
    use serde_json::{json, Value};
    use sqlx::postgres::PgPoolOptions;
    use std::sync::atomic::{AtomicU16, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
    use tokio::net::TcpListener;
    use tokio::sync::OnceCell;
    use uuid::Uuid;

    static MIGRATIONS: OnceCell<()> = OnceCell::const_new();

    fn sample_event(content: &str, kind: u32, tags: Vec<Vec<String>>) -> nostr::RawEvent {
        nostr::RawEvent {
            id: "event-id".to_string(),
            pubkey: "pubkey".to_string(),
            created_at: 123,
            kind,
            tags,
            content: content.to_string(),
            sig: "sig".to_string(),
        }
    }

    #[test]
    fn rule_matches_content_keyword() {
        let condition = RuleCondition {
            kinds: None,
            content_regex: None,
            content_keywords: Some(vec!["spam".to_string()]),
            tag_filters: None,
            author_pubkeys: None,
        };
        let event = sample_event("This is spam content", 1, Vec::new());
        assert!(rule_matches(&condition, &event));
    }

    #[test]
    fn rule_matches_regex() {
        let condition = RuleCondition {
            kinds: None,
            content_regex: Some("spam\\d+".to_string()),
            content_keywords: None,
            tag_filters: None,
            author_pubkeys: None,
        };
        let event = sample_event("spam123 detected", 1, Vec::new());
        assert!(rule_matches(&condition, &event));
    }

    #[test]
    fn rule_matches_tag_filters() {
        let mut tags = Vec::new();
        tags.push(vec!["t".to_string(), "topic-1".to_string()]);
        let condition = RuleCondition {
            kinds: None,
            content_regex: None,
            content_keywords: None,
            tag_filters: Some(std::collections::HashMap::from([(
                "t".to_string(),
                vec!["topic-1".to_string()],
            )])),
            author_pubkeys: None,
        };
        let event = sample_event("content", 1, tags);
        assert!(rule_matches(&condition, &event));
    }

    fn database_url() -> String {
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://cn:cn_password@localhost:15432/cn".to_string())
    }

    async fn ensure_migrated(pool: &Pool<Postgres>) {
        MIGRATIONS
            .get_or_init(|| async {
                cn_core::migrations::run(pool)
                    .await
                    .expect("run migrations");
            })
            .await;
    }

    async fn spawn_dependency_health_server(
        status_code: Arc<AtomicU16>,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let app = Router::new().route(
            "/healthz",
            get({
                let status_code = Arc::clone(&status_code);
                move || {
                    let status_code = Arc::clone(&status_code);
                    async move {
                        let status = StatusCode::from_u16(status_code.load(Ordering::Relaxed))
                            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                        (status, AxumJson(json!({ "status": "mock" })))
                    }
                }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind dependency health mock");
        let addr = listener.local_addr().expect("dependency health mock addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("serve dependency health mock");
        });

        (format!("http://{addr}/healthz"), handle)
    }

    async fn response_json(response: axum::http::Response<axum::body::Body>) -> Value {
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read response body");
        serde_json::from_slice(&bytes).expect("parse json response")
    }

    async fn response_text(response: axum::http::Response<axum::body::Body>) -> String {
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read response body");
        String::from_utf8(bytes.to_vec()).expect("metrics response is utf-8")
    }

    fn assert_metric_line(body: &str, metric_name: &str, labels: &[(&str, &str)]) {
        let found = body.lines().any(|line| {
            if !line.starts_with(metric_name) {
                return false;
            }
            labels.iter().all(|(key, value)| {
                let token = format!("{key}=\"{value}\"");
                line.contains(&token)
            })
        });

        assert!(
            found,
            "metrics body did not contain {metric_name} with labels {labels:?}: {body}"
        );
    }

    async fn insert_event(pool: &Pool<Postgres>, event: &nostr::RawEvent, topic_id: &str) {
        sqlx::query(
            "INSERT INTO cn_relay.events              (event_id, pubkey, kind, created_at, tags, content, sig, raw_json, ingested_at, is_deleted, is_ephemeral, is_current, replaceable_key, addressable_key, expires_at)              VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), FALSE, FALSE, TRUE, NULL, NULL, NULL)              ON CONFLICT (event_id) DO NOTHING",
        )
        .bind(&event.id)
        .bind(&event.pubkey)
        .bind(event.kind as i32)
        .bind(event.created_at)
        .bind(serde_json::to_value(&event.tags).expect("serialize tags"))
        .bind(&event.content)
        .bind(&event.sig)
        .bind(serde_json::to_value(event).expect("serialize event"))
        .execute(pool)
        .await
        .expect("insert relay event");

        sqlx::query(
            "INSERT INTO cn_relay.event_topics (event_id, topic_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(&event.id)
        .bind(topic_id)
        .execute(pool)
        .await
        .expect("insert event topic");
    }

    async fn delete_event_artifacts(pool: &Pool<Postgres>, event_id: &str) {
        sqlx::query("DELETE FROM cn_moderation.llm_inflight WHERE event_id = $1")
            .bind(event_id)
            .execute(pool)
            .await
            .expect("delete llm inflight");
        sqlx::query("DELETE FROM cn_moderation.labels WHERE source_event_id = $1")
            .bind(event_id)
            .execute(pool)
            .await
            .expect("delete labels");
        sqlx::query("DELETE FROM cn_relay.event_topics WHERE event_id = $1")
            .bind(event_id)
            .execute(pool)
            .await
            .expect("delete event topics");
        sqlx::query("DELETE FROM cn_relay.events WHERE event_id = $1")
            .bind(event_id)
            .execute(pool)
            .await
            .expect("delete events");
    }

    async fn reset_llm_budget_state(pool: &Pool<Postgres>) {
        sqlx::query("DELETE FROM cn_moderation.llm_inflight")
            .execute(pool)
            .await
            .expect("delete llm inflight");
        sqlx::query("DELETE FROM cn_moderation.llm_daily_usage")
            .execute(pool)
            .await
            .expect("delete llm usage");
    }

    async fn latest_llm_skip_reason(pool: &Pool<Postgres>, event_id: &str) -> Option<String> {
        let target = format!("event:{event_id}");
        let diff = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT diff_json          FROM cn_admin.audit_logs          WHERE action = 'moderation.llm.skip'            AND target = $1          ORDER BY created_at DESC          LIMIT 1",
        )
        .bind(target)
        .fetch_optional(pool)
        .await
        .expect("query llm skip audit");
        diff.and_then(|value| {
            value
                .get("skip_reason")
                .and_then(|reason| reason.as_str())
                .map(ToString::to_string)
        })
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn lock_env() -> MutexGuard<'static, ()> {
        env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn set_env_var(key: &str, value: &str) {
        unsafe { std::env::set_var(key, value) };
    }

    fn remove_env_var(key: &str) {
        unsafe { std::env::remove_var(key) };
    }

    fn llm_runtime(
        max_requests_per_day: i64,
        max_cost_per_day: f64,
        max_concurrency: usize,
    ) -> config::ModerationRuntimeConfig {
        config::ModerationRuntimeConfig {
            enabled: true,
            consumer_batch_size: 1,
            consumer_poll_seconds: 1,
            queue_max_attempts: 3,
            queue_retry_delay_seconds: 1,
            rules_max_labels_per_event: 0,
            llm: config::LlmRuntimeConfig {
                enabled: true,
                provider: "local".to_string(),
                external_send_enabled: false,
                truncate_chars: 2000,
                mask_pii: true,
                max_requests_per_day,
                max_cost_per_day,
                max_concurrency,
            },
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn process_job_applies_llm_label_with_local_provider() {
        let _guard = lock_env();

        let pool = PgPoolOptions::new()
            .connect(&database_url())
            .await
            .expect("connect database");
        ensure_migrated(&pool).await;
        reset_llm_budget_state(&pool).await;

        let topic_id = format!("kukuri:moderation-llm-{}", Uuid::new_v4());
        let author = Keys::generate();
        let event = nostr::build_signed_event(
            &author,
            1,
            vec![vec!["t".to_string(), topic_id.clone()]],
            "Contact spammer@example.com at https://malicious.test".to_string(),
        )
        .expect("build test event");
        delete_event_artifacts(&pool, &event.id).await;
        insert_event(&pool, &event, &topic_id).await;

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind llm mock");
        let addr = listener.local_addr().expect("mock addr");
        let app = Router::new().route(
            "/classify",
            post(|| async { AxumJson(json!({ "label": "spam", "confidence": 0.93 })) }),
        );
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve llm mock");
        });

        let endpoint = format!("http://{addr}/classify");
        set_env_var("LLM_LOCAL_ENDPOINT", &endpoint);

        let state = AppState {
            pool: pool.clone(),
            config: service_config::static_handle(json!({})),
            node_keys: Keys::generate(),
            health_targets: Arc::new(HashMap::new()),
            health_client: reqwest::Client::new(),
        };
        let runtime = llm_runtime(0, 0.0, 1);
        let job = ModerationJob {
            job_id: Uuid::new_v4().to_string(),
            event_id: event.id.clone(),
            topic_id: topic_id.clone(),
            attempts: 1,
            max_attempts: 3,
        };

        let issued_first = process_job(&state, &runtime, &job)
            .await
            .expect("first process job");
        let issued_second = process_job(&state, &runtime, &job)
            .await
            .expect("second process job");

        assert_eq!(issued_first, 1);
        assert_eq!(issued_second, 0);

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cn_moderation.labels WHERE source_event_id = $1 AND source = 'llm:local'",
        )
        .bind(&event.id)
        .fetch_one(&pool)
        .await
        .expect("count llm labels");
        assert_eq!(count, 1);

        let label: String = sqlx::query_scalar(
            "SELECT label FROM cn_moderation.labels WHERE source_event_id = $1 AND source = 'llm:local' ORDER BY issued_at DESC LIMIT 1",
        )
        .bind(&event.id)
        .fetch_one(&pool)
        .await
        .expect("select llm label");
        assert_eq!(label, "spam");

        remove_env_var("LLM_LOCAL_ENDPOINT");
        server.abort();
        let _ = server.await;
        reset_llm_budget_state(&pool).await;
        delete_event_artifacts(&pool, &event.id).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn process_job_skips_llm_when_max_requests_per_day_reached() {
        let _guard = lock_env();

        let pool = PgPoolOptions::new()
            .connect(&database_url())
            .await
            .expect("connect database");
        ensure_migrated(&pool).await;
        reset_llm_budget_state(&pool).await;

        let topic_id = format!("kukuri:moderation-llm-{}", Uuid::new_v4());
        let author = Keys::generate();
        let event_first = nostr::build_signed_event(
            &author,
            1,
            vec![vec!["t".to_string(), topic_id.clone()]],
            "first llm target".to_string(),
        )
        .expect("build first event");
        let event_second = nostr::build_signed_event(
            &author,
            1,
            vec![vec!["t".to_string(), topic_id.clone()]],
            "second llm target".to_string(),
        )
        .expect("build second event");
        delete_event_artifacts(&pool, &event_first.id).await;
        delete_event_artifacts(&pool, &event_second.id).await;
        insert_event(&pool, &event_first, &topic_id).await;
        insert_event(&pool, &event_second, &topic_id).await;

        let call_count = Arc::new(AtomicUsize::new(0));
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind llm mock");
        let addr = listener.local_addr().expect("mock addr");
        let app = Router::new().route(
            "/classify",
            post({
                let call_count = Arc::clone(&call_count);
                move || {
                    let call_count = Arc::clone(&call_count);
                    async move {
                        call_count.fetch_add(1, Ordering::SeqCst);
                        AxumJson(json!({ "label": "spam", "confidence": 0.93 }))
                    }
                }
            }),
        );
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve llm mock");
        });

        let endpoint = format!("http://{addr}/classify");
        set_env_var("LLM_LOCAL_ENDPOINT", &endpoint);

        let state = AppState {
            pool: pool.clone(),
            config: service_config::static_handle(json!({})),
            node_keys: Keys::generate(),
            health_targets: Arc::new(HashMap::new()),
            health_client: reqwest::Client::new(),
        };
        let runtime = llm_runtime(1, 0.0, 4);
        let job_first = ModerationJob {
            job_id: Uuid::new_v4().to_string(),
            event_id: event_first.id.clone(),
            topic_id: topic_id.clone(),
            attempts: 1,
            max_attempts: 3,
        };
        let job_second = ModerationJob {
            job_id: Uuid::new_v4().to_string(),
            event_id: event_second.id.clone(),
            topic_id: topic_id.clone(),
            attempts: 1,
            max_attempts: 3,
        };

        let issued_first = process_job(&state, &runtime, &job_first)
            .await
            .expect("first process job");
        let issued_second = process_job(&state, &runtime, &job_second)
            .await
            .expect("second process job");

        assert_eq!(issued_first, 1);
        assert_eq!(issued_second, 0);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
        let skip_reason = latest_llm_skip_reason(&pool, &event_second.id).await;
        assert_eq!(skip_reason.as_deref(), Some("max_requests_per_day"));

        remove_env_var("LLM_LOCAL_ENDPOINT");
        server.abort();
        let _ = server.await;
        reset_llm_budget_state(&pool).await;
        delete_event_artifacts(&pool, &event_first.id).await;
        delete_event_artifacts(&pool, &event_second.id).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn process_job_skips_llm_when_max_cost_per_day_reached() {
        let _guard = lock_env();

        let pool = PgPoolOptions::new()
            .connect(&database_url())
            .await
            .expect("connect database");
        ensure_migrated(&pool).await;
        reset_llm_budget_state(&pool).await;

        let topic_id = format!("kukuri:moderation-llm-{}", Uuid::new_v4());
        let author = Keys::generate();
        let event = nostr::build_signed_event(
            &author,
            1,
            vec![vec!["t".to_string(), topic_id.clone()]],
            "cost limit target".to_string(),
        )
        .expect("build test event");
        delete_event_artifacts(&pool, &event.id).await;
        insert_event(&pool, &event, &topic_id).await;

        let call_count = Arc::new(AtomicUsize::new(0));
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind llm mock");
        let addr = listener.local_addr().expect("mock addr");
        let app = Router::new().route(
            "/classify",
            post({
                let call_count = Arc::clone(&call_count);
                move || {
                    let call_count = Arc::clone(&call_count);
                    async move {
                        call_count.fetch_add(1, Ordering::SeqCst);
                        AxumJson(json!({ "label": "spam", "confidence": 0.93 }))
                    }
                }
            }),
        );
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve llm mock");
        });

        let endpoint = format!("http://{addr}/classify");
        set_env_var("LLM_LOCAL_ENDPOINT", &endpoint);

        let state = AppState {
            pool: pool.clone(),
            config: service_config::static_handle(json!({})),
            node_keys: Keys::generate(),
            health_targets: Arc::new(HashMap::new()),
            health_client: reqwest::Client::new(),
        };
        let runtime = llm_runtime(0, 0.00001, 4);
        let job = ModerationJob {
            job_id: Uuid::new_v4().to_string(),
            event_id: event.id.clone(),
            topic_id: topic_id.clone(),
            attempts: 1,
            max_attempts: 3,
        };

        let issued = process_job(&state, &runtime, &job)
            .await
            .expect("process job");
        assert_eq!(issued, 0);
        assert_eq!(call_count.load(Ordering::SeqCst), 0);
        let skip_reason = latest_llm_skip_reason(&pool, &event.id).await;
        assert_eq!(skip_reason.as_deref(), Some("max_cost_per_day"));

        let label_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cn_moderation.labels WHERE source_event_id = $1 AND source = 'llm:local'",
        )
        .bind(&event.id)
        .fetch_one(&pool)
        .await
        .expect("count labels");
        assert_eq!(label_count, 0);

        remove_env_var("LLM_LOCAL_ENDPOINT");
        server.abort();
        let _ = server.await;
        reset_llm_budget_state(&pool).await;
        delete_event_artifacts(&pool, &event.id).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn process_job_skips_llm_when_max_concurrency_reached() {
        let _guard = lock_env();

        let pool = PgPoolOptions::new()
            .connect(&database_url())
            .await
            .expect("connect database");
        ensure_migrated(&pool).await;
        reset_llm_budget_state(&pool).await;

        let topic_id = format!("kukuri:moderation-llm-{}", Uuid::new_v4());
        let author = Keys::generate();
        let event = nostr::build_signed_event(
            &author,
            1,
            vec![vec!["t".to_string(), topic_id.clone()]],
            "concurrency limit target".to_string(),
        )
        .expect("build test event");
        delete_event_artifacts(&pool, &event.id).await;
        insert_event(&pool, &event, &topic_id).await;

        sqlx::query(
            "INSERT INTO cn_moderation.llm_inflight          (request_id, job_id, event_id, provider, estimated_cost, started_at, expires_at)          VALUES ($1, $2, $3, $4, $5, NOW(), NOW() + INTERVAL '5 minutes')",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(Uuid::new_v4().to_string())
        .bind("busy-event")
        .bind("llm:local")
        .bind(0.0_f64)
        .execute(&pool)
        .await
        .expect("insert inflight");

        let call_count = Arc::new(AtomicUsize::new(0));
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind llm mock");
        let addr = listener.local_addr().expect("mock addr");
        let app = Router::new().route(
            "/classify",
            post({
                let call_count = Arc::clone(&call_count);
                move || {
                    let call_count = Arc::clone(&call_count);
                    async move {
                        call_count.fetch_add(1, Ordering::SeqCst);
                        AxumJson(json!({ "label": "spam", "confidence": 0.93 }))
                    }
                }
            }),
        );
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve llm mock");
        });

        let endpoint = format!("http://{addr}/classify");
        set_env_var("LLM_LOCAL_ENDPOINT", &endpoint);

        let state = AppState {
            pool: pool.clone(),
            config: service_config::static_handle(json!({})),
            node_keys: Keys::generate(),
            health_targets: Arc::new(HashMap::new()),
            health_client: reqwest::Client::new(),
        };
        let runtime = llm_runtime(0, 0.0, 1);
        let job = ModerationJob {
            job_id: Uuid::new_v4().to_string(),
            event_id: event.id.clone(),
            topic_id: topic_id.clone(),
            attempts: 1,
            max_attempts: 3,
        };

        let issued = process_job(&state, &runtime, &job)
            .await
            .expect("process job");
        assert_eq!(issued, 0);
        assert_eq!(call_count.load(Ordering::SeqCst), 0);
        let skip_reason = latest_llm_skip_reason(&pool, &event.id).await;
        assert_eq!(skip_reason.as_deref(), Some("max_concurrency"));

        remove_env_var("LLM_LOCAL_ENDPOINT");
        server.abort();
        let _ = server.await;
        reset_llm_budget_state(&pool).await;
        delete_event_artifacts(&pool, &event.id).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn healthz_contract_status_transitions_when_dependency_fails() {
        let pool = PgPoolOptions::new()
            .connect(&database_url())
            .await
            .expect("connect database");
        ensure_migrated(&pool).await;

        let dependency_status = Arc::new(AtomicU16::new(StatusCode::OK.as_u16()));
        let (health_url, server_handle) =
            spawn_dependency_health_server(Arc::clone(&dependency_status)).await;
        let mut health_targets = HashMap::new();
        health_targets.insert("relay".to_string(), health_url);
        let state = AppState {
            pool,
            config: service_config::static_handle(json!({})),
            node_keys: Keys::generate(),
            health_targets: Arc::new(health_targets),
            health_client: reqwest::Client::new(),
        };

        let ok_response = healthz(State(state.clone())).await.into_response();
        assert_eq!(ok_response.status(), StatusCode::OK);
        let ok_payload = response_json(ok_response).await;
        assert_eq!(ok_payload.get("status"), Some(&json!("ok")));

        dependency_status.store(StatusCode::SERVICE_UNAVAILABLE.as_u16(), Ordering::Relaxed);
        let failed_response = healthz(State(state)).await.into_response();
        assert_eq!(failed_response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let failed_payload = response_json(failed_response).await;
        assert_eq!(failed_payload.get("status"), Some(&json!("unavailable")));

        server_handle.abort();
        let _ = server_handle.await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn metrics_contract_prometheus_content_type_shape_compatible() {
        let pool = PgPoolOptions::new()
            .connect(&database_url())
            .await
            .expect("connect database");
        ensure_migrated(&pool).await;

        let state = AppState {
            pool,
            config: service_config::static_handle(json!({})),
            node_keys: Keys::generate(),
            health_targets: Arc::new(HashMap::new()),
            health_client: reqwest::Client::new(),
        };

        metrics::observe_outbox_consumer_batch_size(SERVICE_NAME, CONSUMER_NAME, 2);
        metrics::inc_outbox_consumer_batch_total(
            SERVICE_NAME,
            CONSUMER_NAME,
            metrics::OUTBOX_CONSUMER_RESULT_SUCCESS,
        );
        metrics::inc_outbox_consumer_batch_total(
            SERVICE_NAME,
            CONSUMER_NAME,
            metrics::OUTBOX_CONSUMER_RESULT_ERROR,
        );
        metrics::observe_outbox_consumer_processing_duration(
            SERVICE_NAME,
            CONSUMER_NAME,
            metrics::OUTBOX_CONSUMER_RESULT_SUCCESS,
            std::time::Duration::from_millis(10),
        );
        let route = "/metrics-contract";
        metrics::record_http_request(
            SERVICE_NAME,
            "GET",
            route,
            200,
            std::time::Duration::from_millis(5),
        );

        let response = metrics_endpoint(State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok());
        assert_eq!(content_type, Some("text/plain; version=0.0.4"));

        let body = response_text(response).await;
        assert!(
            body.contains("cn_up{service=\"cn-moderation\"} 1"),
            "metrics body did not contain cn_up for cn-moderation: {body}"
        );
        assert!(
            body.contains(
                "outbox_consumer_batches_total{consumer=\"moderation-v1\",result=\"success\",service=\"cn-moderation\"} "
            ),
            "metrics body did not contain outbox_consumer_batches_total success labels for cn-moderation: {body}"
        );
        assert!(
            body.contains(
                "outbox_consumer_batches_total{consumer=\"moderation-v1\",result=\"error\",service=\"cn-moderation\"} "
            ),
            "metrics body did not contain outbox_consumer_batches_total error labels for cn-moderation: {body}"
        );
        assert!(
            body.contains(
                "outbox_consumer_processing_duration_seconds_count{consumer=\"moderation-v1\",result=\"success\",service=\"cn-moderation\"} "
            ),
            "metrics body did not contain outbox_consumer_processing_duration_seconds labels for cn-moderation: {body}"
        );
        assert!(
            body.contains(
                "outbox_consumer_batch_size_count{consumer=\"moderation-v1\",service=\"cn-moderation\"} "
            ),
            "metrics body did not contain outbox_consumer_batch_size labels for cn-moderation: {body}"
        );
        assert_metric_line(
            &body,
            "http_requests_total",
            &[
                ("service", SERVICE_NAME),
                ("route", route),
                ("method", "GET"),
                ("status", "200"),
            ],
        );
        assert_metric_line(
            &body,
            "http_request_duration_seconds_bucket",
            &[
                ("service", SERVICE_NAME),
                ("route", route),
                ("method", "GET"),
                ("status", "200"),
            ],
        );
    }
}
