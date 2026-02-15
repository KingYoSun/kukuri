use crate::{
    access_control, auth, dashboard, dsar, moderation, policies, reindex, services, subscriptions,
    trust, AppState,
};
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::routing::{get, post, put};
use axum::Router;
use cn_core::service_config;
use nostr_sdk::prelude::Keys;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres, Row};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use tokio::sync::OnceCell;
use tower::ServiceExt;
use uuid::Uuid;

static MIGRATIONS: OnceCell<()> = OnceCell::const_new();

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/postgres".to_string())
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

async fn test_state() -> AppState {
    test_state_with_health_targets(HashMap::new()).await
}

async fn test_state_with_health_targets(health_targets: HashMap<String, String>) -> AppState {
    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let admin_config = service_config::static_handle(serde_json::json!({
        "session_cookie": true,
        "session_ttl_seconds": 86400
    }));

    AppState {
        pool,
        admin_config,
        health_targets: Arc::new(health_targets),
        health_client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(1))
            .build()
            .expect("build health client"),
        dashboard_cache: Arc::new(tokio::sync::Mutex::new(dashboard::DashboardCache::default())),
        node_keys: Keys::generate(),
    }
}

async fn insert_admin_session(pool: &Pool<Postgres>) -> String {
    let (admin_user_id, _) = insert_admin_user(pool, "test-password").await;
    let session_id = Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);
    sqlx::query(
        "INSERT INTO cn_admin.admin_sessions          (session_id, admin_user_id, expires_at)          VALUES ($1, $2, $3)",
    )
    .bind(&session_id)
    .bind(&admin_user_id)
    .bind(expires_at)
    .execute(pool)
    .await
    .expect("insert admin session");

    session_id
}

async fn insert_admin_user(pool: &Pool<Postgres>, password: &str) -> (String, String) {
    let admin_user_id = Uuid::new_v4().to_string();
    let username = format!("admin-{}", &admin_user_id[..8]);
    let password_hash = cn_core::admin::hash_password(password).expect("hash password");

    sqlx::query(
        "INSERT INTO cn_admin.admin_users          (admin_user_id, username, password_hash, is_active)          VALUES ($1, $2, $3, TRUE)",
    )
    .bind(&admin_user_id)
    .bind(&username)
    .bind(&password_hash)
    .execute(pool)
    .await
    .expect("insert admin user");

    (admin_user_id, username)
}

async fn insert_membership(pool: &Pool<Postgres>, topic_id: &str, scope: &str, pubkey: &str) {
    sqlx::query(
        "INSERT INTO cn_user.topic_memberships          (topic_id, scope, pubkey, status)          VALUES ($1, $2, $3, 'active')          ON CONFLICT (topic_id, scope, pubkey) DO NOTHING",
    )
    .bind(topic_id)
    .bind(scope)
    .bind(pubkey)
    .execute(pool)
    .await
    .expect("insert membership");
}

async fn insert_trust_scores(pool: &Pool<Postgres>, subject_pubkey: &str) {
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO cn_trust.report_scores          (subject_pubkey, score, report_count, label_count, window_start, window_end, attestation_id, attestation_exp)          VALUES ($1, 0.85, 4, 2, $2, $3, NULL, NULL)          ON CONFLICT (subject_pubkey) DO UPDATE SET score = EXCLUDED.score, report_count = EXCLUDED.report_count, label_count = EXCLUDED.label_count, window_start = EXCLUDED.window_start, window_end = EXCLUDED.window_end, updated_at = NOW()",
    )
    .bind(subject_pubkey)
    .bind(now - 86400)
    .bind(now)
    .execute(pool)
    .await
    .expect("insert trust report score");

    sqlx::query(
        "INSERT INTO cn_trust.communication_scores          (subject_pubkey, score, interaction_count, peer_count, window_start, window_end, attestation_id, attestation_exp)          VALUES ($1, 0.65, 8, 3, $2, $3, NULL, NULL)          ON CONFLICT (subject_pubkey) DO UPDATE SET score = EXCLUDED.score, interaction_count = EXCLUDED.interaction_count, peer_count = EXCLUDED.peer_count, window_start = EXCLUDED.window_start, window_end = EXCLUDED.window_end, updated_at = NOW()",
    )
    .bind(subject_pubkey)
    .bind(now - 86400)
    .bind(now)
    .execute(pool)
    .await
    .expect("insert trust communication score");
}

async fn post_json(
    app: Router,
    uri: &str,
    payload: Value,
    session_id: &str,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .header("cookie", format!("cn_admin_session={session_id}"))
        .body(Body::from(payload.to_string()))
        .expect("request");
    let response = app.oneshot(request).await.expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    let payload: Value = serde_json::from_slice(&body).expect("json body");
    (status, payload)
}

async fn put_json(app: Router, uri: &str, payload: Value, session_id: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("PUT")
        .uri(uri)
        .header("content-type", "application/json")
        .header("cookie", format!("cn_admin_session={session_id}"))
        .body(Body::from(payload.to_string()))
        .expect("request");
    let response = app.oneshot(request).await.expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    let payload: Value = serde_json::from_slice(&body).expect("json body");
    (status, payload)
}

async fn get_json(app: Router, uri: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .header("host", "localhost:8081")
        .body(Body::empty())
        .expect("request");
    let response = app.oneshot(request).await.expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    let payload: Value = serde_json::from_slice(&body).expect("json body");
    (status, payload)
}

async fn get_json_with_session(app: Router, uri: &str, session_id: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .header("host", "localhost:8081")
        .header("cookie", format!("cn_admin_session={session_id}"))
        .body(Body::empty())
        .expect("request");
    let response = app.oneshot(request).await.expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    let payload: Value = serde_json::from_slice(&body).expect("json body");
    (status, payload)
}

async fn get_text(app: Router, uri: &str) -> (StatusCode, Option<String>, String) {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .header("host", "localhost:8081")
        .body(Body::empty())
        .expect("request");
    let response = app.oneshot(request).await.expect("response");
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(std::string::ToString::to_string);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    (
        status,
        content_type,
        String::from_utf8_lossy(&body).to_string(),
    )
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

async fn insert_service_health(pool: &Pool<Postgres>, service: &str, status: &str, details: Value) {
    sqlx::query(
        "INSERT INTO cn_admin.service_health          (service, status, checked_at, details_json)          VALUES ($1, $2, NOW(), $3)          ON CONFLICT (service) DO UPDATE SET status = EXCLUDED.status, checked_at = EXCLUDED.checked_at, details_json = EXCLUDED.details_json",
    )
    .bind(service)
    .bind(status)
    .bind(details)
    .execute(pool)
    .await
    .expect("insert service health");
}

async fn upsert_service_config(pool: &Pool<Postgres>, service: &str, config_json: Value) {
    sqlx::query(
        "INSERT INTO cn_admin.service_configs          (service, version, config_json, updated_by)          VALUES ($1, 1, $2, 'contract-test')          ON CONFLICT (service) DO UPDATE SET config_json = EXCLUDED.config_json, version = cn_admin.service_configs.version + 1, updated_by = EXCLUDED.updated_by, updated_at = NOW()",
    )
    .bind(service)
    .bind(config_json)
    .execute(pool)
    .await
    .expect("upsert service config");
}

async fn fetch_service_health_row(pool: &Pool<Postgres>, service: &str) -> (String, Value) {
    let row =
        sqlx::query("SELECT status, details_json FROM cn_admin.service_health WHERE service = $1")
            .bind(service)
            .fetch_one(pool)
            .await
            .expect("fetch service health row");
    let status: String = row.try_get("status").expect("status");
    let details: Value = row
        .try_get::<Option<Value>, _>("details_json")
        .expect("details_json")
        .expect("details_json should not be null");
    (status, details)
}

async fn spawn_healthz_mock(status_code: Arc<AtomicU16>) -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new().route(
        "/healthz",
        get({
            let status_code = Arc::clone(&status_code);
            move || {
                let status_code = Arc::clone(&status_code);
                async move {
                    let status = StatusCode::from_u16(status_code.load(Ordering::Relaxed))
                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                    (status, axum::Json(json!({ "status": "mock" })))
                }
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind health mock");
    let addr = listener.local_addr().expect("health mock addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve health mock");
    });

    (format!("http://{addr}/healthz"), handle)
}

async fn spawn_relay_metrics_mock(metrics_body: String) -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route(
            "/healthz",
            get(|| async { (StatusCode::OK, axum::Json(json!({ "status": "ok" }))) }),
        )
        .route(
            "/metrics",
            get(move || {
                let metrics_body = metrics_body.clone();
                async move {
                    (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
                        metrics_body,
                    )
                }
            }),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind relay metrics mock");
    let addr = listener.local_addr().expect("relay metrics mock addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve relay metrics mock");
    });

    (format!("http://{addr}/healthz"), handle)
}

async fn insert_report(pool: &Pool<Postgres>, reporter_pubkey: &str, target: &str, reason: &str) {
    let report_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO cn_user.reports          (report_id, reporter_pubkey, target, reason, report_event_json)          VALUES ($1, $2, $3, $4, NULL)",
    )
    .bind(report_id)
    .bind(reporter_pubkey)
    .bind(target)
    .bind(reason)
    .execute(pool)
    .await
    .expect("insert report");
}

async fn insert_subscription_request(
    pool: &Pool<Postgres>,
    requester_pubkey: &str,
    topic_id: &str,
    requested_services: Value,
) -> String {
    let request_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO cn_user.topic_subscription_requests          (request_id, requester_pubkey, topic_id, requested_services, status)          VALUES ($1, $2, $3, $4, 'pending')",
    )
    .bind(&request_id)
    .bind(requester_pubkey)
    .bind(topic_id)
    .bind(requested_services)
    .execute(pool)
    .await
    .expect("insert subscription request");
    request_id
}

async fn insert_usage_counter(
    pool: &Pool<Postgres>,
    subscriber_pubkey: &str,
    metric: &str,
    count: i64,
) {
    sqlx::query(
        "INSERT INTO cn_user.usage_counters_daily          (subscriber_pubkey, metric, day, count)          VALUES ($1, $2, $3, $4)          ON CONFLICT (subscriber_pubkey, metric, day) DO UPDATE SET count = EXCLUDED.count",
    )
    .bind(subscriber_pubkey)
    .bind(metric)
    .bind(chrono::Utc::now().date_naive())
    .bind(count)
    .execute(pool)
    .await
    .expect("insert usage counter");
}

async fn insert_audit_log(
    pool: &Pool<Postgres>,
    actor_admin_user_id: &str,
    action: &str,
    target: &str,
    diff_json: Value,
    request_id: &str,
) {
    sqlx::query(
        "INSERT INTO cn_admin.audit_logs          (actor_admin_user_id, action, target, diff_json, request_id)          VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(actor_admin_user_id)
    .bind(action)
    .bind(target)
    .bind(diff_json)
    .bind(request_id)
    .execute(pool)
    .await
    .expect("insert audit log");
}

async fn ensure_audit_failure_trigger(pool: &Pool<Postgres>) {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS cn_admin.test_audit_failures (
            failure_id BIGSERIAL PRIMARY KEY,
            action TEXT NOT NULL,
            target TEXT NOT NULL,
            diff_filter JSONB NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("create test_audit_failures table");

    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION cn_admin.fail_audit_for_test()
        RETURNS trigger
        LANGUAGE plpgsql
        AS $$
        BEGIN
            IF EXISTS (
                SELECT 1
                  FROM cn_admin.test_audit_failures fail
                 WHERE fail.action = NEW.action
                   AND fail.target = NEW.target
                   AND (
                       fail.diff_filter IS NULL
                       OR COALESCE(NEW.diff_json, '{}'::jsonb) @> fail.diff_filter
                   )
            ) THEN
                RAISE EXCEPTION 'forced audit failure for contract test';
            END IF;
            RETURN NEW;
        END;
        $$;
        "#,
    )
    .execute(pool)
    .await
    .expect("create fail_audit_for_test function");

    sqlx::query("DROP TRIGGER IF EXISTS test_audit_failures_trigger ON cn_admin.audit_logs")
        .execute(pool)
        .await
        .expect("drop test_audit_failures_trigger");
    sqlx::query(
        r#"
        CREATE TRIGGER test_audit_failures_trigger
        BEFORE INSERT ON cn_admin.audit_logs
        FOR EACH ROW
        EXECUTE FUNCTION cn_admin.fail_audit_for_test()
        "#,
    )
    .execute(pool)
    .await
    .expect("create test_audit_failures_trigger");
}

async fn register_audit_failure(
    pool: &Pool<Postgres>,
    action: &str,
    target: &str,
    diff_filter: Option<Value>,
) {
    ensure_audit_failure_trigger(pool).await;
    sqlx::query(
        "INSERT INTO cn_admin.test_audit_failures (action, target, diff_filter) VALUES ($1, $2, $3)",
    )
    .bind(action)
    .bind(target)
    .bind(diff_filter)
    .execute(pool)
    .await
    .expect("insert test audit failure");
}

async fn ensure_commit_failure_trigger(pool: &Pool<Postgres>) {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS cn_admin.test_commit_failures (
            failure_id BIGSERIAL PRIMARY KEY,
            action TEXT NOT NULL,
            target TEXT NOT NULL,
            diff_filter JSONB NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("create test_commit_failures table");

    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION cn_admin.fail_commit_for_test()
        RETURNS trigger
        LANGUAGE plpgsql
        AS $$
        BEGIN
            IF EXISTS (
                SELECT 1
                  FROM cn_admin.test_commit_failures fail
                 WHERE fail.action = NEW.action
                   AND fail.target = NEW.target
                   AND (
                       fail.diff_filter IS NULL
                       OR COALESCE(NEW.diff_json, '{}'::jsonb) @> fail.diff_filter
                   )
            ) THEN
                RAISE EXCEPTION 'forced commit failure for contract test';
            END IF;
            RETURN NEW;
        END;
        $$;
        "#,
    )
    .execute(pool)
    .await
    .expect("create fail_commit_for_test function");

    sqlx::query("DROP TRIGGER IF EXISTS test_commit_failures_trigger ON cn_admin.audit_logs")
        .execute(pool)
        .await
        .expect("drop test_commit_failures_trigger");
    sqlx::query(
        r#"
        CREATE CONSTRAINT TRIGGER test_commit_failures_trigger
        AFTER INSERT ON cn_admin.audit_logs
        DEFERRABLE INITIALLY DEFERRED
        FOR EACH ROW
        EXECUTE FUNCTION cn_admin.fail_commit_for_test()
        "#,
    )
    .execute(pool)
    .await
    .expect("create test_commit_failures_trigger");
}

async fn register_commit_failure(
    pool: &Pool<Postgres>,
    action: &str,
    target: &str,
    diff_filter: Option<Value>,
) {
    ensure_commit_failure_trigger(pool).await;
    sqlx::query(
        "INSERT INTO cn_admin.test_commit_failures (action, target, diff_filter) VALUES ($1, $2, $3)",
    )
    .bind(action)
    .bind(target)
    .bind(diff_filter)
    .execute(pool)
    .await
    .expect("insert test commit failure");
}

async fn ensure_logout_failure_trigger(pool: &Pool<Postgres>) {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS cn_admin.test_logout_failures (
            failure_id BIGSERIAL PRIMARY KEY,
            session_id TEXT NOT NULL UNIQUE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("create test_logout_failures table");

    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION cn_admin.fail_logout_delete_for_test()
        RETURNS trigger
        LANGUAGE plpgsql
        AS $$
        BEGIN
            IF EXISTS (
                SELECT 1
                  FROM cn_admin.test_logout_failures fail
                 WHERE fail.session_id = OLD.session_id
            ) THEN
                RAISE EXCEPTION 'forced logout delete failure for contract test';
            END IF;
            RETURN OLD;
        END;
        $$;
        "#,
    )
    .execute(pool)
    .await
    .expect("create fail_logout_delete_for_test function");

    sqlx::query("DROP TRIGGER IF EXISTS test_logout_failures_trigger ON cn_admin.admin_sessions")
        .execute(pool)
        .await
        .expect("drop test_logout_failures_trigger");
    sqlx::query(
        r#"
        CREATE TRIGGER test_logout_failures_trigger
        BEFORE DELETE ON cn_admin.admin_sessions
        FOR EACH ROW
        EXECUTE FUNCTION cn_admin.fail_logout_delete_for_test()
        "#,
    )
    .execute(pool)
    .await
    .expect("create test_logout_failures_trigger");
}

async fn register_logout_failure(pool: &Pool<Postgres>, session_id: &str) {
    ensure_logout_failure_trigger(pool).await;
    sqlx::query("INSERT INTO cn_admin.test_logout_failures (session_id) VALUES ($1)")
        .bind(session_id)
        .execute(pool)
        .await
        .expect("insert test logout failure");
}

fn assert_audit_log_required(status: StatusCode, payload: &Value) {
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        payload.get("code").and_then(Value::as_str),
        Some("AUDIT_LOG_ERROR")
    );
}

fn assert_db_error(status: StatusCode, payload: &Value) {
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        payload.get("code").and_then(Value::as_str),
        Some("DB_ERROR")
    );
}

async fn insert_export_request(
    pool: &Pool<Postgres>,
    export_request_id: &str,
    requester_pubkey: &str,
    status: &str,
    error_message: Option<&str>,
) {
    let completed_at = if status == "completed" || status == "failed" {
        Some(chrono::Utc::now())
    } else {
        None
    };
    sqlx::query(
        "INSERT INTO cn_user.personal_data_export_requests \
         (export_request_id, requester_pubkey, status, completed_at, error_message) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(export_request_id)
    .bind(requester_pubkey)
    .bind(status)
    .bind(completed_at)
    .bind(error_message)
    .execute(pool)
    .await
    .expect("insert export request");
}

async fn insert_deletion_request(
    pool: &Pool<Postgres>,
    deletion_request_id: &str,
    requester_pubkey: &str,
    status: &str,
    error_message: Option<&str>,
) {
    let completed_at = if status == "completed" || status == "failed" {
        Some(chrono::Utc::now())
    } else {
        None
    };
    sqlx::query(
        "INSERT INTO cn_user.personal_data_deletion_requests \
         (deletion_request_id, requester_pubkey, status, completed_at, error_message) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(deletion_request_id)
    .bind(requester_pubkey)
    .bind(status)
    .bind(completed_at)
    .bind(error_message)
    .execute(pool)
    .await
    .expect("insert deletion request");
}

#[tokio::test]
async fn access_control_rotate_contract_success() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let topic_id = format!("kukuri:contract-{}", Uuid::new_v4());
    let scope = "invite";
    let member_pubkey = Keys::generate().public_key().to_hex();
    insert_membership(&state.pool, &topic_id, scope, &member_pubkey).await;

    let app = Router::new()
        .route(
            "/v1/admin/access-control/rotate",
            post(access_control::rotate_epoch),
        )
        .with_state(state);

    let (status, payload) = post_json(
        app,
        "/v1/admin/access-control/rotate",
        json!({
            "topic_id": topic_id,
            "scope": scope
        }),
        &session_id,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("topic_id").and_then(Value::as_str),
        Some(topic_id.as_str())
    );
    assert_eq!(payload.get("scope").and_then(Value::as_str), Some(scope));
    assert_eq!(payload.get("recipients").and_then(Value::as_u64), Some(1));
    assert_eq!(
        payload.get("previous_epoch").and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(payload.get("new_epoch").and_then(Value::as_i64), Some(1));
    let distribution_results = payload
        .get("distribution_results")
        .and_then(Value::as_array)
        .expect("distribution_results array");
    assert_eq!(distribution_results.len(), 1);
    assert_eq!(
        distribution_results[0]
            .get("recipient_pubkey")
            .and_then(Value::as_str),
        Some(member_pubkey.as_str())
    );
    assert_eq!(
        distribution_results[0]
            .get("status")
            .and_then(Value::as_str),
        Some("success")
    );
    assert_eq!(
        distribution_results[0]
            .get("reason")
            .and_then(Value::as_str),
        None
    );
}

#[tokio::test]
async fn access_control_revoke_contract_success() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let topic_id = format!("kukuri:contract-{}", Uuid::new_v4());
    let scope = "friend";
    let member_pubkey = Keys::generate().public_key().to_hex();
    insert_membership(&state.pool, &topic_id, scope, &member_pubkey).await;

    let app = Router::new()
        .route(
            "/v1/admin/access-control/revoke",
            post(access_control::revoke_member),
        )
        .with_state(state);

    let (status, payload) = post_json(
        app,
        "/v1/admin/access-control/revoke",
        json!({
            "topic_id": topic_id,
            "scope": scope,
            "pubkey": member_pubkey,
            "reason": "contract-test"
        }),
        &session_id,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("topic_id").and_then(Value::as_str),
        Some(topic_id.as_str())
    );
    assert_eq!(payload.get("scope").and_then(Value::as_str), Some(scope));
    assert_eq!(
        payload.get("revoked_pubkey").and_then(Value::as_str),
        Some(member_pubkey.as_str())
    );
    assert_eq!(payload.get("recipients").and_then(Value::as_u64), Some(0));
    assert_eq!(
        payload.get("previous_epoch").and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(payload.get("new_epoch").and_then(Value::as_i64), Some(1));
    let distribution_results = payload
        .get("distribution_results")
        .and_then(Value::as_array)
        .expect("distribution_results array");
    assert!(distribution_results.is_empty());
}

#[tokio::test]
async fn access_control_memberships_contract_search_success() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let topic_id = format!("kukuri:contract-{}", Uuid::new_v4());
    let invite_pubkey = Keys::generate().public_key().to_hex();
    let friend_pubkey = Keys::generate().public_key().to_hex();
    insert_membership(&state.pool, &topic_id, "invite", &invite_pubkey).await;
    insert_membership(&state.pool, &topic_id, "friend", &friend_pubkey).await;

    let app = Router::new()
        .route(
            "/v1/admin/access-control/memberships",
            get(access_control::list_memberships),
        )
        .with_state(state);

    let uri = format!(
        "/v1/admin/access-control/memberships?topic_id={topic_id}&scope=invite&pubkey={invite_pubkey}&limit=10"
    );
    let (status, payload) = get_json_with_session(app, &uri, &session_id).await;

    assert_eq!(status, StatusCode::OK);
    let rows = payload.as_array().expect("array payload");
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].get("topic_id").and_then(Value::as_str),
        Some(topic_id.as_str())
    );
    assert_eq!(rows[0].get("scope").and_then(Value::as_str), Some("invite"));
    assert_eq!(
        rows[0].get("pubkey").and_then(Value::as_str),
        Some(invite_pubkey.as_str())
    );
    assert_eq!(
        rows[0].get("status").and_then(Value::as_str),
        Some("active")
    );
}

#[tokio::test]
async fn access_control_distribution_results_contract_search_success() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let topic_id = format!("kukuri:contract-{}", Uuid::new_v4());
    let scope = "invite";
    let success_pubkey = Keys::generate().public_key().to_hex();
    let failed_pubkey = "failed-recipient";

    sqlx::query(
        "INSERT INTO cn_user.key_envelope_distribution_results          (topic_id, scope, epoch, recipient_pubkey, status, reason)          VALUES ($1, $2, 3, $3, 'success', NULL),                 ($1, $2, 3, $4, 'failed', 'invalid pubkey')",
    )
    .bind(&topic_id)
    .bind(scope)
    .bind(&success_pubkey)
    .bind(failed_pubkey)
    .execute(&state.pool)
    .await
    .expect("insert distribution rows");

    let app = Router::new()
        .route(
            "/v1/admin/access-control/distribution-results",
            get(access_control::list_distribution_results),
        )
        .with_state(state);

    let uri = format!(
        "/v1/admin/access-control/distribution-results?topic_id={topic_id}&scope={scope}&status=failed&limit=10"
    );
    let (status, payload) = get_json_with_session(app, &uri, &session_id).await;

    assert_eq!(status, StatusCode::OK);
    let rows = payload.as_array().expect("array payload");
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].get("topic_id").and_then(Value::as_str),
        Some(topic_id.as_str())
    );
    assert_eq!(rows[0].get("scope").and_then(Value::as_str), Some(scope));
    assert_eq!(rows[0].get("epoch").and_then(Value::as_i64), Some(3));
    assert_eq!(
        rows[0].get("recipient_pubkey").and_then(Value::as_str),
        Some(failed_pubkey)
    );
    assert_eq!(
        rows[0].get("status").and_then(Value::as_str),
        Some("failed")
    );
    assert_eq!(
        rows[0].get("reason").and_then(Value::as_str),
        Some("invalid pubkey")
    );
}

#[tokio::test]
async fn access_control_invite_contract_issue_list_revoke_success() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let topic_id = format!("kukuri:invite-contract:{}", Uuid::new_v4());
    let nonce = format!("invite-{}", Uuid::new_v4().simple());

    let app = Router::new()
        .route(
            "/v1/admin/access-control/invites",
            get(access_control::list_invites).post(access_control::issue_invite),
        )
        .route(
            "/v1/admin/access-control/invites/{nonce}/revoke",
            post(access_control::revoke_invite),
        )
        .with_state(state);

    let (status, payload) = post_json(
        app.clone(),
        "/v1/admin/access-control/invites",
        json!({
            "topic_id": topic_id,
            "scope": "invite",
            "expires_in_seconds": 3600,
            "max_uses": 2,
            "nonce": nonce
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload.get("scope").and_then(Value::as_str), Some("invite"));
    assert_eq!(payload.get("max_uses").and_then(Value::as_i64), Some(2));
    assert_eq!(payload.get("used_count").and_then(Value::as_i64), Some(0));
    assert_eq!(
        payload.get("status").and_then(Value::as_str),
        Some("active")
    );
    let issued_nonce = payload
        .get("nonce")
        .and_then(Value::as_str)
        .expect("nonce")
        .to_string();

    let (status, payload) = get_json_with_session(
        app.clone(),
        &format!("/v1/admin/access-control/invites?status=active&topic_id={topic_id}"),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rows = payload.as_array().expect("array payload");
    assert!(rows.iter().any(|row| {
        row.get("nonce").and_then(Value::as_str) == Some(issued_nonce.as_str())
            && row.get("status").and_then(Value::as_str) == Some("active")
    }));

    let (status, payload) = post_json(
        app.clone(),
        &format!("/v1/admin/access-control/invites/{issued_nonce}/revoke"),
        json!({}),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("status").and_then(Value::as_str),
        Some("revoked")
    );

    let (status, payload) = get_json_with_session(
        app,
        &format!("/v1/admin/access-control/invites?status=revoked&topic_id={topic_id}"),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rows = payload.as_array().expect("array payload");
    assert!(rows.iter().any(|row| {
        row.get("nonce").and_then(Value::as_str) == Some(issued_nonce.as_str())
            && row.get("status").and_then(Value::as_str) == Some("revoked")
    }));
}

#[tokio::test]
async fn reindex_contract_success() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let topic_id = format!("kukuri:contract-{}", Uuid::new_v4());

    let app = Router::new()
        .route("/v1/reindex", post(reindex::enqueue_reindex))
        .with_state(state);

    let (status, payload) = post_json(
        app,
        "/v1/reindex",
        json!({ "topic_id": topic_id }),
        &session_id,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let job_id = payload
        .get("job_id")
        .and_then(Value::as_str)
        .expect("job_id");
    assert!(Uuid::parse_str(job_id).is_ok());
    assert_eq!(
        payload.get("status").and_then(Value::as_str),
        Some("pending")
    );
}

#[tokio::test]
async fn openapi_contract_contains_admin_paths() {
    let app = Router::new().route("/v1/openapi.json", get(crate::openapi_json));
    let (status, payload) = get_json(app, "/v1/openapi.json").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("openapi").and_then(Value::as_str),
        Some("3.0.3")
    );
    assert!(payload
        .pointer("/paths/~1v1~1admin~1auth~1login/post")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1services/get")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1moderation~1rules/get")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1moderation~1rules~1test/post")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1moderation~1labels~1{label_id}~1review/post")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1moderation~1labels~1{label_id}~1rejudge/post")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1dashboard/get")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1auth~1logout/post/responses/500")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1services~1{service}~1config/put/responses/400")
        .is_some());
    assert!(payload
        .pointer(
            "/paths/~1v1~1admin~1subscription-requests~1{request_id}~1approve/post/responses/429"
        )
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1access-control~1memberships/get")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1access-control~1distribution-results/get")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1access-control~1invites/get")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1access-control~1invites/post")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1access-control~1invites~1{nonce}~1revoke/post")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1personal-data-jobs/get")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1personal-data-jobs~1{job_type}~1{job_id}~1retry/post")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1personal-data-jobs~1{job_type}~1{job_id}~1cancel/post")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1trust~1schedules/get")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1trust~1targets/get")
        .is_some());
    assert!(payload.pointer("/paths/~1v1~1reindex/post").is_some());
    assert!(payload.pointer("/components/schemas/ServiceInfo").is_some());
    assert!(payload
        .pointer("/components/schemas/DashboardSnapshot")
        .is_some());
    assert!(payload
        .pointer("/components/schemas/TrustScheduleRow")
        .is_some());
    assert!(payload.pointer("/components/schemas/DsarJobRow").is_some());
}

#[tokio::test]
async fn healthz_contract_success_shape_compatible() {
    let state = test_state().await;
    let app = Router::new()
        .route("/healthz", get(crate::healthz))
        .with_state(state);
    let (status, payload) = get_json(app, "/healthz").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload.get("status").and_then(Value::as_str), Some("ok"));
}

#[tokio::test]
async fn healthz_contract_dependency_unavailable_shape_compatible() {
    let mut health_targets = HashMap::new();
    health_targets.insert(
        "relay".to_string(),
        "http://127.0.0.1:1/healthz".to_string(),
    );
    let state = test_state_with_health_targets(health_targets).await;

    let app = Router::new()
        .route("/healthz", get(crate::healthz))
        .with_state(state);
    let (status, payload) = get_json(app, "/healthz").await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        payload.get("status").and_then(Value::as_str),
        Some("unavailable")
    );
}

#[tokio::test]
async fn metrics_contract_prometheus_content_type_shape_compatible() {
    let route = "/metrics-contract";
    cn_core::metrics::record_http_request(
        crate::SERVICE_NAME,
        "GET",
        route,
        200,
        std::time::Duration::from_millis(5),
    );

    let app = Router::new().route("/metrics", get(crate::metrics_endpoint));
    let (status, content_type, body) = get_text(app, "/metrics").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some("text/plain; version=0.0.4"));
    assert!(
        body.contains("cn_up{service=\"cn-admin-api\"} 1"),
        "metrics body did not contain cn_up for cn-admin-api: {body}"
    );
    assert_metric_line(
        &body,
        "http_requests_total",
        &[
            ("service", crate::SERVICE_NAME),
            ("route", route),
            ("method", "GET"),
            ("status", "200"),
        ],
    );
    assert_metric_line(
        &body,
        "http_request_duration_seconds_bucket",
        &[
            ("service", crate::SERVICE_NAME),
            ("route", route),
            ("method", "GET"),
            ("status", "200"),
        ],
    );
}

#[tokio::test]
async fn dashboard_contract_runbook_signals_shape_compatible() {
    let (relay_health_url, relay_server) = spawn_relay_metrics_mock(
        r#"
# HELP ingest_rejected_total Total ingest messages rejected
# TYPE ingest_rejected_total counter
ingest_rejected_total{service="cn-relay",reason="auth"} 5
ingest_rejected_total{service="cn-relay",reason="ratelimit"} 9
"#
        .to_string(),
    )
    .await;

    let mut health_targets = HashMap::new();
    health_targets.insert("relay".to_string(), relay_health_url);
    let state = test_state_with_health_targets(health_targets).await;
    let session_id = insert_admin_session(&state.pool).await;

    let event_id = format!("evt-{}", Uuid::new_v4());
    let topic_id = format!("kukuri:dashboard:{}", Uuid::new_v4());
    let consumer_name = format!("contract-dashboard-{}", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO cn_relay.events_outbox          (op, event_id, topic_id, kind, created_at, ingested_at, effective_key, reason)          VALUES ('upsert', $1, $2, 1, $3, NOW(), NULL, NULL)",
    )
    .bind(&event_id)
    .bind(&topic_id)
    .bind(chrono::Utc::now().timestamp())
    .execute(&state.pool)
    .await
    .expect("insert outbox row for dashboard");
    sqlx::query(
        "INSERT INTO cn_relay.consumer_offsets (consumer, last_seq) VALUES ($1, 0)          ON CONFLICT (consumer) DO UPDATE SET last_seq = EXCLUDED.last_seq, updated_at = NOW()",
    )
    .bind(&consumer_name)
    .execute(&state.pool)
    .await
    .expect("upsert consumer offset for dashboard");

    let app = Router::new()
        .route(
            "/v1/admin/dashboard",
            get(dashboard::get_dashboard_snapshot),
        )
        .with_state(state.clone());
    let (status, payload) = get_json_with_session(app, "/v1/admin/dashboard", &session_id).await;

    assert_eq!(status, StatusCode::OK);
    assert!(payload
        .get("collected_at")
        .and_then(Value::as_i64)
        .is_some());
    assert!(payload
        .pointer("/outbox_backlog/max_backlog")
        .and_then(Value::as_i64)
        .is_some());

    let consumers = payload
        .pointer("/outbox_backlog/consumers")
        .and_then(Value::as_array)
        .expect("outbox consumers");
    let consumer_row = consumers
        .iter()
        .find(|row| row.get("consumer").and_then(Value::as_str) == Some(consumer_name.as_str()))
        .expect("consumer backlog row");
    assert!(
        consumer_row
            .get("backlog")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            >= 1
    );

    assert_eq!(
        payload
            .pointer("/reject_surge/current_total")
            .and_then(Value::as_i64),
        Some(14)
    );
    assert_eq!(
        payload
            .pointer("/reject_surge/source_status")
            .and_then(Value::as_str),
        Some("ok")
    );
    assert!(payload
        .pointer("/db_pressure/db_size_bytes")
        .and_then(Value::as_i64)
        .is_some());
    assert!(payload
        .pointer("/db_pressure/connection_utilization")
        .and_then(Value::as_f64)
        .is_some());

    sqlx::query("DELETE FROM cn_relay.consumer_offsets WHERE consumer = $1")
        .bind(&consumer_name)
        .execute(&state.pool)
        .await
        .expect("cleanup consumer offset");
    sqlx::query("DELETE FROM cn_relay.events_outbox WHERE event_id = $1")
        .bind(&event_id)
        .execute(&state.pool)
        .await
        .expect("cleanup outbox row");

    relay_server.abort();
    let _ = relay_server.await;
}

#[tokio::test]
async fn auth_contract_login_me_logout_success() {
    let state = test_state().await;
    let password = "test-password";
    let (admin_user_id, username) = insert_admin_user(&state.pool, password).await;

    let app = Router::new()
        .route("/v1/admin/auth/login", post(auth::login))
        .route("/v1/admin/auth/me", get(auth::me))
        .route("/v1/admin/auth/logout", post(auth::logout))
        .with_state(state);

    let login_request = Request::builder()
        .method("POST")
        .uri("/v1/admin/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "username": username,
                "password": password
            })
            .to_string(),
        ))
        .expect("request");
    let login_response = app.clone().oneshot(login_request).await.expect("response");
    assert_eq!(login_response.status(), StatusCode::OK);
    let session_cookie = login_response
        .headers()
        .get("set-cookie")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .expect("set-cookie")
        .to_string();
    assert!(session_cookie.starts_with("cn_admin_session="));

    let login_body = to_bytes(login_response.into_body(), usize::MAX)
        .await
        .expect("response body");
    let login_payload: Value = serde_json::from_slice(&login_body).expect("json body");
    assert_eq!(
        login_payload.get("admin_user_id").and_then(Value::as_str),
        Some(admin_user_id.as_str())
    );
    assert_eq!(
        login_payload.get("username").and_then(Value::as_str),
        Some(username.as_str())
    );
    assert!(login_payload
        .get("expires_at")
        .and_then(Value::as_i64)
        .is_some());

    let me_request = Request::builder()
        .method("GET")
        .uri("/v1/admin/auth/me")
        .header("cookie", &session_cookie)
        .body(Body::empty())
        .expect("request");
    let me_response = app.clone().oneshot(me_request).await.expect("response");
    assert_eq!(me_response.status(), StatusCode::OK);
    let me_body = to_bytes(me_response.into_body(), usize::MAX)
        .await
        .expect("response body");
    let me_payload: Value = serde_json::from_slice(&me_body).expect("json body");
    assert_eq!(
        me_payload.get("admin_user_id").and_then(Value::as_str),
        Some(admin_user_id.as_str())
    );
    assert_eq!(
        me_payload.get("username").and_then(Value::as_str),
        Some(username.as_str())
    );

    let logout_request = Request::builder()
        .method("POST")
        .uri("/v1/admin/auth/logout")
        .header("cookie", &session_cookie)
        .body(Body::empty())
        .expect("request");
    let logout_response = app.clone().oneshot(logout_request).await.expect("response");
    assert_eq!(logout_response.status(), StatusCode::OK);
    let logout_body = to_bytes(logout_response.into_body(), usize::MAX)
        .await
        .expect("response body");
    let logout_payload: Value = serde_json::from_slice(&logout_body).expect("json body");
    assert_eq!(
        logout_payload.get("status").and_then(Value::as_str),
        Some("ok")
    );

    let me_after_logout_request = Request::builder()
        .method("GET")
        .uri("/v1/admin/auth/me")
        .header("cookie", &session_cookie)
        .body(Body::empty())
        .expect("request");
    let me_after_logout_response = app
        .clone()
        .oneshot(me_after_logout_request)
        .await
        .expect("response");
    assert_eq!(me_after_logout_response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn auth_contract_logout_returns_500_when_session_delete_fails() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    register_logout_failure(&state.pool, &session_id).await;

    let app = Router::new()
        .route("/v1/admin/auth/logout", post(auth::logout))
        .with_state(state.clone());

    let logout_request = Request::builder()
        .method("POST")
        .uri("/v1/admin/auth/logout")
        .header("cookie", format!("cn_admin_session={session_id}"))
        .body(Body::empty())
        .expect("request");
    let logout_response = app.clone().oneshot(logout_request).await.expect("response");
    assert_eq!(logout_response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let logout_body = to_bytes(logout_response.into_body(), usize::MAX)
        .await
        .expect("response body");
    let logout_payload: Value = serde_json::from_slice(&logout_body).expect("json body");
    assert_eq!(
        logout_payload.get("code").and_then(Value::as_str),
        Some("DB_ERROR")
    );

    let session_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM cn_admin.admin_sessions WHERE session_id = $1)",
    )
    .bind(&session_id)
    .fetch_one(&state.pool)
    .await
    .expect("session exists");
    assert!(session_exists);
}

#[tokio::test]
async fn admin_mutations_fail_when_audit_log_write_fails() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let app = Router::new()
        .route("/v1/admin/auth/login", post(auth::login))
        .route(
            "/v1/admin/services/{service}/config",
            put(services::update_service_config),
        )
        .route("/v1/admin/policies", post(policies::create_policy))
        .route(
            "/v1/admin/subscriptions/{subscriber_pubkey}",
            put(subscriptions::upsert_subscription),
        )
        .route(
            "/v1/admin/moderation/rules/test",
            post(moderation::test_rule),
        )
        .route(
            "/v1/admin/moderation/labels",
            post(moderation::create_label),
        )
        .route(
            "/v1/admin/moderation/labels/{label_id}/review",
            post(moderation::review_label),
        )
        .route(
            "/v1/admin/moderation/labels/{label_id}/rejudge",
            post(moderation::rejudge_label),
        )
        .route(
            "/v1/admin/trust/schedules/{job_type}",
            put(trust::update_schedule),
        )
        .route(
            "/v1/admin/access-control/invites",
            post(access_control::issue_invite),
        )
        .route(
            "/v1/admin/personal-data-jobs/{job_type}/{job_id}/retry",
            post(dsar::retry_job),
        )
        .route("/v1/reindex", post(reindex::enqueue_reindex))
        .with_state(state.clone());

    let (login_admin_user_id, login_username) =
        insert_admin_user(&state.pool, "audit-required").await;
    register_audit_failure(
        &state.pool,
        "admin.login",
        &format!("admin_user:{login_admin_user_id}"),
        None,
    )
    .await;
    let (status, payload) = post_json(
        app.clone(),
        "/v1/admin/auth/login",
        json!({
            "username": login_username,
            "password": "audit-required"
        }),
        &session_id,
    )
    .await;
    assert_audit_log_required(status, &payload);

    let service = format!("contract-service-{}", Uuid::new_v4().simple());
    register_audit_failure(
        &state.pool,
        "service_config.update",
        &format!("service:{service}"),
        None,
    )
    .await;
    let (status, payload) = put_json(
        app.clone(),
        &format!("/v1/admin/services/{service}/config"),
        json!({
            "config_json": {
                "enabled": true,
                "token": format!("token-{}", Uuid::new_v4().simple())
            }
        }),
        &session_id,
    )
    .await;
    assert_audit_log_required(status, &payload);

    let policy_type = format!("terms-{}", Uuid::new_v4().simple());
    let policy_version = "v-audit-fail";
    let policy_locale = "ja-JP";
    register_audit_failure(
        &state.pool,
        "policy.create",
        &format!("policy:{policy_type}:{policy_version}:{policy_locale}"),
        None,
    )
    .await;
    let (status, payload) = post_json(
        app.clone(),
        "/v1/admin/policies",
        json!({
            "policy_type": policy_type,
            "version": policy_version,
            "locale": policy_locale,
            "title": "監査ログ必須テスト",
            "content_md": "must fail when audit write fails"
        }),
        &session_id,
    )
    .await;
    assert_audit_log_required(status, &payload);

    let plan_id = format!("audit-plan-{}", Uuid::new_v4().simple());
    sqlx::query("INSERT INTO cn_user.plans (plan_id, name, is_active) VALUES ($1, $2, TRUE)")
        .bind(&plan_id)
        .bind("Audit Plan")
        .execute(&state.pool)
        .await
        .expect("insert plan");
    let subscriber_pubkey = Keys::generate().public_key().to_hex();
    register_audit_failure(
        &state.pool,
        "subscription.update",
        &format!("subscription:{subscriber_pubkey}"),
        None,
    )
    .await;
    let (status, payload) = put_json(
        app.clone(),
        &format!("/v1/admin/subscriptions/{subscriber_pubkey}"),
        json!({
            "plan_id": plan_id,
            "status": "active"
        }),
        &session_id,
    )
    .await;
    assert_audit_log_required(status, &payload);

    let sample_pubkey = Keys::generate().public_key().to_hex();
    let sample_event_id = format!("event-{}", Uuid::new_v4().simple());
    register_audit_failure(
        &state.pool,
        "moderation_rule.test",
        &format!("rule-test:{sample_event_id}"),
        None,
    )
    .await;
    let (status, payload) = post_json(
        app.clone(),
        "/v1/admin/moderation/rules/test",
        json!({
            "conditions": {
                "kinds": [1],
                "content_keywords": ["spam"]
            },
            "action": {
                "label": "spam",
                "confidence": 0.9,
                "exp_seconds": 3600,
                "policy_url": "https://example.com/policy",
                "policy_ref": "policy:spam:v1"
            },
            "sample": {
                "event_id": sample_event_id,
                "pubkey": sample_pubkey,
                "kind": 1,
                "content": "spam sample",
                "tags": []
            }
        }),
        &session_id,
    )
    .await;
    assert_audit_log_required(status, &payload);

    let review_target = format!("event:{}", Uuid::new_v4().simple());
    let review_topic_id = format!("kukuri:topic:review-audit-fail:{}", Uuid::new_v4());
    let (status, payload) = post_json(
        app.clone(),
        "/v1/admin/moderation/labels",
        json!({
            "target": review_target,
            "label": "manual-spam",
            "confidence": 0.8,
            "exp": chrono::Utc::now().timestamp() + 3600,
            "policy_url": "https://example.com/policy/manual",
            "policy_ref": "manual-v1",
            "topic_id": review_topic_id
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let review_label_id = payload
        .get("label_id")
        .and_then(Value::as_str)
        .expect("review label id")
        .to_string();

    register_audit_failure(
        &state.pool,
        "moderation_label.review",
        &format!("label:{review_label_id}"),
        None,
    )
    .await;
    let (status, payload) = post_json(
        app.clone(),
        &format!("/v1/admin/moderation/labels/{review_label_id}/review"),
        json!({
            "enabled": false,
            "reason": "false positive"
        }),
        &session_id,
    )
    .await;
    assert_audit_log_required(status, &payload);

    register_audit_failure(
        &state.pool,
        "moderation_label.rejudge",
        &format!("label:{review_label_id}"),
        None,
    )
    .await;
    let (status, payload) = post_json(
        app.clone(),
        &format!("/v1/admin/moderation/labels/{review_label_id}/rejudge"),
        json!({
            "reason": "manual override"
        }),
        &session_id,
    )
    .await;
    assert_audit_log_required(status, &payload);

    let trust_interval = 7777_i64;
    register_audit_failure(
        &state.pool,
        "trust.schedule.update",
        "trust:schedule:report_based",
        Some(json!({
            "interval_seconds": trust_interval,
            "is_enabled": true
        })),
    )
    .await;
    let (status, payload) = put_json(
        app.clone(),
        "/v1/admin/trust/schedules/report_based",
        json!({
            "interval_seconds": trust_interval,
            "is_enabled": true
        }),
        &session_id,
    )
    .await;
    assert_audit_log_required(status, &payload);

    let invite_nonce = format!("invite-{}", Uuid::new_v4().simple());
    register_audit_failure(
        &state.pool,
        "access_control.invite.issue",
        &format!("invite:{invite_nonce}"),
        None,
    )
    .await;
    let (status, payload) = post_json(
        app.clone(),
        "/v1/admin/access-control/invites",
        json!({
            "topic_id": format!("kukuri:topic:audit-fail:{}", Uuid::new_v4()),
            "scope": "invite",
            "expires_in_seconds": 3600,
            "max_uses": 1,
            "nonce": invite_nonce
        }),
        &session_id,
    )
    .await;
    assert_audit_log_required(status, &payload);

    let export_request_id = Uuid::new_v4().to_string();
    let exporter_pubkey = Keys::generate().public_key().to_hex();
    insert_export_request(
        &state.pool,
        &export_request_id,
        &exporter_pubkey,
        "failed",
        Some("initial failure"),
    )
    .await;
    register_audit_failure(
        &state.pool,
        "dsar.job.retry",
        &format!("dsar:export:{export_request_id}"),
        None,
    )
    .await;
    let (status, payload) = post_json(
        app.clone(),
        &format!("/v1/admin/personal-data-jobs/export/{export_request_id}/retry"),
        json!({}),
        &session_id,
    )
    .await;
    assert_audit_log_required(status, &payload);

    let reindex_topic_id = format!("kukuri:topic:reindex-audit-fail:{}", Uuid::new_v4());
    register_audit_failure(
        &state.pool,
        "index.reindex.request",
        "index:reindex",
        Some(json!({ "topic_id": reindex_topic_id })),
    )
    .await;
    let (status, payload) = post_json(
        app,
        "/v1/reindex",
        json!({ "topic_id": reindex_topic_id }),
        &session_id,
    )
    .await;
    assert_audit_log_required(status, &payload);
}

#[tokio::test]
async fn transactional_admin_mutations_rollback_when_audit_log_write_fails() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let app = Router::new()
        .route(
            "/v1/admin/services/{service}/config",
            put(services::update_service_config),
        )
        .route(
            "/v1/admin/policies/{policy_id}/make-current",
            post(policies::make_current_policy),
        )
        .route(
            "/v1/admin/subscription-requests/{request_id}/approve",
            post(subscriptions::approve_subscription_request),
        )
        .route("/v1/admin/plans", post(subscriptions::create_plan))
        .route("/v1/admin/plans/{plan_id}", put(subscriptions::update_plan))
        .with_state(state.clone());

    let service = format!("rollback-audit-service-{}", Uuid::new_v4().simple());
    register_audit_failure(
        &state.pool,
        "service_config.update",
        &format!("service:{service}"),
        None,
    )
    .await;
    let (status, payload) = put_json(
        app.clone(),
        &format!("/v1/admin/services/{service}/config"),
        json!({
            "config_json": {
                "enabled": true
            }
        }),
        &session_id,
    )
    .await;
    assert_audit_log_required(status, &payload);
    let service_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_admin.service_configs WHERE service = $1",
    )
    .bind(&service)
    .fetch_one(&state.pool)
    .await
    .expect("count service config after failed audit");
    assert_eq!(service_count, 0);

    let policy_type = format!("rollback-audit-policy-{}", Uuid::new_v4().simple());
    let policy_locale = "ja-JP";
    let current_policy_id = format!("{policy_type}:v1:{policy_locale}");
    let target_policy_id = format!("{policy_type}:v2:{policy_locale}");
    sqlx::query(
        "INSERT INTO cn_admin.policies          (policy_id, type, version, locale, title, content_md, content_hash, is_current)          VALUES ($1, $2, 'v1', $3, 'current', 'current', 'hash-current', TRUE),                 ($4, $2, 'v2', $3, 'target', 'target', 'hash-target', FALSE)",
    )
    .bind(&current_policy_id)
    .bind(&policy_type)
    .bind(policy_locale)
    .bind(&target_policy_id)
    .execute(&state.pool)
    .await
    .expect("insert policies for audit rollback test");
    register_audit_failure(
        &state.pool,
        "policy.make_current",
        &format!("policy:{target_policy_id}"),
        None,
    )
    .await;
    let (status, payload) = post_json(
        app.clone(),
        &format!("/v1/admin/policies/{target_policy_id}/make-current"),
        json!({}),
        &session_id,
    )
    .await;
    assert_audit_log_required(status, &payload);
    let current_is_current = sqlx::query_scalar::<_, bool>(
        "SELECT is_current FROM cn_admin.policies WHERE policy_id = $1",
    )
    .bind(&current_policy_id)
    .fetch_one(&state.pool)
    .await
    .expect("fetch current policy after failed audit");
    let target_is_current = sqlx::query_scalar::<_, bool>(
        "SELECT is_current FROM cn_admin.policies WHERE policy_id = $1",
    )
    .bind(&target_policy_id)
    .fetch_one(&state.pool)
    .await
    .expect("fetch target policy after failed audit");
    assert!(current_is_current);
    assert!(!target_is_current);

    let requester_pubkey = Keys::generate().public_key().to_hex();
    let approve_topic_id = format!("kukuri:topic:rollback-audit:{}", Uuid::new_v4());
    let request_id = insert_subscription_request(
        &state.pool,
        &requester_pubkey,
        &approve_topic_id,
        json!({ "search": true }),
    )
    .await;
    register_audit_failure(
        &state.pool,
        "subscription_request.approve",
        &format!("subscription_request:{request_id}"),
        None,
    )
    .await;
    let (status, payload) = post_json(
        app.clone(),
        &format!("/v1/admin/subscription-requests/{request_id}/approve"),
        json!({ "review_note": "approve should rollback on audit failure" }),
        &session_id,
    )
    .await;
    assert_audit_log_required(status, &payload);
    let request_status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM cn_user.topic_subscription_requests WHERE request_id = $1",
    )
    .bind(&request_id)
    .fetch_one(&state.pool)
    .await
    .expect("fetch request status after failed audit");
    assert_eq!(request_status, "pending");
    let topic_subscription_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_user.topic_subscriptions WHERE topic_id = $1 AND subscriber_pubkey = $2",
    )
    .bind(&approve_topic_id)
    .bind(&requester_pubkey)
    .fetch_one(&state.pool)
    .await
    .expect("count topic subscriptions after failed audit");
    assert_eq!(topic_subscription_count, 0);
    let node_subscription_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_admin.node_subscriptions WHERE topic_id = $1",
    )
    .bind(&approve_topic_id)
    .fetch_one(&state.pool)
    .await
    .expect("count node subscriptions after failed audit");
    assert_eq!(node_subscription_count, 0);

    let create_plan_id = format!("rollback-audit-plan-create-{}", Uuid::new_v4().simple());
    register_audit_failure(
        &state.pool,
        "plan.create",
        &format!("plan:{create_plan_id}"),
        None,
    )
    .await;
    let (status, payload) = post_json(
        app.clone(),
        "/v1/admin/plans",
        json!({
            "plan_id": create_plan_id.clone(),
            "name": "Rollback audit create",
            "is_active": true,
            "limits": [
                {
                    "metric": "search_query",
                    "window": "day",
                    "limit": 120
                }
            ]
        }),
        &session_id,
    )
    .await;
    assert_audit_log_required(status, &payload);
    let created_plan_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM cn_user.plans WHERE plan_id = $1")
            .bind(&create_plan_id)
            .fetch_one(&state.pool)
            .await
            .expect("count created plan after failed audit");
    assert_eq!(created_plan_count, 0);
    let created_plan_limit_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM cn_user.plan_limits WHERE plan_id = $1")
            .bind(&create_plan_id)
            .fetch_one(&state.pool)
            .await
            .expect("count created plan limits after failed audit");
    assert_eq!(created_plan_limit_count, 0);

    let update_plan_id = format!("rollback-audit-plan-update-{}", Uuid::new_v4().simple());
    sqlx::query("INSERT INTO cn_user.plans (plan_id, name, is_active) VALUES ($1, $2, TRUE)")
        .bind(&update_plan_id)
        .bind("Before rollback audit update")
        .execute(&state.pool)
        .await
        .expect("insert plan for update rollback test");
    sqlx::query(
        "INSERT INTO cn_user.plan_limits (plan_id, metric, \"window\", \"limit\") VALUES ($1, 'search_query', 'day', 40)",
    )
    .bind(&update_plan_id)
    .execute(&state.pool)
    .await
    .expect("insert plan limit for update rollback test");
    register_audit_failure(
        &state.pool,
        "plan.update",
        &format!("plan:{update_plan_id}"),
        None,
    )
    .await;
    let (status, payload) = put_json(
        app,
        &format!("/v1/admin/plans/{update_plan_id}"),
        json!({
            "plan_id": update_plan_id.clone(),
            "name": "After rollback audit update",
            "is_active": false,
            "limits": [
                {
                    "metric": "search_query",
                    "window": "day",
                    "limit": 10
                },
                {
                    "metric": "report_submit",
                    "window": "day",
                    "limit": 1
                }
            ]
        }),
        &session_id,
    )
    .await;
    assert_audit_log_required(status, &payload);
    let row = sqlx::query("SELECT name, is_active FROM cn_user.plans WHERE plan_id = $1")
        .bind(&update_plan_id)
        .fetch_one(&state.pool)
        .await
        .expect("fetch plan after failed audit update");
    assert_eq!(
        row.try_get::<String, _>("name")
            .expect("plan name after failed audit"),
        "Before rollback audit update"
    );
    assert!(row
        .try_get::<bool, _>("is_active")
        .expect("plan is_active after failed audit"));
    let limit_rows = sqlx::query(
        "SELECT metric, \"window\", \"limit\" FROM cn_user.plan_limits WHERE plan_id = $1 ORDER BY metric",
    )
    .bind(&update_plan_id)
    .fetch_all(&state.pool)
    .await
    .expect("fetch plan limits after failed audit update");
    assert_eq!(limit_rows.len(), 1);
    assert_eq!(
        limit_rows[0]
            .try_get::<String, _>("metric")
            .expect("metric"),
        "search_query"
    );
    assert_eq!(
        limit_rows[0]
            .try_get::<String, _>("window")
            .expect("window"),
        "day"
    );
    assert_eq!(limit_rows[0].try_get::<i64, _>("limit").expect("limit"), 40);
}

#[tokio::test]
async fn transactional_admin_mutations_rollback_when_commit_fails() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let app = Router::new()
        .route(
            "/v1/admin/services/{service}/config",
            put(services::update_service_config),
        )
        .route(
            "/v1/admin/policies/{policy_id}/make-current",
            post(policies::make_current_policy),
        )
        .route(
            "/v1/admin/subscription-requests/{request_id}/approve",
            post(subscriptions::approve_subscription_request),
        )
        .route("/v1/admin/plans", post(subscriptions::create_plan))
        .route("/v1/admin/plans/{plan_id}", put(subscriptions::update_plan))
        .with_state(state.clone());

    let service = format!("rollback-commit-service-{}", Uuid::new_v4().simple());
    let service_target = format!("service:{service}");
    register_commit_failure(&state.pool, "service_config.update", &service_target, None).await;
    let (status, payload) = put_json(
        app.clone(),
        &format!("/v1/admin/services/{service}/config"),
        json!({
            "config_json": {
                "enabled": true
            }
        }),
        &session_id,
    )
    .await;
    assert_db_error(status, &payload);
    let service_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_admin.service_configs WHERE service = $1",
    )
    .bind(&service)
    .fetch_one(&state.pool)
    .await
    .expect("count service config after commit failure");
    assert_eq!(service_count, 0);
    let service_audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_admin.audit_logs WHERE action = $1 AND target = $2",
    )
    .bind("service_config.update")
    .bind(&service_target)
    .fetch_one(&state.pool)
    .await
    .expect("count service audit logs after commit failure");
    assert_eq!(service_audit_count, 0);

    let policy_type = format!("rollback-commit-policy-{}", Uuid::new_v4().simple());
    let policy_locale = "ja-JP";
    let current_policy_id = format!("{policy_type}:v1:{policy_locale}");
    let target_policy_id = format!("{policy_type}:v2:{policy_locale}");
    sqlx::query(
        "INSERT INTO cn_admin.policies          (policy_id, type, version, locale, title, content_md, content_hash, is_current)          VALUES ($1, $2, 'v1', $3, 'current', 'current', 'hash-current', TRUE),                 ($4, $2, 'v2', $3, 'target', 'target', 'hash-target', FALSE)",
    )
    .bind(&current_policy_id)
    .bind(&policy_type)
    .bind(policy_locale)
    .bind(&target_policy_id)
    .execute(&state.pool)
    .await
    .expect("insert policies for commit rollback test");
    let make_current_target = format!("policy:{target_policy_id}");
    register_commit_failure(
        &state.pool,
        "policy.make_current",
        &make_current_target,
        None,
    )
    .await;
    let (status, payload) = post_json(
        app.clone(),
        &format!("/v1/admin/policies/{target_policy_id}/make-current"),
        json!({}),
        &session_id,
    )
    .await;
    assert_db_error(status, &payload);
    let current_is_current = sqlx::query_scalar::<_, bool>(
        "SELECT is_current FROM cn_admin.policies WHERE policy_id = $1",
    )
    .bind(&current_policy_id)
    .fetch_one(&state.pool)
    .await
    .expect("fetch current policy after commit failure");
    let target_is_current = sqlx::query_scalar::<_, bool>(
        "SELECT is_current FROM cn_admin.policies WHERE policy_id = $1",
    )
    .bind(&target_policy_id)
    .fetch_one(&state.pool)
    .await
    .expect("fetch target policy after commit failure");
    assert!(current_is_current);
    assert!(!target_is_current);
    let policy_audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_admin.audit_logs WHERE action = $1 AND target = $2",
    )
    .bind("policy.make_current")
    .bind(&make_current_target)
    .fetch_one(&state.pool)
    .await
    .expect("count make_current audit logs after commit failure");
    assert_eq!(policy_audit_count, 0);

    let requester_pubkey = Keys::generate().public_key().to_hex();
    let approve_topic_id = format!("kukuri:topic:rollback-commit:{}", Uuid::new_v4());
    let request_id = insert_subscription_request(
        &state.pool,
        &requester_pubkey,
        &approve_topic_id,
        json!({ "search": true }),
    )
    .await;
    let approve_target = format!("subscription_request:{request_id}");
    register_commit_failure(
        &state.pool,
        "subscription_request.approve",
        &approve_target,
        None,
    )
    .await;
    let (status, payload) = post_json(
        app.clone(),
        &format!("/v1/admin/subscription-requests/{request_id}/approve"),
        json!({ "review_note": "approve should rollback on commit failure" }),
        &session_id,
    )
    .await;
    assert_db_error(status, &payload);
    let request_status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM cn_user.topic_subscription_requests WHERE request_id = $1",
    )
    .bind(&request_id)
    .fetch_one(&state.pool)
    .await
    .expect("fetch request status after commit failure");
    assert_eq!(request_status, "pending");
    let topic_subscription_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_user.topic_subscriptions WHERE topic_id = $1 AND subscriber_pubkey = $2",
    )
    .bind(&approve_topic_id)
    .bind(&requester_pubkey)
    .fetch_one(&state.pool)
    .await
    .expect("count topic subscriptions after commit failure");
    assert_eq!(topic_subscription_count, 0);
    let node_subscription_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_admin.node_subscriptions WHERE topic_id = $1",
    )
    .bind(&approve_topic_id)
    .fetch_one(&state.pool)
    .await
    .expect("count node subscriptions after commit failure");
    assert_eq!(node_subscription_count, 0);
    let approve_audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_admin.audit_logs WHERE action = $1 AND target = $2",
    )
    .bind("subscription_request.approve")
    .bind(&approve_target)
    .fetch_one(&state.pool)
    .await
    .expect("count approve audit logs after commit failure");
    assert_eq!(approve_audit_count, 0);

    let create_plan_id = format!("rollback-commit-plan-create-{}", Uuid::new_v4().simple());
    let create_plan_target = format!("plan:{create_plan_id}");
    register_commit_failure(&state.pool, "plan.create", &create_plan_target, None).await;
    let (status, payload) = post_json(
        app.clone(),
        "/v1/admin/plans",
        json!({
            "plan_id": create_plan_id.clone(),
            "name": "Rollback commit create",
            "is_active": true,
            "limits": [
                {
                    "metric": "search_query",
                    "window": "day",
                    "limit": 120
                }
            ]
        }),
        &session_id,
    )
    .await;
    assert_db_error(status, &payload);
    let created_plan_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM cn_user.plans WHERE plan_id = $1")
            .bind(&create_plan_id)
            .fetch_one(&state.pool)
            .await
            .expect("count created plan after commit failure");
    assert_eq!(created_plan_count, 0);
    let created_plan_limit_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM cn_user.plan_limits WHERE plan_id = $1")
            .bind(&create_plan_id)
            .fetch_one(&state.pool)
            .await
            .expect("count created plan limits after commit failure");
    assert_eq!(created_plan_limit_count, 0);
    let create_plan_audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_admin.audit_logs WHERE action = $1 AND target = $2",
    )
    .bind("plan.create")
    .bind(&create_plan_target)
    .fetch_one(&state.pool)
    .await
    .expect("count create plan audit logs after commit failure");
    assert_eq!(create_plan_audit_count, 0);

    let update_plan_id = format!("rollback-commit-plan-update-{}", Uuid::new_v4().simple());
    sqlx::query("INSERT INTO cn_user.plans (plan_id, name, is_active) VALUES ($1, $2, TRUE)")
        .bind(&update_plan_id)
        .bind("Before rollback commit update")
        .execute(&state.pool)
        .await
        .expect("insert plan for commit failure update test");
    sqlx::query(
        "INSERT INTO cn_user.plan_limits (plan_id, metric, \"window\", \"limit\") VALUES ($1, 'search_query', 'day', 40)",
    )
    .bind(&update_plan_id)
    .execute(&state.pool)
    .await
    .expect("insert plan limit for commit failure update test");
    let update_plan_target = format!("plan:{update_plan_id}");
    register_commit_failure(&state.pool, "plan.update", &update_plan_target, None).await;
    let (status, payload) = put_json(
        app,
        &format!("/v1/admin/plans/{update_plan_id}"),
        json!({
            "plan_id": update_plan_id.clone(),
            "name": "After rollback commit update",
            "is_active": false,
            "limits": [
                {
                    "metric": "search_query",
                    "window": "day",
                    "limit": 10
                },
                {
                    "metric": "report_submit",
                    "window": "day",
                    "limit": 1
                }
            ]
        }),
        &session_id,
    )
    .await;
    assert_db_error(status, &payload);
    let row = sqlx::query("SELECT name, is_active FROM cn_user.plans WHERE plan_id = $1")
        .bind(&update_plan_id)
        .fetch_one(&state.pool)
        .await
        .expect("fetch plan after commit failure update");
    assert_eq!(
        row.try_get::<String, _>("name")
            .expect("plan name after commit failure"),
        "Before rollback commit update"
    );
    assert!(row
        .try_get::<bool, _>("is_active")
        .expect("plan is_active after commit failure"));
    let limit_rows = sqlx::query(
        "SELECT metric, \"window\", \"limit\" FROM cn_user.plan_limits WHERE plan_id = $1 ORDER BY metric",
    )
    .bind(&update_plan_id)
    .fetch_all(&state.pool)
    .await
    .expect("fetch plan limits after commit failure update");
    assert_eq!(limit_rows.len(), 1);
    assert_eq!(
        limit_rows[0]
            .try_get::<String, _>("metric")
            .expect("metric"),
        "search_query"
    );
    assert_eq!(
        limit_rows[0]
            .try_get::<String, _>("window")
            .expect("window"),
        "day"
    );
    assert_eq!(limit_rows[0].try_get::<i64, _>("limit").expect("limit"), 40);
    let update_plan_audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_admin.audit_logs WHERE action = $1 AND target = $2",
    )
    .bind("plan.update")
    .bind(&update_plan_target)
    .fetch_one(&state.pool)
    .await
    .expect("count update plan audit logs after commit failure");
    assert_eq!(update_plan_audit_count, 0);
}

#[tokio::test]
async fn services_contract_success_and_shape() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let service = format!("service-{}", Uuid::new_v4());
    let service_config = json!({
        "enabled": true,
        "refresh_seconds": 30
    });

    let app = Router::new()
        .route("/v1/admin/services", get(services::list_services))
        .route(
            "/v1/admin/services/{service}/config",
            get(services::get_service_config).put(services::update_service_config),
        )
        .with_state(state.clone());

    let (status, payload) = put_json(
        app.clone(),
        &format!("/v1/admin/services/{service}/config"),
        json!({
            "config_json": service_config,
            "expected_version": null
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("service").and_then(Value::as_str),
        Some(service.as_str())
    );
    assert_eq!(payload.get("version").and_then(Value::as_i64), Some(1));
    assert_eq!(payload.get("config_json"), Some(&service_config));
    assert!(payload.get("updated_at").and_then(Value::as_i64).is_some());
    assert!(payload.get("updated_by").and_then(Value::as_str).is_some());

    insert_service_health(
        &state.pool,
        &service,
        "healthy",
        json!({ "status": 200, "source": "contract-test" }),
    )
    .await;

    let (status, payload) = get_json_with_session(
        app.clone(),
        &format!("/v1/admin/services/{service}/config"),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("service").and_then(Value::as_str),
        Some(service.as_str())
    );
    assert_eq!(payload.get("version").and_then(Value::as_i64), Some(1));
    assert_eq!(payload.get("config_json"), Some(&service_config));

    let (status, payload) = get_json_with_session(app, "/v1/admin/services", &session_id).await;
    assert_eq!(status, StatusCode::OK);
    let rows = payload.as_array().expect("array payload");
    let service_row = rows
        .iter()
        .find(|row| row.get("service").and_then(Value::as_str) == Some(service.as_str()))
        .expect("service row");
    assert_eq!(service_row.get("version").and_then(Value::as_i64), Some(1));
    assert_eq!(service_row.get("config_json"), Some(&service_config));
    let health = service_row
        .get("health")
        .and_then(Value::as_object)
        .expect("health object");
    assert_eq!(
        health.get("status").and_then(Value::as_str),
        Some("healthy")
    );
    assert!(health.get("checked_at").and_then(Value::as_i64).is_some());
}

#[tokio::test]
async fn services_update_contract_rejects_secret_keys_and_preserves_storage() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let service = format!("secret-reject-{}", Uuid::new_v4().simple());
    let target = format!("service:{service}");

    let app = Router::new()
        .route(
            "/v1/admin/services/{service}/config",
            put(services::update_service_config),
        )
        .with_state(state.clone());

    let (status, payload) = put_json(
        app,
        &format!("/v1/admin/services/{service}/config"),
        json!({
            "config_json": {
                "llm": {
                    "provider": "openai",
                    "OPENAI_API_KEY": "sk-test"
                }
            }
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        payload.get("code").and_then(Value::as_str),
        Some("SECRET_CONFIG_FORBIDDEN")
    );
    let message = payload
        .get("message")
        .and_then(Value::as_str)
        .expect("error message");
    assert!(
        message.contains("/llm/OPENAI_API_KEY"),
        "unexpected reject message: {message}"
    );

    let service_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_admin.service_configs WHERE service = $1",
    )
    .bind(&service)
    .fetch_one(&state.pool)
    .await
    .expect("count service config after secret reject");
    assert_eq!(service_count, 0);

    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_admin.audit_logs WHERE action = $1 AND target = $2",
    )
    .bind("service_config.update")
    .bind(target)
    .fetch_one(&state.pool)
    .await
    .expect("count audit logs after secret reject");
    assert_eq!(audit_count, 0);
}

#[tokio::test]
async fn services_health_poll_contract_status_matrix_backward_compatible() {
    let healthy_status = Arc::new(AtomicU16::new(200));
    let degraded_status = Arc::new(AtomicU16::new(503));
    let (healthy_url, healthy_server) = spawn_healthz_mock(Arc::clone(&healthy_status)).await;
    let (degraded_url, degraded_server) = spawn_healthz_mock(Arc::clone(&degraded_status)).await;

    let healthy_service = format!("healthy-{}", Uuid::new_v4());
    let degraded_service = format!("degraded-{}", Uuid::new_v4());
    let unreachable_service = format!("unreachable-{}", Uuid::new_v4());
    let mut health_targets = HashMap::new();
    health_targets.insert(healthy_service.clone(), healthy_url);
    health_targets.insert(degraded_service.clone(), degraded_url);
    health_targets.insert(
        unreachable_service.clone(),
        "http://127.0.0.1:1/healthz".to_string(),
    );

    let state = test_state_with_health_targets(health_targets).await;
    let session_id = insert_admin_session(&state.pool).await;

    upsert_service_config(&state.pool, &healthy_service, json!({ "enabled": true })).await;
    upsert_service_config(&state.pool, &degraded_service, json!({ "enabled": true })).await;
    upsert_service_config(
        &state.pool,
        &unreachable_service,
        json!({ "enabled": true }),
    )
    .await;

    services::poll_health_once(&state).await;

    let app = Router::new()
        .route("/v1/admin/services", get(services::list_services))
        .with_state(state);
    let (status, payload) = get_json_with_session(app, "/v1/admin/services", &session_id).await;
    assert_eq!(status, StatusCode::OK);

    let rows = payload.as_array().expect("array payload");
    let healthy = rows
        .iter()
        .find(|row| row.get("service").and_then(Value::as_str) == Some(healthy_service.as_str()))
        .expect("healthy service row");
    let healthy_health = healthy
        .get("health")
        .and_then(Value::as_object)
        .expect("healthy health");
    assert_eq!(
        healthy_health.get("status").and_then(Value::as_str),
        Some("healthy")
    );
    assert_eq!(
        healthy_health
            .get("details")
            .and_then(|details| details.get("status"))
            .and_then(Value::as_u64),
        Some(200)
    );

    let degraded = rows
        .iter()
        .find(|row| row.get("service").and_then(Value::as_str) == Some(degraded_service.as_str()))
        .expect("degraded service row");
    let degraded_health = degraded
        .get("health")
        .and_then(Value::as_object)
        .expect("degraded health");
    assert_eq!(
        degraded_health.get("status").and_then(Value::as_str),
        Some("degraded")
    );
    assert_eq!(
        degraded_health
            .get("details")
            .and_then(|details| details.get("status"))
            .and_then(Value::as_u64),
        Some(503)
    );

    let unreachable = rows
        .iter()
        .find(|row| {
            row.get("service").and_then(Value::as_str) == Some(unreachable_service.as_str())
        })
        .expect("unreachable service row");
    let unreachable_health = unreachable
        .get("health")
        .and_then(Value::as_object)
        .expect("unreachable health");
    assert_eq!(
        unreachable_health.get("status").and_then(Value::as_str),
        Some("unreachable")
    );
    assert!(unreachable_health
        .get("details")
        .and_then(|details| details.get("error"))
        .and_then(Value::as_str)
        .is_some());

    healthy_server.abort();
    let _ = healthy_server.await;
    degraded_server.abort();
    let _ = degraded_server.await;
}

#[tokio::test]
async fn services_health_poll_updates_details_json_on_status_change() {
    let health_status = Arc::new(AtomicU16::new(200));
    let (health_url, health_server) = spawn_healthz_mock(Arc::clone(&health_status)).await;

    let service = format!("poll-update-{}", Uuid::new_v4());
    let mut health_targets = HashMap::new();
    health_targets.insert(service.clone(), health_url);
    let state = test_state_with_health_targets(health_targets).await;

    services::poll_health_once(&state).await;
    let (status, details) = fetch_service_health_row(&state.pool, &service).await;
    assert_eq!(status, "healthy");
    assert_eq!(details.get("status").and_then(Value::as_u64), Some(200));

    health_status.store(503, Ordering::Relaxed);
    services::poll_health_once(&state).await;
    let (status, details) = fetch_service_health_row(&state.pool, &service).await;
    assert_eq!(status, "degraded");
    assert_eq!(details.get("status").and_then(Value::as_u64), Some(503));

    health_server.abort();
    let _ = health_server.await;
    services::poll_health_once(&state).await;
    let (status, details) = fetch_service_health_row(&state.pool, &service).await;
    assert_eq!(status, "unreachable");
    assert!(details.get("status").is_none());
    assert!(details.get("error").and_then(Value::as_str).is_some());
}

#[tokio::test]
async fn services_health_poll_collects_relay_auth_transition_metrics() {
    let metrics_body = r#"
# HELP ws_connections Active websocket connections
# TYPE ws_connections gauge
ws_connections{service="cn-relay"} 4
# HELP ws_unauthenticated_connections Active websocket connections without successful AUTH
# TYPE ws_unauthenticated_connections gauge
ws_unauthenticated_connections{service="cn-relay"} 2
# HELP ingest_rejected_total Total ingest messages rejected
# TYPE ingest_rejected_total counter
ingest_rejected_total{service="cn-relay",reason="auth"} 8
ingest_rejected_total{service="cn-relay",reason="ratelimit"} 3
# HELP ws_auth_disconnect_total Total websocket disconnects caused by auth transition enforcement
# TYPE ws_auth_disconnect_total counter
ws_auth_disconnect_total{service="cn-relay",reason="timeout"} 5
ws_auth_disconnect_total{service="cn-relay",reason="deadline"} 1
"#
    .to_string();
    let (relay_url, relay_server) = spawn_relay_metrics_mock(metrics_body).await;

    let mut health_targets = HashMap::new();
    health_targets.insert("relay".to_string(), relay_url);
    let state = test_state_with_health_targets(health_targets).await;

    services::poll_health_once(&state).await;
    let (status, details) = fetch_service_health_row(&state.pool, "relay").await;
    assert_eq!(status, "healthy");
    assert_eq!(details.get("status").and_then(Value::as_u64), Some(200));
    assert_eq!(
        details
            .pointer("/auth_transition/metrics_status")
            .and_then(Value::as_u64),
        Some(200)
    );
    assert_eq!(
        details
            .pointer("/auth_transition/ws_connections")
            .and_then(Value::as_i64),
        Some(4)
    );
    assert_eq!(
        details
            .pointer("/auth_transition/ws_unauthenticated_connections")
            .and_then(Value::as_i64),
        Some(2)
    );
    assert_eq!(
        details
            .pointer("/auth_transition/ingest_rejected_auth_total")
            .and_then(Value::as_i64),
        Some(8)
    );
    assert_eq!(
        details
            .pointer("/auth_transition/ws_auth_disconnect_timeout_total")
            .and_then(Value::as_i64),
        Some(5)
    );
    assert_eq!(
        details
            .pointer("/auth_transition/ws_auth_disconnect_deadline_total")
            .and_then(Value::as_i64),
        Some(1)
    );

    relay_server.abort();
    let _ = relay_server.await;
}

#[tokio::test]
async fn policies_contract_lifecycle_success() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let policy_type = format!("terms-{}", &Uuid::new_v4().to_string()[..8]);
    let locale = "ja-JP";
    let version = "v1";
    let policy_id = format!("{policy_type}:{version}:{locale}");
    let effective_at = chrono::Utc::now().timestamp() + 3600;

    let app = Router::new()
        .route(
            "/v1/admin/policies",
            get(policies::list_policies).post(policies::create_policy),
        )
        .route(
            "/v1/admin/policies/{policy_id}",
            put(policies::update_policy),
        )
        .route(
            "/v1/admin/policies/{policy_id}/publish",
            post(policies::publish_policy),
        )
        .route(
            "/v1/admin/policies/{policy_id}/make-current",
            post(policies::make_current_policy),
        )
        .with_state(state);

    let (status, payload) = post_json(
        app.clone(),
        "/v1/admin/policies",
        json!({
            "policy_type": policy_type,
            "version": version,
            "locale": locale,
            "title": "利用規約",
            "content_md": "初版"
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("policy_id").and_then(Value::as_str),
        Some(policy_id.as_str())
    );
    assert_eq!(payload.get("published_at").and_then(Value::as_i64), None);
    assert_eq!(payload.get("effective_at").and_then(Value::as_i64), None);
    assert_eq!(
        payload.get("is_current").and_then(Value::as_bool),
        Some(false)
    );
    let created_hash = payload
        .get("content_hash")
        .and_then(Value::as_str)
        .expect("content hash")
        .to_string();

    let (status, payload) = put_json(
        app.clone(),
        &format!("/v1/admin/policies/{policy_id}"),
        json!({
            "title": "利用規約 改訂版",
            "content_md": "更新後"
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("title").and_then(Value::as_str),
        Some("利用規約 改訂版")
    );
    let updated_hash = payload
        .get("content_hash")
        .and_then(Value::as_str)
        .expect("updated content hash");
    assert_ne!(updated_hash, created_hash);

    let (status, payload) = post_json(
        app.clone(),
        &format!("/v1/admin/policies/{policy_id}/publish"),
        json!({
            "effective_at": effective_at
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(payload
        .get("published_at")
        .and_then(Value::as_i64)
        .is_some());
    assert_eq!(
        payload.get("effective_at").and_then(Value::as_i64),
        Some(effective_at)
    );

    let (status, payload) = post_json(
        app.clone(),
        &format!("/v1/admin/policies/{policy_id}/make-current"),
        json!({}),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("is_current").and_then(Value::as_bool),
        Some(true)
    );

    let (status, payload) = get_json_with_session(
        app,
        &format!("/v1/admin/policies?policy_type={policy_type}&locale={locale}"),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rows = payload.as_array().expect("array payload");
    let row = rows
        .iter()
        .find(|row| row.get("policy_id").and_then(Value::as_str) == Some(policy_id.as_str()))
        .expect("policy row");
    assert_eq!(
        row.get("policy_type").and_then(Value::as_str),
        Some(policy_type.as_str())
    );
    assert_eq!(row.get("version").and_then(Value::as_str), Some(version));
    assert_eq!(row.get("locale").and_then(Value::as_str), Some(locale));
    assert_eq!(row.get("is_current").and_then(Value::as_bool), Some(true));
}

#[tokio::test]
async fn legacy_admin_path_aliases_contract_success() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let policy_type = format!("terms-legacy-{}", &Uuid::new_v4().to_string()[..8]);
    let locale = "ja-JP";
    let version = "v1";
    let policy_id = format!("{policy_type}:{version}:{locale}");
    let target = format!("event:{}", Uuid::new_v4().simple());
    let subject_pubkey = Keys::generate().public_key().to_hex();
    let effective_at = chrono::Utc::now().timestamp() + 1800;

    let app = Router::new()
        .route(
            "/v1/admin/policies",
            get(policies::list_policies).post(policies::create_policy),
        )
        .route(
            "/v1/admin/policies/{policy_id}",
            put(policies::update_policy),
        )
        .route(
            "/v1/admin/policies/{policy_id}/publish",
            post(policies::publish_policy),
        )
        .route(
            "/v1/admin/policies/{policy_id}/make-current",
            post(policies::make_current_policy),
        )
        .route(
            "/v1/policies",
            get(policies::list_policies).post(policies::create_policy),
        )
        .route("/v1/policies/{policy_id}", put(policies::update_policy))
        .route(
            "/v1/policies/{policy_id}/publish",
            post(policies::publish_policy),
        )
        .route(
            "/v1/policies/{policy_id}/make-current",
            post(policies::make_current_policy),
        )
        .route(
            "/v1/admin/moderation/labels",
            get(moderation::list_labels).post(moderation::create_label),
        )
        .route("/v1/labels", post(moderation::create_label))
        .route(
            "/v1/admin/trust/jobs",
            get(trust::list_jobs).post(trust::create_job),
        )
        .route("/v1/attestations", post(trust::create_job))
        .with_state(state);

    let (status, payload) = post_json(
        app.clone(),
        "/v1/policies",
        json!({
            "policy_type": policy_type,
            "version": version,
            "locale": locale,
            "title": "Legacy Policy",
            "content_md": "legacy-create"
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("policy_id").and_then(Value::as_str),
        Some(policy_id.as_str())
    );

    let (status, payload) = put_json(
        app.clone(),
        &format!("/v1/policies/{policy_id}"),
        json!({
            "title": "Legacy Policy Updated",
            "content_md": "legacy-update"
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("title").and_then(Value::as_str),
        Some("Legacy Policy Updated")
    );

    let (status, payload) = post_json(
        app.clone(),
        &format!("/v1/policies/{policy_id}/publish"),
        json!({
            "effective_at": effective_at
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("effective_at").and_then(Value::as_i64),
        Some(effective_at)
    );

    let (status, payload) = post_json(
        app.clone(),
        &format!("/v1/policies/{policy_id}/make-current"),
        json!({}),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("is_current").and_then(Value::as_bool),
        Some(true)
    );

    let (status, payload) = get_json_with_session(
        app.clone(),
        &format!("/v1/admin/policies?policy_type={policy_type}&locale={locale}"),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rows = payload.as_array().expect("array payload");
    assert!(rows.iter().any(|row| {
        row.get("policy_id").and_then(Value::as_str) == Some(policy_id.as_str())
            && row.get("is_current").and_then(Value::as_bool) == Some(true)
    }));

    let exp = chrono::Utc::now().timestamp() + 3600;
    let (status, payload) = post_json(
        app.clone(),
        "/v1/labels",
        json!({
            "target": target,
            "label": "legacy-manual",
            "confidence": 0.7,
            "exp": exp,
            "policy_url": "https://example.com/policy/legacy",
            "policy_ref": "legacy-policy-v1",
            "topic_id": "kukuri:topic:legacy"
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let label_id = payload
        .get("label_id")
        .and_then(Value::as_str)
        .expect("label_id")
        .to_string();

    let (status, payload) = get_json_with_session(
        app.clone(),
        &format!("/v1/admin/moderation/labels?target={target}&limit=10"),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let labels = payload.as_array().expect("array payload");
    assert!(labels.iter().any(|row| {
        row.get("label_id").and_then(Value::as_str) == Some(label_id.as_str())
            && row.get("source").and_then(Value::as_str) == Some("manual")
    }));

    let (status, payload) = post_json(
        app.clone(),
        "/v1/attestations",
        json!({
            "job_type": "report_based",
            "subject_pubkey": subject_pubkey
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let job_id = payload
        .get("job_id")
        .and_then(Value::as_str)
        .expect("job_id")
        .to_string();

    let (status, payload) = get_json_with_session(
        app,
        &format!("/v1/admin/trust/jobs?subject_pubkey={subject_pubkey}&limit=10"),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let jobs = payload.as_array().expect("array payload");
    assert!(jobs.iter().any(|row| {
        row.get("job_id").and_then(Value::as_str) == Some(job_id.as_str())
            && row.get("job_type").and_then(Value::as_str) == Some("report_based")
    }));
}

#[tokio::test]
async fn moderation_contract_success_and_shape() {
    let state = test_state().await;
    let pool = state.pool.clone();
    let session_id = insert_admin_session(&state.pool).await;
    let reporter_pubkey = Keys::generate().public_key().to_hex();
    let target = format!("event:{}", Uuid::new_v4().simple());
    insert_report(&state.pool, &reporter_pubkey, &target, "spam").await;

    let app = Router::new()
        .route(
            "/v1/admin/moderation/rules",
            get(moderation::list_rules).post(moderation::create_rule),
        )
        .route(
            "/v1/admin/moderation/rules/{rule_id}",
            put(moderation::update_rule),
        )
        .route(
            "/v1/admin/moderation/reports",
            get(moderation::list_reports),
        )
        .route(
            "/v1/admin/moderation/labels",
            get(moderation::list_labels).post(moderation::create_label),
        )
        .route(
            "/v1/admin/moderation/labels/{label_id}/review",
            post(moderation::review_label),
        )
        .route(
            "/v1/admin/moderation/labels/{label_id}/rejudge",
            post(moderation::rejudge_label),
        )
        .with_state(state);

    let (status, payload) = post_json(
        app.clone(),
        "/v1/admin/moderation/rules",
        json!({
            "name": "spam-rule",
            "description": "contract test rule",
            "is_enabled": true,
            "priority": 10,
            "conditions": {
                "content_keywords": ["spam"]
            },
            "action": {
                "label": "spam",
                "confidence": 0.9,
                "exp_seconds": 3600,
                "policy_url": "https://example.com/policy",
                "policy_ref": "policy:spam:v1"
            }
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rule_id = payload
        .get("rule_id")
        .and_then(Value::as_str)
        .expect("rule_id")
        .to_string();
    assert_eq!(
        payload.get("name").and_then(Value::as_str),
        Some("spam-rule")
    );
    assert_eq!(
        payload.get("is_enabled").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(payload.get("priority").and_then(Value::as_i64), Some(10));
    assert!(payload
        .get("conditions")
        .and_then(Value::as_object)
        .is_some());
    assert!(payload.get("action").and_then(Value::as_object).is_some());

    let (status, payload) = get_json_with_session(
        app.clone(),
        "/v1/admin/moderation/rules?enabled=true&limit=10",
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rules = payload.as_array().expect("array payload");
    assert!(rules.iter().any(|row| {
        row.get("rule_id").and_then(Value::as_str) == Some(rule_id.as_str())
            && row.get("name").and_then(Value::as_str) == Some("spam-rule")
    }));

    let (status, payload) = put_json(
        app.clone(),
        &format!("/v1/admin/moderation/rules/{rule_id}"),
        json!({
            "name": "spam-rule-updated",
            "description": "updated",
            "is_enabled": true,
            "priority": 20,
            "conditions": {
                "content_keywords": ["spam", "scam"]
            },
            "action": {
                "label": "spam",
                "confidence": 0.95,
                "exp_seconds": 7200,
                "policy_url": "https://example.com/policy/v2",
                "policy_ref": "policy:spam:v2"
            }
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("name").and_then(Value::as_str),
        Some("spam-rule-updated")
    );
    assert_eq!(payload.get("priority").and_then(Value::as_i64), Some(20));

    let (status, payload) = get_json_with_session(
        app.clone(),
        &format!("/v1/admin/moderation/reports?target={target}&limit=10"),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let reports = payload.as_array().expect("array payload");
    let report = reports.first().expect("report row");
    assert_eq!(
        report.get("target").and_then(Value::as_str),
        Some(target.as_str())
    );
    assert_eq!(
        report.get("reporter_pubkey").and_then(Value::as_str),
        Some(reporter_pubkey.as_str())
    );
    assert_eq!(report.get("reason").and_then(Value::as_str), Some("spam"));
    assert!(report.get("created_at").and_then(Value::as_i64).is_some());

    let exp = chrono::Utc::now().timestamp() + 3600;
    let (status, payload) = post_json(
        app.clone(),
        "/v1/admin/moderation/labels",
        json!({
            "target": target,
            "label": "manual-spam",
            "confidence": 0.8,
            "exp": exp,
            "policy_url": "https://example.com/policy/manual",
            "policy_ref": "manual-v1",
            "topic_id": "kukuri:topic:contract"
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let label_id = payload
        .get("label_id")
        .and_then(Value::as_str)
        .expect("label_id")
        .to_string();
    assert_eq!(
        payload.get("status").and_then(Value::as_str),
        Some("created")
    );

    let (status, payload) = get_json_with_session(
        app.clone(),
        &format!("/v1/admin/moderation/labels?target={target}&limit=10"),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let labels = payload.as_array().expect("array payload");
    let label = labels
        .iter()
        .find(|row| row.get("label_id").and_then(Value::as_str) == Some(label_id.as_str()))
        .expect("label row");
    assert_eq!(
        label.get("target").and_then(Value::as_str),
        Some(target.as_str())
    );
    assert_eq!(
        label.get("label").and_then(Value::as_str),
        Some("manual-spam")
    );
    assert_eq!(label.get("source").and_then(Value::as_str), Some("manual"));
    assert!(label.get("issuer_pubkey").and_then(Value::as_str).is_some());
    assert_eq!(
        label.get("review_status").and_then(Value::as_str),
        Some("active")
    );
    assert_eq!(
        label.get("review_reason").and_then(Value::as_str),
        Some("manual-label-issued")
    );
    assert!(label.get("reviewed_by").and_then(Value::as_str).is_some());
    assert!(label.get("reviewed_at").and_then(Value::as_i64).is_some());

    let (status, payload) = post_json(
        app.clone(),
        &format!("/v1/admin/moderation/labels/{label_id}/review"),
        json!({
            "enabled": false,
            "reason": "false positive"
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("review_status").and_then(Value::as_str),
        Some("disabled")
    );
    assert_eq!(
        payload.get("review_reason").and_then(Value::as_str),
        Some("false positive")
    );
    assert!(payload.get("reviewed_by").and_then(Value::as_str).is_some());
    assert!(payload.get("reviewed_at").and_then(Value::as_i64).is_some());

    let (status, payload) = post_json(
        app.clone(),
        &format!("/v1/admin/moderation/labels/{label_id}/rejudge"),
        json!({
            "reason": "manual rejudge"
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("label_id").and_then(Value::as_str),
        Some(label_id.as_str())
    );
    assert_eq!(
        payload.get("event_id").and_then(Value::as_str),
        target.strip_prefix("event:")
    );
    assert_eq!(
        payload.get("status").and_then(Value::as_str),
        Some("queued")
    );
    assert_eq!(
        payload.get("enqueued_jobs").and_then(Value::as_i64),
        Some(1)
    );

    let queued_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_moderation.jobs WHERE event_id = $1 AND topic_id = $2 AND source = 'manual-rejudge' AND status = 'pending'",
    )
    .bind(target.strip_prefix("event:").expect("event target"))
    .bind("kukuri:topic:contract")
    .fetch_one(&pool)
    .await
    .expect("count rejudge jobs");
    assert_eq!(queued_count, 1);
}

#[tokio::test]
async fn moderation_rule_test_contract_success() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let sample_pubkey = Keys::generate().public_key().to_hex();

    let app = Router::new()
        .route(
            "/v1/admin/moderation/rules/test",
            post(moderation::test_rule),
        )
        .with_state(state);

    let (status, payload) = post_json(
        app,
        "/v1/admin/moderation/rules/test",
        json!({
            "conditions": {
                "kinds": [1],
                "content_keywords": ["spam"],
                "tag_filters": { "t": ["kukuri:topic:contract"] }
            },
            "action": {
                "label": "spam",
                "confidence": 0.9,
                "exp_seconds": 3600,
                "policy_url": "https://example.com/policy",
                "policy_ref": "policy:spam:v1"
            },
            "sample": {
                "event_id": "event-123",
                "pubkey": sample_pubkey,
                "kind": 1,
                "content": "this message looks like spam",
                "tags": [["t", "kukuri:topic:contract"]]
            }
        }),
        &session_id,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload.get("matched").and_then(Value::as_bool), Some(true));
    assert!(payload
        .get("reasons")
        .and_then(Value::as_array)
        .is_some_and(|rows| !rows.is_empty()));
    assert_eq!(
        payload.pointer("/preview/label").and_then(Value::as_str),
        Some("spam")
    );
    assert_eq!(
        payload.pointer("/preview/target").and_then(Value::as_str),
        Some("event:event-123")
    );
}

#[tokio::test]
async fn subscription_requests_and_node_subscriptions_contract_success() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let requester_approve = Keys::generate().public_key().to_hex();
    let requester_reject = Keys::generate().public_key().to_hex();
    let approve_topic_id = format!("kukuri:topic:approve:{}", Uuid::new_v4());
    let reject_topic_id = format!("kukuri:topic:reject:{}", Uuid::new_v4());
    let approve_request_id = insert_subscription_request(
        &state.pool,
        &requester_approve,
        &approve_topic_id,
        json!(["search", "trust"]),
    )
    .await;
    let reject_request_id = insert_subscription_request(
        &state.pool,
        &requester_reject,
        &reject_topic_id,
        json!(["bootstrap"]),
    )
    .await;

    let app = Router::new()
        .route(
            "/v1/admin/subscription-requests",
            get(subscriptions::list_subscription_requests),
        )
        .route(
            "/v1/admin/subscription-requests/{request_id}/approve",
            post(subscriptions::approve_subscription_request),
        )
        .route(
            "/v1/admin/subscription-requests/{request_id}/reject",
            post(subscriptions::reject_subscription_request),
        )
        .route(
            "/v1/admin/node-subscriptions",
            get(subscriptions::list_node_subscriptions),
        )
        .route(
            "/v1/admin/node-subscriptions/{topic_id}",
            put(subscriptions::update_node_subscription),
        )
        .with_state(state.clone());

    let (status, payload) = get_json_with_session(
        app.clone(),
        "/v1/admin/subscription-requests?status=pending",
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let pending_rows = payload.as_array().expect("array payload");
    assert!(pending_rows.iter().any(|row| {
        row.get("request_id").and_then(Value::as_str) == Some(approve_request_id.as_str())
            && row.get("status").and_then(Value::as_str) == Some("pending")
    }));

    let (status, payload) = post_json(
        app.clone(),
        &format!("/v1/admin/subscription-requests/{approve_request_id}/approve"),
        json!({ "review_note": "approved in contract test" }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("status").and_then(Value::as_str),
        Some("approved")
    );

    let (status, payload) = post_json(
        app.clone(),
        &format!("/v1/admin/subscription-requests/{reject_request_id}/reject"),
        json!({ "review_note": "rejected in contract test" }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("status").and_then(Value::as_str),
        Some("rejected")
    );

    let (status, payload) = get_json_with_session(
        app.clone(),
        "/v1/admin/subscription-requests?status=approved",
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let approved_rows = payload.as_array().expect("array payload");
    let approved_row = approved_rows
        .iter()
        .find(|row| {
            row.get("request_id").and_then(Value::as_str) == Some(approve_request_id.as_str())
        })
        .expect("approved row");
    assert_eq!(
        approved_row.get("topic_id").and_then(Value::as_str),
        Some(approve_topic_id.as_str())
    );
    assert_eq!(
        approved_row.get("requester_pubkey").and_then(Value::as_str),
        Some(requester_approve.as_str())
    );
    assert!(approved_row
        .get("requested_services")
        .and_then(Value::as_array)
        .is_some());
    assert!(approved_row
        .get("reviewed_at")
        .and_then(Value::as_i64)
        .is_some());

    let (status, payload) =
        get_json_with_session(app.clone(), "/v1/admin/node-subscriptions", &session_id).await;
    assert_eq!(status, StatusCode::OK);
    let node_rows = payload.as_array().expect("array payload");
    let node_row = node_rows
        .iter()
        .find(|row| row.get("topic_id").and_then(Value::as_str) == Some(approve_topic_id.as_str()))
        .expect("node subscription row");
    assert_eq!(node_row.get("enabled").and_then(Value::as_bool), Some(true));
    assert!(node_row.get("ref_count").and_then(Value::as_i64).is_some());
    assert!(node_row.get("ingest_policy").is_some_and(Value::is_null));
    assert!(node_row.get("updated_at").and_then(Value::as_i64).is_some());

    let (status, payload) = put_json(
        app.clone(),
        &format!("/v1/admin/node-subscriptions/{approve_topic_id}"),
        json!({
            "enabled": false,
            "ingest_policy": {
                "retention_days": 7,
                "max_events": 50,
                "max_bytes": 1024,
                "allow_backfill": false
            }
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("topic_id").and_then(Value::as_str),
        Some(approve_topic_id.as_str())
    );
    assert_eq!(payload.get("enabled").and_then(Value::as_bool), Some(false));
    assert!(payload.get("ref_count").and_then(Value::as_i64).is_some());
    let ingest_policy = payload
        .get("ingest_policy")
        .and_then(Value::as_object)
        .expect("ingest_policy object");
    assert_eq!(
        ingest_policy.get("retention_days").and_then(Value::as_i64),
        Some(7)
    );
    assert_eq!(
        ingest_policy.get("max_events").and_then(Value::as_i64),
        Some(50)
    );
    assert_eq!(
        ingest_policy.get("max_bytes").and_then(Value::as_i64),
        Some(1024)
    );
    assert_eq!(
        ingest_policy.get("allow_backfill").and_then(Value::as_bool),
        Some(false)
    );

    let (status, payload) = put_json(
        app.clone(),
        &format!("/v1/admin/node-subscriptions/{approve_topic_id}"),
        json!({ "enabled": true }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload.get("enabled").and_then(Value::as_bool), Some(true));
    let preserved_policy = payload
        .get("ingest_policy")
        .and_then(Value::as_object)
        .expect("preserved ingest_policy object");
    assert_eq!(
        preserved_policy
            .get("allow_backfill")
            .and_then(Value::as_bool),
        Some(false)
    );

    let stored_policy: Value = sqlx::query_scalar(
        "SELECT ingest_policy FROM cn_admin.node_subscriptions WHERE topic_id = $1",
    )
    .bind(&approve_topic_id)
    .fetch_one(&state.pool)
    .await
    .expect("load stored ingest policy");
    assert_eq!(
        stored_policy.get("retention_days").and_then(Value::as_i64),
        Some(7)
    );
}

#[tokio::test]
async fn subscription_request_approve_rejects_when_node_topic_limit_reached() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;

    let mut effective_enabled_topics = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_admin.node_subscriptions WHERE enabled = TRUE",
    )
    .fetch_one(&state.pool)
    .await
    .expect("count existing enabled node subscriptions");
    if effective_enabled_topics == 0 {
        let existing_topic_id = format!("kukuri:topic:limit-existing:{}", Uuid::new_v4());
        sqlx::query(
            "INSERT INTO cn_admin.node_subscriptions (topic_id, enabled, ref_count) VALUES ($1, TRUE, 1)",
        )
        .bind(&existing_topic_id)
        .execute(&state.pool)
        .await
        .expect("insert existing enabled node subscription");
        effective_enabled_topics = 1;
    }

    upsert_service_config(
        &state.pool,
        "relay",
        json!({
            "node_subscription": {
                "max_concurrent_topics": effective_enabled_topics
            }
        }),
    )
    .await;

    let requester_pubkey = Keys::generate().public_key().to_hex();
    let topic_id = format!("kukuri:topic:limit-over:{}", Uuid::new_v4());
    let request_id =
        insert_subscription_request(&state.pool, &requester_pubkey, &topic_id, json!(["search"]))
            .await;

    let app = Router::new()
        .route(
            "/v1/admin/subscription-requests/{request_id}/approve",
            post(subscriptions::approve_subscription_request),
        )
        .with_state(state.clone());

    let (status, payload) = post_json(
        app,
        &format!("/v1/admin/subscription-requests/{request_id}/approve"),
        json!({ "review_note": "over-limit approval should fail" }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        payload.get("code").and_then(Value::as_str),
        Some("NODE_SUBSCRIPTION_TOPIC_LIMIT_REACHED")
    );
    assert_eq!(
        payload.pointer("/details/metric").and_then(Value::as_str),
        Some("node_subscriptions.enabled_topics")
    );
    assert_eq!(
        payload.pointer("/details/scope").and_then(Value::as_str),
        Some("node")
    );
    assert_eq!(
        payload.pointer("/details/current").and_then(Value::as_i64),
        Some(effective_enabled_topics)
    );
    assert_eq!(
        payload.pointer("/details/limit").and_then(Value::as_i64),
        Some(effective_enabled_topics)
    );

    let request_status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM cn_user.topic_subscription_requests WHERE request_id = $1",
    )
    .bind(&request_id)
    .fetch_one(&state.pool)
    .await
    .expect("fetch request status after over-limit failure");
    assert_eq!(request_status, "pending");

    let topic_subscription_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_user.topic_subscriptions WHERE topic_id = $1 AND subscriber_pubkey = $2",
    )
    .bind(&topic_id)
    .bind(&requester_pubkey)
    .fetch_one(&state.pool)
    .await
    .expect("count topic subscriptions after over-limit failure");
    assert_eq!(topic_subscription_count, 0);

    let node_subscription_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_admin.node_subscriptions WHERE topic_id = $1",
    )
    .bind(&topic_id)
    .fetch_one(&state.pool)
    .await
    .expect("count node subscriptions after over-limit failure");
    assert_eq!(node_subscription_count, 0);

    upsert_service_config(
        &state.pool,
        "relay",
        json!({
            "node_subscription": {
                "max_concurrent_topics": cn_core::service_config::DEFAULT_MAX_CONCURRENT_NODE_TOPICS
            }
        }),
    )
    .await;
}

#[tokio::test]
async fn subscription_request_approve_rejects_when_node_topic_limit_already_exceeded() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;

    let mut effective_enabled_topics = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_admin.node_subscriptions WHERE enabled = TRUE",
    )
    .fetch_one(&state.pool)
    .await
    .expect("count existing enabled node subscriptions");
    while effective_enabled_topics < 2 {
        let existing_topic_id = format!("kukuri:topic:limit-exceeded-existing:{}", Uuid::new_v4());
        sqlx::query(
            "INSERT INTO cn_admin.node_subscriptions (topic_id, enabled, ref_count) VALUES ($1, TRUE, 1)",
        )
        .bind(&existing_topic_id)
        .execute(&state.pool)
        .await
        .expect("insert existing enabled node subscription");
        effective_enabled_topics += 1;
    }

    let configured_limit = effective_enabled_topics - 1;
    upsert_service_config(
        &state.pool,
        "relay",
        json!({
            "node_subscription": {
                "max_concurrent_topics": configured_limit
            }
        }),
    )
    .await;

    let requester_pubkey = Keys::generate().public_key().to_hex();
    let topic_id = format!("kukuri:topic:limit-exceeded-over:{}", Uuid::new_v4());
    let request_id =
        insert_subscription_request(&state.pool, &requester_pubkey, &topic_id, json!(["search"]))
            .await;

    let app = Router::new()
        .route(
            "/v1/admin/subscription-requests/{request_id}/approve",
            post(subscriptions::approve_subscription_request),
        )
        .with_state(state.clone());

    let (status, payload) = post_json(
        app,
        &format!("/v1/admin/subscription-requests/{request_id}/approve"),
        json!({ "review_note": "over-limit approval should fail when current > limit" }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        payload.get("code").and_then(Value::as_str),
        Some("NODE_SUBSCRIPTION_TOPIC_LIMIT_REACHED")
    );
    assert_eq!(
        payload.pointer("/details/metric").and_then(Value::as_str),
        Some("node_subscriptions.enabled_topics")
    );
    assert_eq!(
        payload.pointer("/details/scope").and_then(Value::as_str),
        Some("node")
    );
    assert_eq!(
        payload.pointer("/details/current").and_then(Value::as_i64),
        Some(effective_enabled_topics)
    );
    assert_eq!(
        payload.pointer("/details/limit").and_then(Value::as_i64),
        Some(configured_limit)
    );
    assert!(payload
        .pointer("/details/current")
        .and_then(Value::as_i64)
        .zip(payload.pointer("/details/limit").and_then(Value::as_i64))
        .is_some_and(|(current, limit)| current > limit));

    let request_status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM cn_user.topic_subscription_requests WHERE request_id = $1",
    )
    .bind(&request_id)
    .fetch_one(&state.pool)
    .await
    .expect("fetch request status after over-limit failure");
    assert_eq!(request_status, "pending");

    let topic_subscription_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_user.topic_subscriptions WHERE topic_id = $1 AND subscriber_pubkey = $2",
    )
    .bind(&topic_id)
    .bind(&requester_pubkey)
    .fetch_one(&state.pool)
    .await
    .expect("count topic subscriptions after over-limit failure");
    assert_eq!(topic_subscription_count, 0);

    let node_subscription_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_admin.node_subscriptions WHERE topic_id = $1",
    )
    .bind(&topic_id)
    .fetch_one(&state.pool)
    .await
    .expect("count node subscriptions after over-limit failure");
    assert_eq!(node_subscription_count, 0);

    upsert_service_config(
        &state.pool,
        "relay",
        json!({
            "node_subscription": {
                "max_concurrent_topics": cn_core::service_config::DEFAULT_MAX_CONCURRENT_NODE_TOPICS
            }
        }),
    )
    .await;
}

#[tokio::test]
async fn plans_subscriptions_usage_contract_success() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let plan_id = format!("plan-{}", &Uuid::new_v4().to_string()[..8]);
    let subscriber_pubkey = Keys::generate().public_key().to_hex();

    let app = Router::new()
        .route(
            "/v1/admin/plans",
            get(subscriptions::list_plans).post(subscriptions::create_plan),
        )
        .route("/v1/admin/plans/{plan_id}", put(subscriptions::update_plan))
        .route(
            "/v1/admin/subscriptions",
            get(subscriptions::list_subscriptions),
        )
        .route(
            "/v1/admin/subscriptions/{subscriber_pubkey}",
            put(subscriptions::upsert_subscription),
        )
        .route("/v1/admin/usage", get(subscriptions::list_usage))
        .with_state(state.clone());

    let (status, payload) = post_json(
        app.clone(),
        "/v1/admin/plans",
        json!({
            "plan_id": plan_id,
            "name": "Starter",
            "is_active": true,
            "limits": [
                { "metric": "search", "window": "day", "limit": 100 },
                { "metric": "labels", "window": "day", "limit": 50 }
            ]
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("plan_id").and_then(Value::as_str),
        Some(plan_id.as_str())
    );
    assert_eq!(payload.get("name").and_then(Value::as_str), Some("Starter"));
    assert_eq!(
        payload.get("is_active").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .get("limits")
            .and_then(Value::as_array)
            .map(std::vec::Vec::len),
        Some(2)
    );

    let (status, payload) = put_json(
        app.clone(),
        &format!("/v1/admin/plans/{plan_id}"),
        json!({
            "plan_id": plan_id,
            "name": "Starter Plus",
            "is_active": true,
            "limits": [
                { "metric": "search", "window": "day", "limit": 200 }
            ]
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("name").and_then(Value::as_str),
        Some("Starter Plus")
    );
    assert_eq!(
        payload
            .get("limits")
            .and_then(Value::as_array)
            .map(std::vec::Vec::len),
        Some(1)
    );

    let (status, payload) =
        get_json_with_session(app.clone(), "/v1/admin/plans", &session_id).await;
    assert_eq!(status, StatusCode::OK);
    let plans = payload.as_array().expect("array payload");
    let plan = plans
        .iter()
        .find(|row| row.get("plan_id").and_then(Value::as_str) == Some(plan_id.as_str()))
        .expect("plan row");
    assert_eq!(
        plan.get("name").and_then(Value::as_str),
        Some("Starter Plus")
    );
    assert!(plan.get("limits").and_then(Value::as_array).is_some());

    let (status, payload) = put_json(
        app.clone(),
        &format!("/v1/admin/subscriptions/{subscriber_pubkey}"),
        json!({
            "plan_id": plan_id,
            "status": "active"
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload.get("status").and_then(Value::as_str), Some("ok"));

    let (status, payload) = get_json_with_session(
        app.clone(),
        &format!("/v1/admin/subscriptions?pubkey={subscriber_pubkey}"),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let subscriptions = payload.as_array().expect("array payload");
    let subscription = subscriptions.first().expect("subscription row");
    assert_eq!(
        subscription
            .get("subscriber_pubkey")
            .and_then(Value::as_str),
        Some(subscriber_pubkey.as_str())
    );
    assert_eq!(
        subscription.get("plan_id").and_then(Value::as_str),
        Some(plan_id.as_str())
    );
    assert_eq!(
        subscription.get("status").and_then(Value::as_str),
        Some("active")
    );
    assert!(subscription
        .get("started_at")
        .and_then(Value::as_i64)
        .is_some());

    insert_usage_counter(&state.pool, &subscriber_pubkey, "search", 42).await;
    let (status, payload) = get_json_with_session(
        app,
        &format!("/v1/admin/usage?pubkey={subscriber_pubkey}&metric=search&days=30"),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let usage = payload.as_array().expect("array payload");
    let usage_row = usage.first().expect("usage row");
    assert_eq!(
        usage_row.get("metric").and_then(Value::as_str),
        Some("search")
    );
    assert!(usage_row.get("day").and_then(Value::as_str).is_some());
    assert_eq!(usage_row.get("count").and_then(Value::as_i64), Some(42));
}

#[tokio::test]
async fn audit_logs_contract_success_and_shape() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let action = format!("contract.action.{}", &Uuid::new_v4().to_string()[..8]);
    let target = format!("contract-target:{}", Uuid::new_v4());
    let actor = format!("admin-{}", Uuid::new_v4());
    let request_id = format!("req-{}", Uuid::new_v4());
    insert_audit_log(
        &state.pool,
        &actor,
        &action,
        &target,
        json!({ "diff": "ok" }),
        &request_id,
    )
    .await;

    let app = Router::new()
        .route("/v1/admin/audit-logs", get(services::list_audit_logs))
        .with_state(state);

    let (status, payload) = get_json_with_session(
        app,
        &format!("/v1/admin/audit-logs?action={action}&target={target}&limit=10"),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rows = payload.as_array().expect("array payload");
    let row = rows.first().expect("audit log row");
    assert!(row.get("audit_id").and_then(Value::as_i64).is_some());
    assert_eq!(
        row.get("actor_admin_user_id").and_then(Value::as_str),
        Some(actor.as_str())
    );
    assert_eq!(
        row.get("action").and_then(Value::as_str),
        Some(action.as_str())
    );
    assert_eq!(
        row.get("target").and_then(Value::as_str),
        Some(target.as_str())
    );
    assert!(row.get("diff_json").and_then(Value::as_object).is_some());
    assert_eq!(
        row.get("request_id").and_then(Value::as_str),
        Some(request_id.as_str())
    );
    assert!(row.get("created_at").and_then(Value::as_i64).is_some());
}

#[tokio::test]
async fn dsar_jobs_contract_list_retry_cancel_and_audit_success() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let export_request_id = Uuid::new_v4().to_string();
    let deletion_request_id = Uuid::new_v4().to_string();
    let export_requester = Keys::generate().public_key().to_hex();
    let deletion_requester = Keys::generate().public_key().to_hex();
    insert_export_request(
        &state.pool,
        &export_request_id,
        &export_requester,
        "failed",
        Some("timeout"),
    )
    .await;
    insert_deletion_request(
        &state.pool,
        &deletion_request_id,
        &deletion_requester,
        "running",
        None,
    )
    .await;

    let app = Router::new()
        .route("/v1/admin/personal-data-jobs", get(dsar::list_jobs))
        .route(
            "/v1/admin/personal-data-jobs/{job_type}/{job_id}/retry",
            post(dsar::retry_job),
        )
        .route(
            "/v1/admin/personal-data-jobs/{job_type}/{job_id}/cancel",
            post(dsar::cancel_job),
        )
        .with_state(state.clone());

    let (status, payload) = get_json_with_session(
        app.clone(),
        "/v1/admin/personal-data-jobs?limit=10",
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rows = payload.as_array().expect("array payload");
    assert!(rows.iter().any(|row| {
        row.get("job_id").and_then(Value::as_str) == Some(export_request_id.as_str())
            && row.get("request_type").and_then(Value::as_str) == Some("export")
            && row.get("status").and_then(Value::as_str) == Some("failed")
    }));
    assert!(rows.iter().any(|row| {
        row.get("job_id").and_then(Value::as_str) == Some(deletion_request_id.as_str())
            && row.get("request_type").and_then(Value::as_str) == Some("deletion")
            && row.get("status").and_then(Value::as_str) == Some("running")
    }));

    let (status, payload) = post_json(
        app.clone(),
        &format!("/v1/admin/personal-data-jobs/export/{export_request_id}/retry"),
        json!({}),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("job_id").and_then(Value::as_str),
        Some(export_request_id.as_str())
    );
    assert_eq!(
        payload.get("request_type").and_then(Value::as_str),
        Some("export")
    );
    assert_eq!(
        payload.get("status").and_then(Value::as_str),
        Some("queued")
    );
    assert!(payload.get("completed_at").is_some_and(Value::is_null));
    assert!(payload.get("error_message").is_some_and(Value::is_null));

    let (status, payload) = post_json(
        app.clone(),
        &format!("/v1/admin/personal-data-jobs/deletion/{deletion_request_id}/cancel"),
        json!({}),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("job_id").and_then(Value::as_str),
        Some(deletion_request_id.as_str())
    );
    assert_eq!(
        payload.get("request_type").and_then(Value::as_str),
        Some("deletion")
    );
    assert_eq!(
        payload.get("status").and_then(Value::as_str),
        Some("failed")
    );
    assert!(payload
        .get("completed_at")
        .and_then(Value::as_i64)
        .is_some());
    assert_eq!(
        payload.get("error_message").and_then(Value::as_str),
        Some("canceled by admin")
    );

    let (status, payload) = get_json_with_session(
        app.clone(),
        "/v1/admin/personal-data-jobs?status=queued&request_type=export&limit=10",
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rows = payload.as_array().expect("array payload");
    assert!(rows.iter().any(|row| {
        row.get("job_id").and_then(Value::as_str) == Some(export_request_id.as_str())
            && row.get("status").and_then(Value::as_str) == Some("queued")
    }));

    let (status, payload) = get_json_with_session(
        app,
        "/v1/admin/personal-data-jobs?status=invalid",
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        payload.get("code").and_then(Value::as_str),
        Some("INVALID_STATUS")
    );

    let retry_target = format!("dsar:export:{export_request_id}");
    let cancel_target = format!("dsar:deletion:{deletion_request_id}");
    let audit_rows = sqlx::query(
        "SELECT action, target, diff_json \
         FROM cn_admin.audit_logs \
         WHERE target = $1 OR target = $2 \
         ORDER BY audit_id DESC",
    )
    .bind(&retry_target)
    .bind(&cancel_target)
    .fetch_all(&state.pool)
    .await
    .expect("fetch dsar audit rows");
    assert!(audit_rows.iter().any(|row| {
        let action: String = row.try_get("action").expect("action");
        let target: String = row.try_get("target").expect("target");
        let diff: Value = row
            .try_get::<Option<Value>, _>("diff_json")
            .expect("diff_json")
            .expect("diff_json");
        action == "dsar.job.retry"
            && target == retry_target
            && diff.get("previous_status").and_then(Value::as_str) == Some("failed")
            && diff.get("next_status").and_then(Value::as_str) == Some("queued")
    }));
    assert!(audit_rows.iter().any(|row| {
        let action: String = row.try_get("action").expect("action");
        let target: String = row.try_get("target").expect("target");
        let diff: Value = row
            .try_get::<Option<Value>, _>("diff_json")
            .expect("diff_json")
            .expect("diff_json");
        action == "dsar.job.cancel"
            && target == cancel_target
            && diff.get("previous_status").and_then(Value::as_str) == Some("running")
            && diff.get("next_status").and_then(Value::as_str) == Some("failed")
    }));
}

#[tokio::test]
async fn trust_targets_contract_search_success() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let subject_pubkey = Keys::generate().public_key().to_hex();
    insert_trust_scores(&state.pool, &subject_pubkey).await;

    let app = Router::new()
        .route("/v1/admin/trust/targets", get(trust::list_targets))
        .with_state(state);

    let (status, payload) = get_json_with_session(
        app,
        &format!(
            "/v1/admin/trust/targets?pubkey={}&limit=10",
            &subject_pubkey[..16]
        ),
        &session_id,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let rows = payload.as_array().expect("array payload");
    let row = rows
        .iter()
        .find(|row| {
            row.get("subject_pubkey").and_then(Value::as_str) == Some(subject_pubkey.as_str())
        })
        .expect("target row");
    assert_eq!(row.get("report_score").and_then(Value::as_f64), Some(0.85));
    assert_eq!(
        row.get("communication_score").and_then(Value::as_f64),
        Some(0.65)
    );
    assert!(row.get("updated_at").and_then(Value::as_i64).is_some());
}

#[tokio::test]
async fn trust_contract_success_and_shape() {
    let state = test_state().await;
    let session_id = insert_admin_session(&state.pool).await;
    let subject_pubkey = Keys::generate().public_key().to_hex();
    let job_type = "report_based";

    let app = Router::new()
        .route(
            "/v1/admin/trust/jobs",
            get(trust::list_jobs).post(trust::create_job),
        )
        .route("/v1/admin/trust/schedules", get(trust::list_schedules))
        .route(
            "/v1/admin/trust/schedules/{job_type}",
            put(trust::update_schedule),
        )
        .with_state(state);

    let (status, payload) = post_json(
        app.clone(),
        "/v1/admin/trust/jobs",
        json!({
            "job_type": job_type,
            "subject_pubkey": subject_pubkey
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let job_id = payload
        .get("job_id")
        .and_then(Value::as_str)
        .expect("job_id")
        .to_string();
    assert!(Uuid::parse_str(&job_id).is_ok());
    assert_eq!(
        payload.get("job_type").and_then(Value::as_str),
        Some(job_type)
    );
    assert_eq!(
        payload.get("subject_pubkey").and_then(Value::as_str),
        Some(subject_pubkey.as_str())
    );
    assert_eq!(
        payload.get("status").and_then(Value::as_str),
        Some("pending")
    );
    assert!(payload
        .get("requested_by")
        .and_then(Value::as_str)
        .is_some());
    assert!(payload
        .get("requested_at")
        .and_then(Value::as_i64)
        .is_some());

    let (status, payload) = get_json_with_session(
        app.clone(),
        &format!(
            "/v1/admin/trust/jobs?status=pending&job_type={job_type}&subject_pubkey={subject_pubkey}&limit=10"
        ),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let jobs = payload.as_array().expect("array payload");
    assert!(jobs.iter().any(|row| {
        row.get("job_id").and_then(Value::as_str) == Some(job_id.as_str())
            && row.get("job_type").and_then(Value::as_str) == Some(job_type)
    }));

    let (status, payload) = put_json(
        app.clone(),
        "/v1/admin/trust/schedules/report_based",
        json!({
            "interval_seconds": 1800,
            "is_enabled": true
        }),
        &session_id,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("job_type").and_then(Value::as_str),
        Some(job_type)
    );
    assert_eq!(
        payload.get("interval_seconds").and_then(Value::as_i64),
        Some(1800)
    );
    assert_eq!(
        payload.get("is_enabled").and_then(Value::as_bool),
        Some(true)
    );
    assert!(payload.get("next_run_at").and_then(Value::as_i64).is_some());
    assert!(payload.get("updated_at").and_then(Value::as_i64).is_some());

    let (status, payload) =
        get_json_with_session(app, "/v1/admin/trust/schedules", &session_id).await;
    assert_eq!(status, StatusCode::OK);
    let rows = payload.as_array().expect("array payload");
    assert!(rows.iter().any(|row| {
        row.get("job_type").and_then(Value::as_str) == Some(job_type)
            && row.get("interval_seconds").and_then(Value::as_i64) == Some(1800)
    }));
}
