use crate::{access_control, reindex, AppState};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::routing::{get, post};
use axum::Router;
use cn_core::service_config;
use nostr_sdk::prelude::Keys;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::collections::HashMap;
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
        health_targets: Arc::new(HashMap::new()),
        health_client: reqwest::Client::new(),
        node_keys: Keys::generate(),
    }
}

async fn insert_admin_session(pool: &Pool<Postgres>) -> String {
    let admin_user_id = Uuid::new_v4().to_string();
    let username = format!("admin-{}", &admin_user_id[..8]);
    let password_hash = cn_core::admin::hash_password("test-password").expect("hash password");

    sqlx::query(
        "INSERT INTO cn_admin.admin_users          (admin_user_id, username, password_hash, is_active)          VALUES ($1, $2, $3, TRUE)",
    )
    .bind(&admin_user_id)
    .bind(&username)
    .bind(&password_hash)
    .execute(pool)
    .await
    .expect("insert admin user");

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
    assert_eq!(rows[0].get("status").and_then(Value::as_str), Some("active"));
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
        .pointer("/paths/~1v1~1admin~1access-control~1memberships/get")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1admin~1trust~1schedules/get")
        .is_some());
    assert!(payload.pointer("/paths/~1v1~1reindex/post").is_some());
    assert!(payload.pointer("/components/schemas/ServiceInfo").is_some());
    assert!(payload
        .pointer("/components/schemas/TrustScheduleRow")
        .is_some());
}
