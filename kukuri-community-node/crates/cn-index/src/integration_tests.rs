use super::*;
use axum::body::to_bytes;
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres, Row};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use tokio::net::TcpListener;
use tokio::sync::OnceCell;

static MIGRATIONS: OnceCell<()> = OnceCell::const_new();
static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static UNIQUE_COUNTER: AtomicU64 = AtomicU64::new(1);

struct ReindexJobRow {
    status: String,
    total_events: Option<i64>,
    processed_events: i64,
    cutoff_seq: Option<i64>,
    error_message: Option<String>,
    started_at: Option<i64>,
    completed_at: Option<i64>,
}

struct BackfillJobRow {
    status: String,
    processed_rows: i64,
    high_watermark_seq: Option<i64>,
    error_message: Option<String>,
    started_at: Option<i64>,
    completed_at: Option<i64>,
}

fn lock_tests() -> MutexGuard<'static, ()> {
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn next_id(prefix: &str) -> String {
    let sequence = UNIQUE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{sequence}")
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
            ensure_suggest_graph(pool)
                .await
                .expect("ensure suggest graph");
            bootstrap_suggest_graph_if_needed(pool)
                .await
                .expect("bootstrap suggest graph");
        })
        .await;
}

fn build_state(pool: Pool<Postgres>) -> AppState {
    AppState {
        pool,
        config: service_config::static_handle(json!({})),
        health_targets: Arc::new(HashMap::new()),
        health_client: reqwest::Client::new(),
    }
}

fn raw_event(event_id: &str, topic_id: &str, created_at: i64, content: &str) -> nostr::RawEvent {
    nostr::RawEvent {
        id: event_id.to_string(),
        pubkey: format!("pubkey-{event_id}"),
        created_at,
        kind: 1,
        tags: vec![
            vec!["t".to_string(), topic_id.to_string()],
            vec!["title".to_string(), format!("title-{event_id}")],
        ],
        content: content.to_string(),
        sig: "sig".to_string(),
    }
}

async fn insert_event(
    pool: &Pool<Postgres>,
    topic_id: &str,
    event: &nostr::RawEvent,
    expires_at: Option<i64>,
) {
    sqlx::query(
        "INSERT INTO cn_relay.events \
         (event_id, pubkey, kind, created_at, tags, content, sig, raw_json, ingested_at, is_deleted, is_ephemeral, is_current, replaceable_key, addressable_key, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), FALSE, FALSE, TRUE, NULL, NULL, $9) \
         ON CONFLICT (event_id) DO NOTHING",
    )
    .bind(&event.id)
    .bind(&event.pubkey)
    .bind(event.kind as i32)
    .bind(event.created_at)
    .bind(serde_json::to_value(&event.tags).expect("serialize tags"))
    .bind(&event.content)
    .bind(&event.sig)
    .bind(serde_json::to_value(event).expect("serialize raw event"))
    .bind(expires_at)
    .execute(pool)
    .await
    .expect("insert event");

    sqlx::query(
        "INSERT INTO cn_relay.event_topics (event_id, topic_id) \
         VALUES ($1, $2) \
         ON CONFLICT DO NOTHING",
    )
    .bind(&event.id)
    .bind(topic_id)
    .execute(pool)
    .await
    .expect("insert event topic");
}

async fn insert_outbox_row(
    pool: &Pool<Postgres>,
    op: &str,
    topic_id: &str,
    event: &nostr::RawEvent,
    effective_key: Option<&str>,
) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO cn_relay.events_outbox \
         (op, event_id, topic_id, kind, created_at, ingested_at, effective_key, reason) \
         VALUES ($1, $2, $3, $4, $5, NOW(), $6, 'integration-test') \
         RETURNING seq",
    )
    .bind(op)
    .bind(&event.id)
    .bind(topic_id)
    .bind(event.kind as i32)
    .bind(event.created_at)
    .bind(effective_key)
    .fetch_one(pool)
    .await
    .expect("insert outbox row")
}

async fn insert_node_subscription(pool: &Pool<Postgres>, topic_id: &str) {
    sqlx::query(
        "INSERT INTO cn_admin.node_subscriptions (topic_id, enabled, ref_count) \
         VALUES ($1, TRUE, 1) \
         ON CONFLICT (topic_id) DO UPDATE \
         SET enabled = TRUE, ref_count = EXCLUDED.ref_count, updated_at = NOW()",
    )
    .bind(topic_id)
    .execute(pool)
    .await
    .expect("insert node subscription");
}

async fn insert_reindex_job(pool: &Pool<Postgres>, job_id: &str, topic_id: &str) {
    sqlx::query(
        "INSERT INTO cn_index.reindex_jobs \
         (job_id, topic_id, status, requested_by, requested_at) \
         VALUES ($1, $2, 'pending', 'integration-test', NOW() - INTERVAL '100 years')",
    )
    .bind(job_id)
    .bind(topic_id)
    .execute(pool)
    .await
    .expect("insert reindex job");
}

async fn fetch_reindex_job(pool: &Pool<Postgres>, job_id: &str) -> ReindexJobRow {
    let row = sqlx::query(
        "SELECT status, total_events, processed_events, cutoff_seq, error_message, \
         EXTRACT(EPOCH FROM started_at)::BIGINT AS started_at, \
         EXTRACT(EPOCH FROM completed_at)::BIGINT AS completed_at \
         FROM cn_index.reindex_jobs \
         WHERE job_id = $1",
    )
    .bind(job_id)
    .fetch_one(pool)
    .await
    .expect("fetch reindex job");

    ReindexJobRow {
        status: row.try_get("status").expect("status"),
        total_events: row.try_get("total_events").expect("total_events"),
        processed_events: row.try_get("processed_events").expect("processed_events"),
        cutoff_seq: row.try_get("cutoff_seq").expect("cutoff_seq"),
        error_message: row.try_get("error_message").expect("error_message"),
        started_at: row.try_get("started_at").expect("started_at"),
        completed_at: row.try_get("completed_at").expect("completed_at"),
    }
}

async fn insert_backfill_job(
    pool: &Pool<Postgres>,
    job_id: &str,
    target: &str,
    high_watermark_seq: Option<i64>,
) {
    sqlx::query(
        "INSERT INTO cn_search.backfill_jobs \
         (job_id, target, status, high_watermark_seq, processed_rows, updated_at) \
         VALUES ($1, $2, 'pending', $3, 0, NOW())",
    )
    .bind(job_id)
    .bind(target)
    .bind(high_watermark_seq)
    .execute(pool)
    .await
    .expect("insert backfill job");
}

async fn fetch_backfill_job(pool: &Pool<Postgres>, job_id: &str) -> BackfillJobRow {
    let row = sqlx::query(
        "SELECT status, processed_rows, high_watermark_seq, error_message, \
         EXTRACT(EPOCH FROM started_at)::BIGINT AS started_at, \
         EXTRACT(EPOCH FROM completed_at)::BIGINT AS completed_at \
         FROM cn_search.backfill_jobs \
         WHERE job_id = $1",
    )
    .bind(job_id)
    .fetch_one(pool)
    .await
    .expect("fetch backfill job");

    BackfillJobRow {
        status: row.try_get("status").expect("status"),
        processed_rows: row.try_get("processed_rows").expect("processed_rows"),
        high_watermark_seq: row
            .try_get("high_watermark_seq")
            .expect("high_watermark_seq"),
        error_message: row.try_get("error_message").expect("error_message"),
        started_at: row.try_get("started_at").expect("started_at"),
        completed_at: row.try_get("completed_at").expect("completed_at"),
    }
}

async fn upsert_backfill_checkpoint_for_test(
    pool: &Pool<Postgres>,
    job_id: &str,
    cursor: &BackfillCursor,
) {
    let serialized = serde_json::to_string(cursor).expect("serialize cursor");
    sqlx::query(
        "INSERT INTO cn_search.backfill_checkpoints \
         (job_id, shard_key, last_cursor, updated_at) \
         VALUES ($1, $2, $3, NOW()) \
         ON CONFLICT (job_id, shard_key) DO UPDATE \
         SET last_cursor = EXCLUDED.last_cursor, updated_at = NOW()",
    )
    .bind(job_id)
    .bind(BACKFILL_DEFAULT_SHARD_KEY)
    .bind(serialized)
    .execute(pool)
    .await
    .expect("upsert backfill checkpoint");
}

async fn cleanup_records(
    pool: &Pool<Postgres>,
    topic_id: &str,
    event_ids: &[String],
    job_ids: &[String],
) {
    if !job_ids.is_empty() {
        let job_refs: Vec<&str> = job_ids.iter().map(String::as_str).collect();
        sqlx::query("DELETE FROM cn_search.backfill_checkpoints WHERE job_id = ANY($1)")
            .bind(&job_refs)
            .execute(pool)
            .await
            .expect("cleanup backfill checkpoints");
        sqlx::query("DELETE FROM cn_search.backfill_jobs WHERE job_id = ANY($1)")
            .bind(&job_refs)
            .execute(pool)
            .await
            .expect("cleanup backfill jobs");
        sqlx::query("DELETE FROM cn_index.reindex_jobs WHERE job_id = ANY($1)")
            .bind(&job_refs)
            .execute(pool)
            .await
            .expect("cleanup reindex jobs");
    }

    if !event_ids.is_empty() {
        let event_refs: Vec<&str> = event_ids.iter().map(String::as_str).collect();
        sqlx::query("DELETE FROM cn_index.expired_events WHERE event_id = ANY($1)")
            .bind(&event_refs)
            .execute(pool)
            .await
            .expect("cleanup expired events");
        sqlx::query("DELETE FROM cn_search.post_search_documents WHERE post_id = ANY($1)")
            .bind(&event_refs)
            .execute(pool)
            .await
            .expect("cleanup post search documents");
        sqlx::query("DELETE FROM cn_relay.events_outbox WHERE event_id = ANY($1)")
            .bind(&event_refs)
            .execute(pool)
            .await
            .expect("cleanup outbox");
        sqlx::query("DELETE FROM cn_relay.event_topics WHERE event_id = ANY($1)")
            .bind(&event_refs)
            .execute(pool)
            .await
            .expect("cleanup event topics");
        sqlx::query("DELETE FROM cn_relay.events WHERE event_id = ANY($1)")
            .bind(&event_refs)
            .execute(pool)
            .await
            .expect("cleanup events");
    }

    sqlx::query("DELETE FROM cn_search.community_search_terms WHERE community_id = $1")
        .bind(topic_id)
        .execute(pool)
        .await
        .expect("cleanup community search terms");
    sqlx::query("DELETE FROM cn_admin.node_subscriptions WHERE topic_id = $1")
        .bind(topic_id)
        .execute(pool)
        .await
        .expect("cleanup node subscription");
    sqlx::query("DELETE FROM cn_user.topic_subscriptions WHERE topic_id = $1")
        .bind(topic_id)
        .execute(pool)
        .await
        .expect("cleanup topic subscriptions");
    sqlx::query("DELETE FROM cn_user.topic_memberships WHERE topic_id = $1")
        .bind(topic_id)
        .execute(pool)
        .await
        .expect("cleanup topic memberships");
    sqlx::query("DELETE FROM cn_search.user_community_affinity WHERE community_id = $1")
        .bind(topic_id)
        .execute(pool)
        .await
        .expect("cleanup user community affinity");
    sqlx::query("DELETE FROM cn_search.graph_sync_offsets WHERE consumer = $1")
        .bind(GRAPH_SYNC_CONSUMER_NAME)
        .execute(pool)
        .await
        .expect("cleanup graph sync offsets");
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
                    (status, Json(json!({ "status": "mock" })))
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

#[tokio::test(flavor = "current_thread")]
async fn outbox_upsert_delete_updates_post_search_documents_and_terms() {
    let _guard = lock_tests();

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:index-it:{}", next_id("topic"));
    let now = cn_core::auth::unix_seconds().expect("unix seconds") as i64;
    let event = raw_event(&next_id("event-upsert"), &topic_id, now, "upsert-content");
    insert_event(&pool, &topic_id, &event, None).await;
    let state = build_state(pool.clone());

    let upsert_seq = insert_outbox_row(&pool, "upsert", &topic_id, &event, None).await;
    let upsert_rows = fetch_outbox_batch(&pool, upsert_seq - 1, 10)
        .await
        .expect("fetch upsert rows");
    assert_eq!(upsert_rows.len(), 1);
    handle_outbox_row(&state, &upsert_rows[0])
        .await
        .expect("handle upsert row");

    let upserted: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) \
         FROM cn_search.post_search_documents \
         WHERE post_id = $1 AND topic_id = $2 AND is_deleted = FALSE",
    )
    .bind(&event.id)
    .bind(&topic_id)
    .fetch_one(&pool)
    .await
    .expect("count upserted post_search_documents rows");
    assert_eq!(upserted, 1);

    let terms = sqlx::query(
        "SELECT term_type, term_norm, is_primary \
         FROM cn_search.community_search_terms \
         WHERE community_id = $1",
    )
    .bind(&topic_id)
    .fetch_all(&pool)
    .await
    .expect("fetch community search terms");
    assert!(
        !terms.is_empty(),
        "expected generated community search terms"
    );

    let expected_terms = community_search_terms::build_terms_from_topic_id(&topic_id);
    for expected in expected_terms {
        let matched = terms.iter().any(|row| {
            row.try_get::<String, _>("term_type")
                .map(|value| value == expected.term_type)
                .unwrap_or(false)
                && row
                    .try_get::<String, _>("term_norm")
                    .map(|value| value == expected.term_norm)
                    .unwrap_or(false)
                && row
                    .try_get::<bool, _>("is_primary")
                    .map(|value| value == expected.is_primary)
                    .unwrap_or(false)
        });
        assert!(
            matched,
            "missing expected term {:?} for {topic_id}",
            expected.term_norm
        );
    }

    sqlx::query(
        "UPDATE cn_relay.events \
         SET is_deleted = TRUE, is_current = FALSE \
         WHERE event_id = $1",
    )
    .bind(&event.id)
    .execute(&pool)
    .await
    .expect("mark event deleted");
    let delete_seq = insert_outbox_row(&pool, "delete", &topic_id, &event, None).await;
    let delete_rows = fetch_outbox_batch(&pool, delete_seq - 1, 10)
        .await
        .expect("fetch delete rows");
    assert_eq!(delete_rows.len(), 1);
    handle_outbox_row(&state, &delete_rows[0])
        .await
        .expect("handle delete row");

    let is_deleted: bool = sqlx::query_scalar(
        "SELECT is_deleted \
         FROM cn_search.post_search_documents \
         WHERE post_id = $1 AND topic_id = $2",
    )
    .bind(&event.id)
    .bind(&topic_id)
    .fetch_one(&pool)
    .await
    .expect("fetch post_search_documents delete state");
    assert!(
        is_deleted,
        "expected post_search_documents row to be tombstoned"
    );

    cleanup_records(&pool, &topic_id, &[event.id.clone()], &[]).await;
}

#[tokio::test(flavor = "current_thread")]
async fn reindex_job_transitions_pending_running_succeeded_and_updates_post_search_documents() {
    let _guard = lock_tests();

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:index-it:{}", next_id("topic"));
    let now = cn_core::auth::unix_seconds().expect("unix seconds") as i64;
    let event_a = raw_event(&next_id("event-a"), &topic_id, now, "reindex-event-a");
    let event_b = raw_event(&next_id("event-b"), &topic_id, now + 1, "reindex-event-b");
    insert_event(&pool, &topic_id, &event_a, None).await;
    insert_event(&pool, &topic_id, &event_b, None).await;
    insert_node_subscription(&pool, &topic_id).await;

    let stale_event = raw_event("stale-doc", &topic_id, now - 10, "stale");
    let stale_doc = build_post_search_document(&stale_event, &topic_id);
    upsert_post_search_document(&pool, &stale_doc)
        .await
        .expect("seed stale post_search_documents row");

    let state = build_state(pool.clone());
    load_last_seq(&pool)
        .await
        .expect("initialize consumer offset");

    let job_id = next_id("reindex-success");
    insert_reindex_job(&pool, &job_id, &topic_id).await;
    let pending_job = fetch_reindex_job(&pool, &job_id).await;
    assert_eq!(pending_job.status, "pending");
    assert!(pending_job.started_at.is_none());
    assert!(pending_job.completed_at.is_none());

    let claimed = claim_reindex_job(&pool)
        .await
        .expect("claim reindex job")
        .expect("expected pending reindex job");
    assert_eq!(claimed.job_id, job_id);
    assert_eq!(claimed.topic_id.as_deref(), Some(topic_id.as_str()));

    let running_job = fetch_reindex_job(&pool, &job_id).await;
    assert_eq!(running_job.status, "running");
    assert_eq!(running_job.cutoff_seq, Some(claimed.cutoff_seq));
    assert!(running_job.started_at.is_some());
    assert!(running_job.completed_at.is_none());

    run_reindex_job(&state, claimed)
        .await
        .expect("run reindex job");

    let succeeded_job = fetch_reindex_job(&pool, &job_id).await;
    assert_eq!(succeeded_job.status, "succeeded");
    assert_eq!(succeeded_job.total_events, Some(2));
    assert_eq!(succeeded_job.processed_events, 2);
    assert!(succeeded_job.error_message.is_none());
    assert!(succeeded_job.started_at.is_some());
    assert!(succeeded_job.completed_at.is_some());

    let rows = sqlx::query(
        "SELECT post_id \
         FROM cn_search.post_search_documents \
         WHERE topic_id = $1 AND is_deleted = FALSE",
    )
    .bind(&topic_id)
    .fetch_all(&pool)
    .await
    .expect("fetch reindex target rows");
    let mut post_ids: Vec<String> = rows
        .into_iter()
        .map(|row| row.try_get::<String, _>("post_id").expect("post_id"))
        .collect();
    post_ids.sort();
    assert_eq!(post_ids, vec![event_a.id.clone(), event_b.id.clone()]);

    cleanup_records(
        &pool,
        &topic_id,
        &[event_a.id.clone(), event_b.id.clone()],
        &[job_id],
    )
    .await;
    sqlx::query("DELETE FROM cn_search.post_search_documents WHERE post_id = 'stale-doc'")
        .execute(&pool)
        .await
        .expect("cleanup stale document");
}

#[tokio::test(flavor = "current_thread")]
async fn backfill_job_resumes_from_checkpoint_and_completes_post_search_documents() {
    let _guard = lock_tests();

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("~~~~index-backfill-it:{}", next_id("topic"));
    let now = cn_core::auth::unix_seconds().expect("unix seconds") as i64;
    let event_a = raw_event(
        &next_id("event-backfill-a"),
        &topic_id,
        now,
        "backfill-event-a",
    );
    let event_b = raw_event(
        &next_id("event-backfill-b"),
        &topic_id,
        now + 1,
        "backfill-event-b",
    );
    let event_c = raw_event(
        &next_id("event-backfill-c"),
        &topic_id,
        now + 2,
        "backfill-event-c",
    );
    insert_event(&pool, &topic_id, &event_a, None).await;
    insert_event(&pool, &topic_id, &event_b, None).await;
    insert_event(&pool, &topic_id, &event_c, None).await;

    insert_outbox_row(&pool, "upsert", &topic_id, &event_a, None).await;
    insert_outbox_row(&pool, "upsert", &topic_id, &event_b, None).await;
    insert_outbox_row(&pool, "upsert", &topic_id, &event_c, None).await;
    let high_watermark_seq: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(seq), 0) FROM cn_relay.events_outbox")
            .fetch_one(&pool)
            .await
            .expect("fetch high watermark");

    let backfill_job_id = next_id("backfill-resume");
    insert_backfill_job(
        &pool,
        &backfill_job_id,
        BACKFILL_TARGET_POST_SEARCH_DOCUMENTS,
        Some(high_watermark_seq),
    )
    .await;

    let doc_a = build_post_search_document(&event_a, &topic_id);
    upsert_post_search_document(&pool, &doc_a)
        .await
        .expect("seed backfill checkpoint document");
    sqlx::query(
        "UPDATE cn_search.backfill_jobs \
         SET processed_rows = 1, updated_at = NOW() \
         WHERE job_id = $1",
    )
    .bind(&backfill_job_id)
    .execute(&pool)
    .await
    .expect("seed backfill processed rows");
    upsert_backfill_checkpoint_for_test(
        &pool,
        &backfill_job_id,
        &BackfillCursor {
            topic_id: topic_id.clone(),
            created_at: event_a.created_at,
            event_id: event_a.id.clone(),
        },
    )
    .await;

    let claimed = claim_backfill_job(&pool, BACKFILL_TARGET_POST_SEARCH_DOCUMENTS)
        .await
        .expect("claim backfill job")
        .expect("expected pending backfill job");
    assert_eq!(claimed.job_id, backfill_job_id);
    assert_eq!(claimed.target, BACKFILL_TARGET_POST_SEARCH_DOCUMENTS);
    assert_eq!(claimed.high_watermark_seq, Some(high_watermark_seq));
    assert_eq!(claimed.processed_rows, 1);

    let state = build_state(pool.clone());
    run_backfill_job(&state, claimed)
        .await
        .expect("run backfill job");

    let succeeded = fetch_backfill_job(&pool, &backfill_job_id).await;
    assert_eq!(succeeded.status, "succeeded");
    assert!(
        succeeded.processed_rows >= 3,
        "expected at least test topic rows to be processed"
    );
    assert_eq!(succeeded.high_watermark_seq, Some(high_watermark_seq));
    assert!(succeeded.error_message.is_none());
    assert!(succeeded.started_at.is_some());
    assert!(succeeded.completed_at.is_some());

    let rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) \
         FROM cn_search.post_search_documents \
         WHERE topic_id = $1 \
           AND post_id = ANY($2) \
           AND is_deleted = FALSE",
    )
    .bind(&topic_id)
    .bind(vec![
        event_a.id.clone(),
        event_b.id.clone(),
        event_c.id.clone(),
    ])
    .fetch_one(&pool)
    .await
    .expect("count backfilled documents");
    assert_eq!(rows, 3);

    let checkpoint_raw: String = sqlx::query_scalar(
        "SELECT last_cursor \
         FROM cn_search.backfill_checkpoints \
         WHERE job_id = $1 \
           AND shard_key = $2",
    )
    .bind(&backfill_job_id)
    .bind(BACKFILL_DEFAULT_SHARD_KEY)
    .fetch_one(&pool)
    .await
    .expect("fetch checkpoint");
    let checkpoint: BackfillCursor =
        serde_json::from_str(&checkpoint_raw).expect("parse checkpoint cursor");
    let checkpoint_advanced: bool = sqlx::query_scalar(
        "SELECT ($1::TEXT, $2::BIGINT, $3::TEXT) > ($4::TEXT, $5::BIGINT, $6::TEXT)",
    )
    .bind(&checkpoint.topic_id)
    .bind(checkpoint.created_at)
    .bind(&checkpoint.event_id)
    .bind(&topic_id)
    .bind(event_a.created_at)
    .bind(&event_a.id)
    .fetch_one(&pool)
    .await
    .expect("compare checkpoint ordering");
    assert!(
        checkpoint_advanced,
        "checkpoint should advance beyond seeded cursor: checkpoint={checkpoint:?}"
    );

    cleanup_records(
        &pool,
        &topic_id,
        &[event_a.id.clone(), event_b.id.clone(), event_c.id.clone()],
        &[backfill_job_id],
    )
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn claim_backfill_job_reclaims_stale_running_job() {
    let _guard = lock_tests();

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:index-backfill-stale-it:{}", next_id("topic"));
    let job_id = next_id("backfill-stale-running");
    let stale_by_seconds = BACKFILL_RUNNING_LEASE_TIMEOUT_SECONDS + 30;
    sqlx::query(
        "INSERT INTO cn_search.backfill_jobs \
         (job_id, target, status, high_watermark_seq, processed_rows, started_at, completed_at, updated_at) \
         VALUES ( \
             $1, $2, 'running', 42, 7, \
             NOW() - ($3::BIGINT * INTERVAL '1 second'), \
             NULL, \
             NOW() - ($3::BIGINT * INTERVAL '1 second') \
         )",
    )
    .bind(&job_id)
    .bind(BACKFILL_TARGET_POST_SEARCH_DOCUMENTS)
    .bind(stale_by_seconds)
    .execute(&pool)
    .await
    .expect("insert stale running backfill job");

    let claimed = claim_backfill_job(&pool, BACKFILL_TARGET_POST_SEARCH_DOCUMENTS)
        .await
        .expect("claim backfill job")
        .expect("expected stale running backfill job");
    assert_eq!(claimed.job_id, job_id);
    assert_eq!(claimed.target, BACKFILL_TARGET_POST_SEARCH_DOCUMENTS);
    assert_eq!(claimed.high_watermark_seq, Some(42));
    assert_eq!(claimed.processed_rows, 7);

    let running = fetch_backfill_job(&pool, &job_id).await;
    assert_eq!(running.status, "running");
    assert_eq!(running.processed_rows, 7);
    assert!(running.error_message.is_none());
    assert!(running.started_at.is_some());
    assert!(running.completed_at.is_none());
    let running_started_at = running.started_at.expect("running started_at");
    let lease_started_at = claimed.lease_started_at.timestamp();
    assert!(
        (running_started_at - lease_started_at).abs() <= 1,
        "running started_at should track claimed lease start: running={running_started_at}, lease={lease_started_at}"
    );

    let second_claim = claim_backfill_job(&pool, BACKFILL_TARGET_POST_SEARCH_DOCUMENTS)
        .await
        .expect("claim backfill job second time");
    assert!(
        second_claim.is_none(),
        "freshly claimed running job should not be reclaimed immediately"
    );

    cleanup_records(&pool, &topic_id, &[], &[job_id]).await;
}

#[tokio::test(flavor = "current_thread")]
async fn backfill_job_fences_old_lease_after_takeover() {
    let _guard = lock_tests();

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:index-backfill-lease-it:{}", next_id("topic"));
    let job_id = next_id("backfill-lease-fence");
    insert_backfill_job(&pool, &job_id, BACKFILL_TARGET_POST_SEARCH_DOCUMENTS, None).await;

    let claimed = claim_backfill_job(&pool, BACKFILL_TARGET_POST_SEARCH_DOCUMENTS)
        .await
        .expect("claim backfill job")
        .expect("expected pending backfill job");
    assert_eq!(claimed.job_id, job_id);

    sqlx::query(
        "UPDATE cn_search.backfill_jobs \
         SET started_at = started_at + INTERVAL '1 second', updated_at = NOW() \
         WHERE job_id = $1",
    )
    .bind(&job_id)
    .execute(&pool)
    .await
    .expect("simulate lease takeover");

    let progress_err = update_backfill_job_progress(&pool, &job_id, &claimed.lease_started_at, 1)
        .await
        .expect_err("old lease should not update progress");
    assert!(
        progress_err.to_string().contains("lease lost"),
        "unexpected progress error: {progress_err}"
    );

    let complete_err = mark_backfill_job_succeeded(&pool, &job_id, &claimed.lease_started_at, 1)
        .await
        .expect_err("old lease should not mark completion");
    assert!(
        complete_err.to_string().contains("lease lost"),
        "unexpected complete error: {complete_err}"
    );

    mark_backfill_job_failed(
        &pool,
        &job_id,
        &claimed.lease_started_at,
        "old owner failure should be ignored",
    )
    .await
    .expect("mark failure from stale lease should be a no-op");

    let job_row = fetch_backfill_job(&pool, &job_id).await;
    assert_eq!(job_row.status, "running");
    assert_eq!(job_row.processed_rows, 0);
    assert!(job_row.error_message.is_none());

    cleanup_records(&pool, &topic_id, &[], &[job_id]).await;
}

#[tokio::test(flavor = "current_thread")]
async fn healthz_contract_status_transitions_when_dependency_fails() {
    let _guard = lock_tests();

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let dependency_status = Arc::new(AtomicU16::new(StatusCode::OK.as_u16()));
    let (relay_health_url, relay_handle) =
        spawn_dependency_health_server(Arc::clone(&dependency_status)).await;
    let mut health_targets = HashMap::new();
    health_targets.insert("relay".to_string(), relay_health_url);

    let mut state = build_state(pool);
    state.health_targets = Arc::new(health_targets);

    let ok_response = healthz(State(state.clone())).await.into_response();
    assert_eq!(ok_response.status(), StatusCode::OK);
    let ok_payload = response_json(ok_response).await;
    assert_eq!(ok_payload.get("status"), Some(&json!("ok")));

    dependency_status.store(StatusCode::SERVICE_UNAVAILABLE.as_u16(), Ordering::Relaxed);
    let failed_response = healthz(State(state)).await.into_response();
    assert_eq!(failed_response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let failed_payload = response_json(failed_response).await;
    assert_eq!(failed_payload.get("status"), Some(&json!("unavailable")));

    relay_handle.abort();
    let _ = relay_handle.await;
}

#[tokio::test(flavor = "current_thread")]
async fn metrics_contract_prometheus_content_type_shape_compatible() {
    let _guard = lock_tests();

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;
    let state = build_state(pool);

    metrics::observe_outbox_consumer_batch_size(SERVICE_NAME, CONSUMER_NAME, 3);
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
        body.contains("cn_up{service=\"cn-index\"} 1"),
        "metrics body did not contain cn_up for cn-index: {body}"
    );
    assert!(
        body.contains(
            "outbox_consumer_batches_total{consumer=\"index-v1\",result=\"success\",service=\"cn-index\"} "
        ),
        "metrics body did not contain outbox_consumer_batches_total success labels for cn-index: {body}"
    );
    assert!(
        body.contains(
            "outbox_consumer_batches_total{consumer=\"index-v1\",result=\"error\",service=\"cn-index\"} "
        ),
        "metrics body did not contain outbox_consumer_batches_total error labels for cn-index: {body}"
    );
    assert!(
        body.contains(
            "outbox_consumer_processing_duration_seconds_count{consumer=\"index-v1\",result=\"success\",service=\"cn-index\"} "
        ),
        "metrics body did not contain outbox_consumer_processing_duration_seconds labels for cn-index: {body}"
    );
    assert!(
        body.contains(
            "outbox_consumer_batch_size_count{consumer=\"index-v1\",service=\"cn-index\"} "
        ),
        "metrics body did not contain outbox_consumer_batch_size labels for cn-index: {body}"
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
