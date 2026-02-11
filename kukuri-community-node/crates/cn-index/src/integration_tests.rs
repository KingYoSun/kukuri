use super::*;
use axum::body::to_bytes;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres, Row};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use tokio::net::TcpListener;
use tokio::sync::{OnceCell, RwLock};

static MIGRATIONS: OnceCell<()> = OnceCell::const_new();
static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static UNIQUE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Default)]
struct MockMeiliState {
    indexes: Arc<RwLock<HashMap<String, MockIndex>>>,
}

#[derive(Clone, Default)]
struct MockIndex {
    settings: Option<Value>,
    documents: HashMap<String, Value>,
}

#[derive(Deserialize)]
struct CreateIndexPayload {
    uid: String,
}

struct ReindexJobRow {
    status: String,
    total_events: Option<i64>,
    processed_events: i64,
    cutoff_seq: Option<i64>,
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
        })
        .await;
}

fn build_state(pool: Pool<Postgres>, meili_url: &str) -> AppState {
    AppState {
        pool,
        config: service_config::static_handle(json!({})),
        meili: meili::MeiliClient::new(meili_url.to_string(), None).expect("meili client"),
        index_cache: Arc::new(RwLock::new(HashSet::new())),
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

async fn cleanup_records(
    pool: &Pool<Postgres>,
    topic_id: &str,
    event_ids: &[String],
    job_ids: &[String],
) {
    if !job_ids.is_empty() {
        let job_refs: Vec<&str> = job_ids.iter().map(String::as_str).collect();
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

    sqlx::query("DELETE FROM cn_admin.node_subscriptions WHERE topic_id = $1")
        .bind(topic_id)
        .execute(pool)
        .await
        .expect("cleanup topic subscription");
}

async fn mock_health() -> StatusCode {
    StatusCode::OK
}

async fn mock_get_index(
    State(state): State<MockMeiliState>,
    Path(uid): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let indexes = state.indexes.read().await;
    if indexes.contains_key(&uid) {
        Ok(Json(json!({ "uid": uid })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn mock_create_index(
    State(state): State<MockMeiliState>,
    Json(payload): Json<CreateIndexPayload>,
) -> (StatusCode, Json<Value>) {
    let mut indexes = state.indexes.write().await;
    indexes.entry(payload.uid).or_insert_with(|| MockIndex {
        settings: None,
        documents: HashMap::new(),
    });
    (
        StatusCode::ACCEPTED,
        Json(json!({ "taskUid": 1, "status": "enqueued" })),
    )
}

async fn mock_update_settings(
    State(state): State<MockMeiliState>,
    Path(uid): Path<String>,
    Json(settings): Json<Value>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let mut indexes = state.indexes.write().await;
    let Some(index) = indexes.get_mut(&uid) else {
        return Err(StatusCode::NOT_FOUND);
    };
    index.settings = Some(settings);
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "taskUid": 2, "status": "enqueued" })),
    ))
}

async fn mock_upsert_documents(
    State(state): State<MockMeiliState>,
    Path(uid): Path<String>,
    Json(documents): Json<Vec<Value>>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let mut indexes = state.indexes.write().await;
    let Some(index) = indexes.get_mut(&uid) else {
        return Err(StatusCode::NOT_FOUND);
    };
    for document in documents {
        let Some(event_id) = document.get("event_id").and_then(Value::as_str) else {
            return Err(StatusCode::BAD_REQUEST);
        };
        index.documents.insert(event_id.to_string(), document);
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "taskUid": 3, "status": "enqueued" })),
    ))
}

async fn mock_delete_batch(
    State(state): State<MockMeiliState>,
    Path(uid): Path<String>,
    Json(ids): Json<Vec<String>>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let mut indexes = state.indexes.write().await;
    let Some(index) = indexes.get_mut(&uid) else {
        return Err(StatusCode::NOT_FOUND);
    };
    for id in ids {
        index.documents.remove(&id);
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "taskUid": 4, "status": "enqueued" })),
    ))
}

async fn mock_delete_document(
    State(state): State<MockMeiliState>,
    Path((uid, id)): Path<(String, String)>,
) -> StatusCode {
    let mut indexes = state.indexes.write().await;
    let Some(index) = indexes.get_mut(&uid) else {
        return StatusCode::NOT_FOUND;
    };
    index.documents.remove(&id);
    StatusCode::ACCEPTED
}

async fn mock_delete_all(
    State(state): State<MockMeiliState>,
    Path(uid): Path<String>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let mut indexes = state.indexes.write().await;
    let Some(index) = indexes.get_mut(&uid) else {
        return Err(StatusCode::NOT_FOUND);
    };
    index.documents.clear();
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "taskUid": 5, "status": "enqueued" })),
    ))
}

async fn spawn_mock_meili() -> (String, MockMeiliState, tokio::task::JoinHandle<()>) {
    let state = MockMeiliState::default();
    let app = Router::new()
        .route("/health", get(mock_health))
        .route("/indexes", post(mock_create_index))
        .route("/indexes/{uid}", get(mock_get_index))
        .route("/indexes/{uid}/settings", patch(mock_update_settings))
        .route("/indexes/{uid}/documents", post(mock_upsert_documents))
        .route(
            "/indexes/{uid}/documents/delete-batch",
            post(mock_delete_batch),
        )
        .route("/indexes/{uid}/documents/delete-all", post(mock_delete_all))
        .route(
            "/indexes/{uid}/documents/{id}",
            delete(mock_delete_document),
        )
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock meili");
    let addr = listener.local_addr().expect("mock meili addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve mock meili");
    });

    (format!("http://{addr}"), state, handle)
}

async fn spawn_failing_meili() -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new().fallback(|| async {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": "forced meili failure" })),
        )
    });
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind failing meili");
    let addr = listener.local_addr().expect("failing meili addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve failing meili");
    });
    (format!("http://{addr}"), handle)
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

async fn index_document_ids(state: &MockMeiliState, uid: &str) -> Vec<String> {
    let indexes = state.indexes.read().await;
    let mut ids = indexes
        .get(uid)
        .map(|index| index.documents.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    ids.sort();
    ids
}

#[tokio::test(flavor = "current_thread")]
async fn outbox_upsert_delete_and_expiration_reflect_to_meili() {
    let _guard = lock_tests();

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:index-it:{}", next_id("topic"));
    let now = cn_core::auth::unix_seconds().expect("unix seconds") as i64;
    let event_upsert_id = next_id("event-upsert");
    let event_expire_id = next_id("event-expire");
    let event_upsert = raw_event(&event_upsert_id, &topic_id, now, "upsert-content");
    let event_expire = raw_event(&event_expire_id, &topic_id, now + 1, "expire-content");
    let uid = meili::topic_index_uid(&topic_id);

    insert_event(&pool, &topic_id, &event_upsert, None).await;
    insert_event(&pool, &topic_id, &event_expire, Some(now + 3600)).await;

    let (meili_url, meili_state, meili_handle) = spawn_mock_meili().await;
    let state = build_state(pool.clone(), &meili_url);

    let upsert_seq = insert_outbox_row(&pool, "upsert", &topic_id, &event_upsert, None).await;
    let upsert_rows = fetch_outbox_batch(&pool, upsert_seq - 1, 10)
        .await
        .expect("fetch upsert rows");
    assert_eq!(upsert_rows.len(), 1);
    handle_outbox_row(&state, &upsert_rows[0])
        .await
        .expect("handle upsert row");
    assert_eq!(
        index_document_ids(&meili_state, &uid).await,
        vec![event_upsert_id]
    );

    let delete_seq = insert_outbox_row(&pool, "delete", &topic_id, &event_upsert, None).await;
    let delete_rows = fetch_outbox_batch(&pool, delete_seq - 1, 10)
        .await
        .expect("fetch delete rows");
    assert_eq!(delete_rows.len(), 1);
    handle_outbox_row(&state, &delete_rows[0])
        .await
        .expect("handle delete row");
    assert!(index_document_ids(&meili_state, &uid).await.is_empty());

    let expire_upsert_seq =
        insert_outbox_row(&pool, "upsert", &topic_id, &event_expire, None).await;
    let expire_rows = fetch_outbox_batch(&pool, expire_upsert_seq - 1, 10)
        .await
        .expect("fetch expiration upsert rows");
    assert_eq!(expire_rows.len(), 1);
    handle_outbox_row(&state, &expire_rows[0])
        .await
        .expect("handle expiration upsert row");
    assert_eq!(
        index_document_ids(&meili_state, &uid).await,
        vec![event_expire_id.clone()]
    );

    sqlx::query("UPDATE cn_relay.events SET expires_at = $1 WHERE event_id = $2")
        .bind(now - 1)
        .bind(&event_expire.id)
        .execute(&pool)
        .await
        .expect("expire event");
    expire_events_once(&state)
        .await
        .expect("run expiration sweep");

    assert!(index_document_ids(&meili_state, &uid).await.is_empty());
    let expired_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cn_index.expired_events WHERE event_id = $1 AND topic_id = $2",
    )
    .bind(&event_expire.id)
    .bind(&topic_id)
    .fetch_one(&pool)
    .await
    .expect("count expired rows");
    assert_eq!(expired_rows, 1);

    cleanup_records(
        &pool,
        &topic_id,
        &[event_upsert.id.clone(), event_expire.id.clone()],
        &[],
    )
    .await;

    meili_handle.abort();
    let _ = meili_handle.await;
}

#[tokio::test(flavor = "current_thread")]
async fn reindex_job_transitions_pending_running_succeeded_and_updates_meili() {
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

    let (meili_url, meili_state, meili_handle) = spawn_mock_meili().await;
    let state = build_state(pool.clone(), &meili_url);
    let uid = meili::topic_index_uid(&topic_id);

    load_last_seq(&pool)
        .await
        .expect("initialize consumer offset");
    state
        .meili
        .ensure_index(&uid, "event_id", Some(default_index_settings()))
        .await
        .expect("ensure index");
    state
        .meili
        .upsert_documents(
            &uid,
            &[json!({
                "event_id": "stale-doc",
                "topic_id": topic_id.clone(),
                "kind": 1,
                "author": "stale",
                "created_at": now,
                "title": "stale",
                "summary": "stale",
                "content": "stale",
                "tags": []
            })],
        )
        .await
        .expect("seed stale document");
    assert_eq!(
        index_document_ids(&meili_state, &uid).await,
        vec!["stale-doc".to_string()]
    );

    let job_id = next_id("reindex-success");
    insert_reindex_job(&pool, &job_id, &topic_id).await;

    let pending_job = fetch_reindex_job(&pool, &job_id).await;
    assert_eq!(pending_job.status, "pending");
    assert!(pending_job.started_at.is_none());
    assert!(pending_job.completed_at.is_none());

    let claimed = claim_reindex_job(&pool)
        .await
        .expect("claim reindex job")
        .expect("expected pending job");
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

    let mut expected_ids = vec![event_a.id.clone(), event_b.id.clone()];
    expected_ids.sort();
    assert_eq!(index_document_ids(&meili_state, &uid).await, expected_ids);

    cleanup_records(
        &pool,
        &topic_id,
        &[event_a.id.clone(), event_b.id.clone()],
        &[job_id],
    )
    .await;

    meili_handle.abort();
    let _ = meili_handle.await;
}

#[tokio::test(flavor = "current_thread")]
async fn reindex_job_transitions_pending_running_failed_on_meili_error() {
    let _guard = lock_tests();

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:index-it:{}", next_id("topic"));
    let event = raw_event(
        &next_id("event-failed"),
        &topic_id,
        cn_core::auth::unix_seconds().expect("unix seconds") as i64,
        "reindex-failed-event",
    );
    insert_event(&pool, &topic_id, &event, None).await;

    let (meili_url, failing_meili_handle) = spawn_failing_meili().await;
    let state = build_state(pool.clone(), &meili_url);

    let job_id = next_id("reindex-failed");
    insert_reindex_job(&pool, &job_id, &topic_id).await;

    let pending_job = fetch_reindex_job(&pool, &job_id).await;
    assert_eq!(pending_job.status, "pending");

    let claimed = claim_reindex_job(&pool)
        .await
        .expect("claim reindex job")
        .expect("expected pending job");
    assert_eq!(claimed.job_id, job_id);

    let running_job = fetch_reindex_job(&pool, &job_id).await;
    assert_eq!(running_job.status, "running");
    assert!(running_job.started_at.is_some());

    let err = run_reindex_job(&state, claimed)
        .await
        .expect_err("reindex should fail");
    assert!(err.to_string().contains("500"), "unexpected error: {err}");

    let failed_job = fetch_reindex_job(&pool, &job_id).await;
    assert_eq!(failed_job.status, "failed");
    assert_eq!(failed_job.processed_events, 0);
    assert!(failed_job
        .error_message
        .as_deref()
        .unwrap_or("")
        .contains("500"));
    assert!(failed_job.started_at.is_some());
    assert!(failed_job.completed_at.is_some());

    cleanup_records(&pool, &topic_id, &[event.id.clone()], &[job_id]).await;

    failing_meili_handle.abort();
    let _ = failing_meili_handle.await;
}

#[tokio::test(flavor = "current_thread")]
async fn healthz_contract_status_transitions_when_dependency_fails() {
    let _guard = lock_tests();

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let (meili_url, _meili_state, meili_handle) = spawn_mock_meili().await;
    let dependency_status = Arc::new(AtomicU16::new(StatusCode::OK.as_u16()));
    let (relay_health_url, relay_handle) =
        spawn_dependency_health_server(Arc::clone(&dependency_status)).await;
    let mut health_targets = HashMap::new();
    health_targets.insert("relay".to_string(), relay_health_url);

    let mut state = build_state(pool, &meili_url);
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
    meili_handle.abort();
    let _ = meili_handle.await;
}

#[tokio::test(flavor = "current_thread")]
async fn metrics_contract_prometheus_content_type_shape_compatible() {
    let _guard = lock_tests();

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let (meili_url, _meili_state, meili_handle) = spawn_mock_meili().await;
    let state = build_state(pool, &meili_url);

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

    meili_handle.abort();
    let _ = meili_handle.await;
}
