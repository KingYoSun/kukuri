use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use cn_core::{
    config as env_config, db, health, http, logging, meili, metrics, nostr, server, service_config,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{postgres::PgListener, Pool, Postgres, Row};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

mod config;
#[cfg(test)]
mod integration_tests;

const SERVICE_NAME: &str = "cn-index";
const CONSUMER_NAME: &str = "index-v1";
const OUTBOX_CHANNEL: &str = "cn_relay_outbox";
const REINDEX_CHANNEL: &str = "cn_index_reindex";

#[derive(Clone)]
struct AppState {
    pool: Pool<Postgres>,
    config: service_config::ServiceConfigHandle,
    meili: meili::MeiliClient,
    index_cache: Arc<RwLock<HashSet<String>>>,
    health_targets: Arc<HashMap<String, String>>,
    health_client: reqwest::Client,
}

#[derive(Serialize)]
struct HealthStatus {
    status: String,
}

#[derive(Clone)]
pub struct IndexConfig {
    pub addr: SocketAddr,
    pub database_url: String,
    pub meili_url: String,
    pub meili_master_key: Option<String>,
    pub config_poll_seconds: u64,
}

#[derive(Deserialize)]
struct OutboxRow {
    seq: i64,
    op: String,
    event_id: String,
    topic_id: String,
    effective_key: Option<String>,
}

#[derive(Serialize)]
struct IndexDocument {
    event_id: String,
    topic_id: String,
    kind: i32,
    author: String,
    created_at: i64,
    title: String,
    summary: String,
    content: String,
    tags: Vec<String>,
}

pub fn load_config() -> Result<IndexConfig> {
    let addr = env_config::socket_addr_from_env("INDEX_ADDR", "0.0.0.0:8084")?;
    let database_url = env_config::required_env("DATABASE_URL")?;
    let meili_url = env_config::required_env("MEILI_URL")?;
    let meili_master_key = std::env::var("MEILI_MASTER_KEY").ok();
    let config_poll_seconds = std::env::var("INDEX_CONFIG_POLL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(30);
    Ok(IndexConfig {
        addr,
        database_url,
        meili_url,
        meili_master_key,
        config_poll_seconds,
    })
}

pub async fn run(config: IndexConfig) -> Result<()> {
    logging::init(SERVICE_NAME);
    metrics::init(SERVICE_NAME);

    let pool = db::connect(&config.database_url).await?;
    let meili_client = meili::MeiliClient::new(config.meili_url, config.meili_master_key)?;
    let default_config = json!({
        "enabled": true,
        "consumer": { "batch_size": 200, "poll_interval_seconds": 5 },
        "reindex": { "poll_interval_seconds": 30 },
        "expiration": { "sweep_interval_seconds": 300 }
    });
    let config_handle = service_config::watch_service_config(
        pool.clone(),
        "index",
        default_config,
        Duration::from_secs(config.config_poll_seconds),
    )
    .await?;
    let health_targets = Arc::new(health::parse_health_targets(
        "INDEX_HEALTH_TARGETS",
        &[("relay", "RELAY_HEALTH_URL", "http://relay:8082/healthz")],
    ));
    let health_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let state = AppState {
        pool: pool.clone(),
        config: config_handle,
        meili: meili_client,
        index_cache: Arc::new(RwLock::new(HashSet::new())),
        health_targets,
        health_client,
    };

    spawn_outbox_consumer(state.clone());
    spawn_reindex_worker(state.clone());
    spawn_expiration_sweep(state.clone());

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
        state.meili.check_ready().await?;
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
            let runtime = config::IndexRuntimeConfig::from_json(&snapshot.config_json);
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
                    let mut failed = false;
                    for row in &batch {
                        if let Err(err) = handle_outbox_row(&state, row).await {
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

fn spawn_reindex_worker(state: AppState) {
    tokio::spawn(async move {
        let mut listener = match connect_listener(&state.pool, REINDEX_CHANNEL).await {
            Ok(listener) => listener,
            Err(err) => {
                tracing::warn!(error = %err, "failed to listen reindex channel");
                return;
            }
        };

        loop {
            let snapshot = state.config.get().await;
            let runtime = config::IndexRuntimeConfig::from_json(&snapshot.config_json);
            if !runtime.enabled {
                tokio::time::sleep(Duration::from_secs(runtime.reindex_poll_seconds.max(5))).await;
                continue;
            }

            match claim_reindex_job(&state.pool).await {
                Ok(Some(job)) => {
                    if let Err(err) = run_reindex_job(&state, job).await {
                        tracing::error!(error = %err, "reindex job failed");
                    }
                }
                Ok(None) => {
                    if wait_for_notify(&mut listener, runtime.reindex_poll_seconds)
                        .await
                        .is_err()
                    {
                        match connect_listener(&state.pool, REINDEX_CHANNEL).await {
                            Ok(new_listener) => listener = new_listener,
                            Err(err) => {
                                tracing::warn!(
                                    error = %err,
                                    "failed to reconnect reindex listener"
                                );
                                tokio::time::sleep(Duration::from_secs(2)).await;
                            }
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!(error = %err, "reindex job claim failed");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    });
}

fn spawn_expiration_sweep(state: AppState) {
    tokio::spawn(async move {
        loop {
            let snapshot = state.config.get().await;
            let runtime = config::IndexRuntimeConfig::from_json(&snapshot.config_json);
            if runtime.enabled {
                if let Err(err) = expire_events_once(&state).await {
                    tracing::warn!(error = %err, "expiration sweep failed");
                }
            }
            tokio::time::sleep(Duration::from_secs(
                runtime.expiration_sweep_seconds.max(60),
            ))
            .await;
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
        "SELECT seq, op, event_id, topic_id, effective_key          FROM cn_relay.events_outbox          WHERE seq > $1          ORDER BY seq ASC          LIMIT $2",
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
            effective_key: row.try_get("effective_key")?,
        });
    }
    Ok(batch)
}

async fn handle_outbox_row(state: &AppState, row: &OutboxRow) -> Result<()> {
    match row.op.as_str() {
        "upsert" => handle_upsert(state, row).await,
        "delete" => handle_delete(state, row).await,
        other => {
            tracing::warn!(op = other, "unknown outbox op");
            Ok(())
        }
    }
}

async fn handle_upsert(state: &AppState, row: &OutboxRow) -> Result<()> {
    let Some(event) = load_event(&state.pool, &row.event_id).await? else {
        return handle_delete(state, row).await;
    };

    let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
    if event.is_deleted
        || !event.is_current
        || event.is_ephemeral
        || event.expires_at.map(|exp| exp <= now).unwrap_or(false)
    {
        if let Some(expired_at) = event.expires_at.filter(|exp| *exp <= now) {
            record_expired_event(&state.pool, &row.event_id, &row.topic_id, expired_at).await?;
        }
        return handle_delete(state, row).await;
    }

    let uid = ensure_topic_index(state, &row.topic_id).await?;
    let doc = build_document(&event.raw, &row.topic_id);
    state.meili.upsert_documents(&uid, &[doc]).await?;

    if let Some(key) = row.effective_key.as_deref() {
        let stale_ids = find_stale_versions(&state.pool, &row.topic_id, key, &row.event_id).await?;
        if !stale_ids.is_empty() {
            state.meili.delete_documents(&uid, &stale_ids).await?;
        }
    }

    Ok(())
}

async fn handle_delete(state: &AppState, row: &OutboxRow) -> Result<()> {
    let uid = ensure_topic_index(state, &row.topic_id).await?;
    state.meili.delete_document(&uid, &row.event_id).await?;
    Ok(())
}

async fn ensure_topic_index(state: &AppState, topic_id: &str) -> Result<String> {
    let uid = meili::topic_index_uid(topic_id);
    {
        let cache = state.index_cache.read().await;
        if cache.contains(&uid) {
            return Ok(uid);
        }
    }
    let settings = default_index_settings();
    state
        .meili
        .ensure_index(&uid, "event_id", Some(settings))
        .await?;
    let mut cache = state.index_cache.write().await;
    cache.insert(uid.clone());
    Ok(uid)
}

fn default_index_settings() -> Value {
    json!({
        "searchableAttributes": ["title", "summary", "content", "author", "tags"],
        "filterableAttributes": ["author", "kind", "created_at", "tags"],
        "sortableAttributes": ["created_at"]
    })
}

struct IndexedEvent {
    raw: nostr::RawEvent,
    is_deleted: bool,
    is_current: bool,
    is_ephemeral: bool,
    expires_at: Option<i64>,
}

async fn load_event(pool: &Pool<Postgres>, event_id: &str) -> Result<Option<IndexedEvent>> {
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
    Ok(Some(IndexedEvent {
        raw,
        is_deleted: row.try_get("is_deleted")?,
        is_current: row.try_get("is_current")?,
        is_ephemeral: row.try_get("is_ephemeral")?,
        expires_at: row.try_get("expires_at")?,
    }))
}

fn build_document(raw: &nostr::RawEvent, topic_id: &str) -> IndexDocument {
    IndexDocument {
        event_id: raw.id.clone(),
        topic_id: topic_id.to_string(),
        kind: raw.kind as i32,
        author: raw.pubkey.clone(),
        created_at: raw.created_at,
        title: normalize_title(raw),
        summary: normalize_summary(&raw.content),
        content: raw.content.clone(),
        tags: normalize_tags(raw),
    }
}

fn normalize_title(raw: &nostr::RawEvent) -> String {
    let from_tag = raw
        .first_tag_value("title")
        .or_else(|| raw.first_tag_value("subject"))
        .unwrap_or_default();
    if !from_tag.trim().is_empty() {
        return truncate_chars(from_tag.trim(), 80);
    }
    let first_line = raw.content.lines().next().unwrap_or("").trim();
    truncate_chars(first_line, 80)
}

fn normalize_summary(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    truncate_chars(trimmed, 200)
}

fn truncate_chars(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    value.chars().take(max).collect()
}

fn normalize_tags(raw: &nostr::RawEvent) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut tags = Vec::new();
    for tag in raw.tag_values("t") {
        if seen.insert(tag.clone()) {
            tags.push(tag);
        }
    }
    tags
}

async fn find_stale_versions(
    pool: &Pool<Postgres>,
    topic_id: &str,
    effective_key: &str,
    current_event_id: &str,
) -> Result<Vec<String>> {
    let rows = sqlx::query(
        "SELECT e.event_id          FROM cn_relay.events e          JOIN cn_relay.event_topics t            ON e.event_id = t.event_id          WHERE t.topic_id = $1            AND (e.replaceable_key = $2 OR e.addressable_key = $2)            AND e.event_id <> $3            AND e.is_current = FALSE",
    )
    .bind(topic_id)
    .bind(effective_key)
    .bind(current_event_id)
    .fetch_all(pool)
    .await?;

    let mut ids = Vec::new();
    for row in rows {
        ids.push(row.try_get("event_id")?);
    }
    Ok(ids)
}

#[derive(Debug)]
struct ReindexJob {
    job_id: String,
    topic_id: Option<String>,
    cutoff_seq: i64,
}

async fn claim_reindex_job(pool: &Pool<Postgres>) -> Result<Option<ReindexJob>> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query(
        "SELECT job_id, topic_id FROM cn_index.reindex_jobs          WHERE status = 'pending'          ORDER BY requested_at ASC          LIMIT 1          FOR UPDATE SKIP LOCKED",
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(row) = row else {
        tx.commit().await?;
        return Ok(None);
    };
    let job_id: String = row.try_get("job_id")?;
    let topic_id: Option<String> = row.try_get("topic_id")?;
    let cutoff_seq =
        sqlx::query_scalar::<_, i64>("SELECT COALESCE(MAX(seq), 0) FROM cn_relay.events_outbox")
            .fetch_one(&mut *tx)
            .await?;
    sqlx::query(
        "UPDATE cn_index.reindex_jobs          SET status = 'running', started_at = NOW(), cutoff_seq = $1, updated_at = NOW()          WHERE job_id = $2",
    )
    .bind(cutoff_seq)
    .bind(&job_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Some(ReindexJob {
        job_id,
        topic_id,
        cutoff_seq,
    }))
}

async fn run_reindex_job(state: &AppState, job: ReindexJob) -> Result<()> {
    let job_id = job.job_id.clone();
    let result = run_reindex_job_impl(state, &job).await;
    if let Err(err) = &result {
        let _ = update_job_failed(&state.pool, &job_id, &err.to_string()).await;
    }
    result
}

async fn run_reindex_job_impl(state: &AppState, job: &ReindexJob) -> Result<()> {
    let topics = if let Some(topic_id) = job.topic_id.clone() {
        vec![topic_id]
    } else {
        let mut topics = sqlx::query_scalar::<_, String>(
            "SELECT topic_id FROM cn_admin.node_subscriptions WHERE enabled = TRUE",
        )
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default();

        if topics.is_empty() {
            topics = sqlx::query_scalar::<_, String>(
                "SELECT DISTINCT topic_id FROM cn_relay.event_topics",
            )
            .fetch_all(&state.pool)
            .await
            .unwrap_or_default();
        }
        topics
    };

    let total_events = count_reindex_events(&state.pool, &topics).await?;
    update_job_totals(&state.pool, &job.job_id, total_events, 0).await?;

    let mut processed = 0_i64;
    for topic_id in topics {
        let uid = ensure_topic_index(state, &topic_id).await?;
        state.meili.delete_all_documents(&uid).await?;
        processed =
            reindex_topic(state, &topic_id, &uid, &job.job_id, processed, total_events).await?;
    }

    update_job_complete(&state.pool, &job.job_id, processed).await?;
    commit_last_seq(&state.pool, job.cutoff_seq).await?;
    Ok(())
}

async fn count_reindex_events(pool: &Pool<Postgres>, topics: &[String]) -> Result<i64> {
    if topics.is_empty() {
        return Ok(0);
    }
    let rows = sqlx::query(
        "SELECT COUNT(*) AS count          FROM cn_relay.events e          JOIN cn_relay.event_topics t            ON e.event_id = t.event_id          WHERE t.topic_id = ANY($1)            AND e.is_deleted = FALSE            AND e.is_ephemeral = FALSE            AND e.is_current = TRUE            AND (e.expires_at IS NULL OR e.expires_at > $2)",
    )
    .bind(topics)
    .bind(cn_core::auth::unix_seconds().unwrap_or(0) as i64)
    .fetch_one(pool)
    .await?;
    let count: i64 = rows.try_get("count")?;
    Ok(count)
}

async fn reindex_topic(
    state: &AppState,
    topic_id: &str,
    uid: &str,
    job_id: &str,
    mut processed: i64,
    total_events: i64,
) -> Result<i64> {
    let mut last_created_at = 0_i64;
    let mut last_event_id = String::new();
    let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;

    loop {
        let rows = sqlx::query(
            "SELECT e.raw_json, e.created_at, e.event_id          FROM cn_relay.events e          JOIN cn_relay.event_topics t            ON e.event_id = t.event_id          WHERE t.topic_id = $1            AND e.is_deleted = FALSE            AND e.is_ephemeral = FALSE            AND e.is_current = TRUE            AND (e.expires_at IS NULL OR e.expires_at > $2)            AND (e.created_at > $3 OR (e.created_at = $3 AND e.event_id > $4))          ORDER BY e.created_at ASC, e.event_id ASC          LIMIT 200",
        )
        .bind(topic_id)
        .bind(now)
        .bind(last_created_at)
        .bind(&last_event_id)
        .fetch_all(&state.pool)
        .await?;

        if rows.is_empty() {
            break;
        }

        let mut docs = Vec::new();
        for row in rows {
            let raw_json: serde_json::Value = row.try_get("raw_json")?;
            let raw: nostr::RawEvent = serde_json::from_value(raw_json)?;
            let created_at: i64 = row.try_get("created_at")?;
            let event_id: String = row.try_get("event_id")?;
            last_created_at = created_at;
            last_event_id = event_id.clone();
            docs.push(build_document(&raw, topic_id));
        }
        state.meili.upsert_documents(uid, &docs).await?;
        processed += docs.len() as i64;
        update_job_totals(&state.pool, job_id, total_events, processed).await?;
    }
    Ok(processed)
}

async fn update_job_totals(
    pool: &Pool<Postgres>,
    job_id: &str,
    total: i64,
    processed: i64,
) -> Result<()> {
    sqlx::query(
        "UPDATE cn_index.reindex_jobs          SET total_events = $1, processed_events = $2, updated_at = NOW()          WHERE job_id = $3",
    )
    .bind(total)
    .bind(processed)
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn update_job_complete(pool: &Pool<Postgres>, job_id: &str, processed: i64) -> Result<()> {
    sqlx::query(
        "UPDATE cn_index.reindex_jobs          SET status = 'succeeded', processed_events = $1, completed_at = NOW(), updated_at = NOW()          WHERE job_id = $2",
    )
    .bind(processed)
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn update_job_failed(pool: &Pool<Postgres>, job_id: &str, error: &str) -> Result<()> {
    sqlx::query(
        "UPDATE cn_index.reindex_jobs          SET status = 'failed', error_message = $1, completed_at = NOW(), updated_at = NOW()          WHERE job_id = $2",
    )
    .bind(error)
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn expire_events_once(state: &AppState) -> Result<()> {
    let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
    loop {
        let rows = sqlx::query(
            "SELECT e.event_id, t.topic_id, e.expires_at          FROM cn_relay.events e          JOIN cn_relay.event_topics t            ON e.event_id = t.event_id          LEFT JOIN cn_index.expired_events x            ON x.event_id = e.event_id AND x.topic_id = t.topic_id          WHERE e.expires_at IS NOT NULL            AND e.expires_at <= $1            AND e.is_deleted = FALSE            AND e.is_ephemeral = FALSE            AND e.is_current = TRUE            AND x.event_id IS NULL          LIMIT 200",
        )
        .bind(now)
        .fetch_all(&state.pool)
        .await?;

        if rows.is_empty() {
            break;
        }

        for row in rows {
            let event_id: String = row.try_get("event_id")?;
            let topic_id: String = row.try_get("topic_id")?;
            let expires_at: i64 = row.try_get("expires_at")?;
            let uid = ensure_topic_index(state, &topic_id).await?;
            state.meili.delete_document(&uid, &event_id).await?;
            record_expired_event(&state.pool, &event_id, &topic_id, expires_at).await?;
        }
    }
    Ok(())
}

async fn record_expired_event(
    pool: &Pool<Postgres>,
    event_id: &str,
    topic_id: &str,
    expired_at: i64,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_index.expired_events (event_id, topic_id, expired_at)          VALUES ($1, $2, $3)          ON CONFLICT (event_id, topic_id) DO NOTHING",
    )
    .bind(event_id)
    .bind(topic_id)
    .bind(expired_at)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_title_prefers_subject() {
        let raw = nostr::RawEvent {
            id: "id".to_string(),
            pubkey: "pub".to_string(),
            created_at: 1,
            kind: 1,
            tags: vec![vec!["subject".to_string(), "Hello".to_string()]],
            content: "body".to_string(),
            sig: "sig".to_string(),
        };
        assert_eq!(normalize_title(&raw), "Hello");
    }

    #[test]
    fn normalize_summary_truncates() {
        let content = "a".repeat(250);
        let summary = normalize_summary(&content);
        assert_eq!(summary.len(), 200);
    }
}
