use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use cn_core::{
    config, db, health, http, logging, metrics, node_key, nostr, server, service_config,
};
use nostr_sdk::prelude::Keys;
use serde::Serialize;
use serde_json::json;
use sqlx::postgres::PgListener;
use sqlx::{Pool, Postgres, Row, Transaction};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

const SERVICE_NAME: &str = "cn-bootstrap";
const ADMIN_CONFIG_CHANNEL: &str = "cn_admin_config";
const LISTENER_RETRY_INTERVAL_SECONDS: u64 = 5;

#[derive(Clone)]
struct AppState {
    pool: Pool<Postgres>,
    keys: Keys,
    refresh_interval: Duration,
    health_targets: Arc<HashMap<String, String>>,
    health_client: reqwest::Client,
}

#[derive(Serialize)]
struct HealthStatus {
    status: String,
}

pub struct BootstrapConfig {
    pub addr: std::net::SocketAddr,
    pub database_url: String,
    pub node_key_path: PathBuf,
    pub refresh_interval_seconds: u64,
}

pub fn load_config() -> Result<BootstrapConfig> {
    let addr = config::socket_addr_from_env("BOOTSTRAP_ADDR", "0.0.0.0:8083")?;
    let database_url = config::required_env("DATABASE_URL")?;
    let node_key_path = node_key::key_path_from_env("NODE_KEY_PATH", "data/node_key.json")?;
    let refresh_interval_seconds = std::env::var("BOOTSTRAP_REFRESH_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(300);
    Ok(BootstrapConfig {
        addr,
        database_url,
        node_key_path,
        refresh_interval_seconds,
    })
}

pub async fn run(config: BootstrapConfig) -> Result<()> {
    logging::init(SERVICE_NAME);
    let pool = db::connect(&config.database_url).await?;
    let keys = node_key::load_or_generate(&config.node_key_path)?;
    let health_targets = Arc::new(health::parse_health_targets(
        "BOOTSTRAP_HEALTH_TARGETS",
        &[
            ("relay", "RELAY_HEALTH_URL", "http://relay:8082/healthz"),
            (
                "user-api",
                "USER_API_HEALTH_URL",
                "http://user-api:8080/healthz",
            ),
        ],
    ));
    let health_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let state = AppState {
        pool: pool.clone(),
        keys,
        refresh_interval: Duration::from_secs(config.refresh_interval_seconds),
        health_targets,
        health_client,
    };

    tokio::spawn(refresh_loop(state.clone()));

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/metrics", get(metrics_endpoint))
        .with_state(state);

    let router = http::apply_standard_layers(router, SERVICE_NAME);
    server::serve(config.addr, router).await
}

async fn refresh_loop(state: AppState) {
    refresh_once_with_log(&state, "startup").await;
    let mut refresh_interval = tokio::time::interval(state.refresh_interval);
    refresh_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    refresh_interval.tick().await;

    let listener_retry = Duration::from_secs(LISTENER_RETRY_INTERVAL_SECONDS);
    let mut listener = connect_admin_config_listener(&state.pool).await;

    loop {
        if let Some(active_listener) = listener.as_mut() {
            let mut listener_failed = false;
            tokio::select! {
                _ = refresh_interval.tick() => {
                    refresh_once_with_log(&state, "interval").await;
                }
                notification = active_listener.recv() => {
                    match notification {
                        Ok(notification) => {
                            let payload = notification.payload();
                            if should_refresh_on_admin_config_notification(payload) {
                                tracing::info!(
                                    payload = payload,
                                    channel = ADMIN_CONFIG_CHANNEL,
                                    "bootstrap refresh triggered by admin config notification"
                                );
                                refresh_once_with_log(&state, "notify").await;
                            }
                        }
                        Err(err) => {
                            tracing::warn!(
                                error = %err,
                                channel = ADMIN_CONFIG_CHANNEL,
                                "admin config listener disconnected; will reconnect"
                            );
                            listener_failed = true;
                        }
                    }
                }
            }

            if listener_failed {
                listener = None;
            }
        } else {
            tokio::select! {
                _ = refresh_interval.tick() => {
                    refresh_once_with_log(&state, "interval").await;
                }
                _ = tokio::time::sleep(listener_retry) => {
                    listener = connect_admin_config_listener(&state.pool).await;
                }
            }
        }
    }
}

async fn refresh_once_with_log(state: &AppState, trigger: &str) {
    if let Err(err) = refresh_bootstrap_events(state).await {
        tracing::error!(error = %err, trigger = trigger, "bootstrap refresh failed");
    }
}

async fn connect_admin_config_listener(pool: &Pool<Postgres>) -> Option<PgListener> {
    let mut listener = match PgListener::connect_with(pool).await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::warn!(
                error = %err,
                channel = ADMIN_CONFIG_CHANNEL,
                "failed to connect admin config listener"
            );
            return None;
        }
    };

    if let Err(err) = listener.listen(ADMIN_CONFIG_CHANNEL).await {
        tracing::warn!(
            error = %err,
            channel = ADMIN_CONFIG_CHANNEL,
            "failed to subscribe admin config listener"
        );
        return None;
    }

    tracing::info!(
        channel = ADMIN_CONFIG_CHANNEL,
        "admin config listener subscribed"
    );
    Some(listener)
}

fn should_refresh_on_admin_config_notification(payload: &str) -> bool {
    let service = payload.split(':').next().unwrap_or_default().trim();
    service.is_empty()
        || service.eq_ignore_ascii_case("bootstrap")
        || service.eq_ignore_ascii_case("topic_services")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TopicServiceCleanupMode {
    DeleteAll,
    DeleteStale,
}

fn topic_service_cleanup_mode(active_tags: &[String]) -> TopicServiceCleanupMode {
    if active_tags.is_empty() {
        TopicServiceCleanupMode::DeleteAll
    } else {
        TopicServiceCleanupMode::DeleteStale
    }
}

async fn cleanup_stale_topic_service_events(
    tx: &mut Transaction<'_, Postgres>,
    active_tags: &[String],
) -> Result<()> {
    match topic_service_cleanup_mode(active_tags) {
        TopicServiceCleanupMode::DeleteAll => {
            sqlx::query("DELETE FROM cn_bootstrap.events WHERE kind = 39001")
                .execute(&mut **tx)
                .await?;
        }
        TopicServiceCleanupMode::DeleteStale => {
            sqlx::query("DELETE FROM cn_bootstrap.events WHERE kind = 39001 AND d_tag <> ALL($1)")
                .bind(active_tags)
                .execute(&mut **tx)
                .await?;
        }
    }

    Ok(())
}

async fn refresh_bootstrap_events(state: &AppState) -> Result<()> {
    let config = service_config::load_service_config(&state.pool, "bootstrap")
        .await?
        .map(|cfg| cfg.config_json)
        .unwrap_or_else(|| json!({}));

    let descriptor = config
        .get("descriptor")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let exp_config = config.get("exp").cloned().unwrap_or_else(|| json!({}));

    let now = cn_core::auth::unix_seconds()? as i64;
    let descriptor_exp_days = exp_config
        .get("descriptor_days")
        .and_then(|v| v.as_i64())
        .unwrap_or(7);
    let topic_exp_hours = exp_config
        .get("topic_hours")
        .and_then(|v| v.as_i64())
        .unwrap_or(48);

    let descriptor_exp = now + descriptor_exp_days * 86400;
    let topic_exp = now + topic_exp_hours * 3600;

    let descriptor_event = build_descriptor_event(&state.keys, &descriptor, descriptor_exp)?;

    let mut tx = state.pool.begin().await?;
    upsert_bootstrap_event(
        &mut tx,
        &descriptor_event,
        39000,
        "descriptor",
        None,
        None,
        None,
        descriptor_exp,
    )
    .await?;

    let topic_services = load_topic_services(&mut tx).await?;
    let mut active_tags = Vec::new();

    for (topic_id, role, scope) in topic_services {
        let d_tag = format!("topic_service:{topic_id}:{role}:{scope}");
        let event =
            build_topic_service_event(&state.keys, &topic_id, &role, &scope, &d_tag, topic_exp)?;
        upsert_bootstrap_event(
            &mut tx,
            &event,
            39001,
            &d_tag,
            Some(&topic_id),
            Some(&role),
            Some(&scope),
            topic_exp,
        )
        .await?;
        active_tags.push(d_tag);
    }

    cleanup_stale_topic_service_events(&mut tx, &active_tags).await?;

    sqlx::query("DELETE FROM cn_bootstrap.events WHERE expires_at <= $1")
        .bind(now)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(())
}

async fn load_topic_services(
    tx: &mut sqlx::Transaction<'_, Postgres>,
) -> Result<Vec<(String, String, String)>> {
    let rows = sqlx::query(
        "SELECT topic_id, role, scope FROM cn_admin.topic_services WHERE is_active = TRUE",
    )
    .fetch_all(&mut **tx)
    .await?;

    let mut services = Vec::new();
    for row in rows {
        services.push((
            row.try_get("topic_id")?,
            row.try_get("role")?,
            row.try_get("scope")?,
        ));
    }
    Ok(services)
}

#[allow(clippy::too_many_arguments)]
async fn upsert_bootstrap_event(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    event: &nostr::RawEvent,
    kind: i32,
    d_tag: &str,
    topic_id: Option<&str>,
    role: Option<&str>,
    scope: Option<&str>,
    expires_at: i64,
) -> Result<()> {
    sqlx::query("DELETE FROM cn_bootstrap.events WHERE kind = $1 AND d_tag = $2")
        .bind(kind)
        .bind(d_tag)
        .execute(&mut **tx)
        .await?;

    sqlx::query(
        "INSERT INTO cn_bootstrap.events          (event_id, kind, d_tag, topic_id, role, scope, event_json, created_at, expires_at, updated_at, is_active)          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), TRUE)",
    )
    .bind(&event.id)
    .bind(kind)
    .bind(d_tag)
    .bind(topic_id)
    .bind(role)
    .bind(scope)
    .bind(serde_json::to_value(event)?)
    .bind(event.created_at)
    .bind(expires_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

fn build_descriptor_event(
    keys: &Keys,
    descriptor: &serde_json::Value,
    exp: i64,
) -> Result<nostr::RawEvent> {
    let roles = descriptor
        .get("roles")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let role_tags: Vec<Vec<String>> = roles
        .iter()
        .filter_map(|role| {
            role.as_str()
                .map(|r| vec!["role".to_string(), r.to_string()])
        })
        .collect();

    let policy_url = descriptor
        .get("policy_url")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let jurisdiction = descriptor
        .get("jurisdiction")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let mut tags = vec![
        vec!["d".to_string(), "descriptor".to_string()],
        vec!["k".to_string(), "kukuri".to_string()],
        vec!["ver".to_string(), "1".to_string()],
        vec!["exp".to_string(), exp.to_string()],
    ];

    if !policy_url.is_empty() {
        tags.push(vec!["policy".to_string(), policy_url.to_string()]);
    }
    if !jurisdiction.is_empty() {
        tags.push(vec!["jurisdiction".to_string(), jurisdiction.to_string()]);
    }
    tags.extend(role_tags);

    let content = json!({
        "schema": "kukuri-node-desc-v1",
        "name": descriptor.get("name").and_then(|v| v.as_str()).unwrap_or(""),
        "roles": roles,
        "endpoints": descriptor.get("endpoints").cloned().unwrap_or_else(|| json!({})),
        "pricing": descriptor.get("pricing").cloned().unwrap_or_else(|| json!({})),
        "policy_url": policy_url,
        "jurisdiction": jurisdiction,
        "contact": descriptor.get("contact").and_then(|v| v.as_str()).unwrap_or(""),
    })
    .to_string();

    nostr::build_signed_event(keys, 39000, tags, content)
}

fn build_topic_service_event(
    keys: &Keys,
    topic_id: &str,
    role: &str,
    scope: &str,
    d_tag: &str,
    exp: i64,
) -> Result<nostr::RawEvent> {
    let tags = vec![
        vec!["d".to_string(), d_tag.to_string()],
        vec!["t".to_string(), topic_id.to_string()],
        vec!["role".to_string(), role.to_string()],
        vec!["scope".to_string(), scope.to_string()],
        vec!["k".to_string(), "kukuri".to_string()],
        vec!["ver".to_string(), "1".to_string()],
        vec!["exp".to_string(), exp.to_string()],
    ];
    let content = json!({
        "schema": "kukuri-topic-service-v1",
        "topic": topic_id,
        "role": role,
        "scope": scope
    })
    .to_string();
    nostr::build_signed_event(keys, 39001, tags, content)
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

async fn metrics_endpoint() -> impl IntoResponse {
    metrics::metrics_response(SERVICE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{anyhow, bail};
    use axum::body::to_bytes;
    use axum::http::header;
    use axum::response::IntoResponse;
    use nostr_sdk::prelude::Keys;
    use serde_json::json;
    use sqlx::postgres::PgPoolOptions;
    use sqlx::{Pool, Postgres};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::sync::{Arc, OnceLock};
    use std::time::Duration;
    use tokio::sync::{Mutex, OnceCell};
    use uuid::Uuid;

    static MIGRATIONS: OnceCell<()> = OnceCell::const_new();

    fn db_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn database_url() -> String {
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://cn:cn_password@localhost:5432/cn".to_string())
    }

    async fn ensure_migrated(pool: &Pool<Postgres>) {
        MIGRATIONS
            .get_or_init(|| async {
                cn_core::migrations::run(pool)
                    .await
                    .expect("run community-node migrations");
            })
            .await;
    }

    async fn test_state(health_targets: HashMap<String, String>) -> AppState {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url())
            .await
            .expect("connect database");
        ensure_migrated(&pool).await;

        AppState {
            pool,
            keys: Keys::generate(),
            refresh_interval: Duration::from_secs(1),
            health_targets: Arc::new(health_targets),
            health_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("build health client"),
        }
    }

    async fn upsert_topic_service(
        pool: &Pool<Postgres>,
        topic_id: &str,
        role: &str,
        scope: &str,
        is_active: bool,
    ) {
        sqlx::query(
            "INSERT INTO cn_admin.topic_services \
             (topic_id, role, scope, is_active, updated_by) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (topic_id, role, scope) \
             DO UPDATE SET is_active = EXCLUDED.is_active, updated_at = NOW(), updated_by = EXCLUDED.updated_by",
        )
        .bind(topic_id)
        .bind(role)
        .bind(scope)
        .bind(is_active)
        .bind("cn-bootstrap-test")
        .execute(pool)
        .await
        .expect("upsert topic service");
    }

    async fn remove_topic_service(pool: &Pool<Postgres>, topic_id: &str, role: &str, scope: &str) {
        sqlx::query(
            "DELETE FROM cn_admin.topic_services WHERE topic_id = $1 AND role = $2 AND scope = $3",
        )
        .bind(topic_id)
        .bind(role)
        .bind(scope)
        .execute(pool)
        .await
        .expect("remove topic service");
    }

    async fn deactivate_all_active_topic_services(
        pool: &Pool<Postgres>,
    ) -> Vec<(String, String, String)> {
        let rows = sqlx::query(
            "SELECT topic_id, role, scope FROM cn_admin.topic_services WHERE is_active = TRUE",
        )
        .fetch_all(pool)
        .await
        .expect("load active topic services");

        sqlx::query("UPDATE cn_admin.topic_services SET is_active = FALSE WHERE is_active = TRUE")
            .execute(pool)
            .await
            .expect("deactivate active topic services");

        rows.into_iter()
            .map(|row| {
                (
                    row.try_get("topic_id").expect("topic_id"),
                    row.try_get("role").expect("role"),
                    row.try_get("scope").expect("scope"),
                )
            })
            .collect()
    }

    async fn restore_active_topic_services(
        pool: &Pool<Postgres>,
        active_rows: &[(String, String, String)],
    ) {
        for (topic_id, role, scope) in active_rows {
            upsert_topic_service(pool, topic_id, role, scope, true).await;
        }
    }

    async fn response_json(response: axum::response::Response) -> serde_json::Value {
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read response body");
        serde_json::from_slice(&body).expect("response body as json")
    }

    async fn response_text(response: axum::response::Response) -> String {
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read response body");
        String::from_utf8(body.to_vec()).expect("response body as utf-8")
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

    async fn spawn_dependency_health_server(
        dependency_status: Arc<AtomicU16>,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let app = Router::new().route(
            "/healthz",
            get({
                let dependency_status = Arc::clone(&dependency_status);
                move || {
                    let dependency_status = Arc::clone(&dependency_status);
                    async move {
                        let status_code =
                            StatusCode::from_u16(dependency_status.load(Ordering::Relaxed))
                                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                        let body_status = if status_code.is_success() {
                            "ok"
                        } else {
                            "unavailable"
                        };
                        (status_code, Json(json!({ "status": body_status })))
                    }
                }
            }),
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind dependency server");
        let addr = listener
            .local_addr()
            .expect("resolve dependency server address");
        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        (format!("http://{addr}/healthz"), handle)
    }

    fn has_tag(tags: &[Vec<String>], name: &str, value: &str) -> bool {
        tags.iter().any(|tag| {
            tag.get(0).map(|v| v.as_str()) == Some(name)
                && tag.get(1).map(|v| v.as_str()) == Some(value)
        })
    }

    #[test]
    fn build_descriptor_event_includes_required_tags() {
        let keys = Keys::generate();
        let descriptor = json!({
            "name": "Test Node",
            "roles": ["bootstrap", "index"],
            "endpoints": { "http": "https://node.example" },
            "policy_url": "https://node.example/policy",
            "jurisdiction": "JP",
            "contact": "ops@example"
        });
        let exp = 1_725_000_000_i64;

        let event = build_descriptor_event(&keys, &descriptor, exp).expect("event");

        assert!(has_tag(&event.tags, "d", "descriptor"));
        assert!(has_tag(&event.tags, "k", "kukuri"));
        assert!(has_tag(&event.tags, "ver", "1"));
        assert!(has_tag(&event.tags, "exp", &exp.to_string()));
        assert!(has_tag(
            &event.tags,
            "policy",
            "https://node.example/policy"
        ));
        assert!(has_tag(&event.tags, "jurisdiction", "JP"));
        assert!(has_tag(&event.tags, "role", "bootstrap"));
        assert!(has_tag(&event.tags, "role", "index"));

        let content: serde_json::Value =
            serde_json::from_str(&event.content).expect("content json");
        assert_eq!(
            content.get("schema").and_then(|v| v.as_str()),
            Some("kukuri-node-desc-v1")
        );
    }

    #[test]
    fn build_topic_service_event_includes_required_tags() {
        let keys = Keys::generate();
        let exp = 1_725_000_000_i64;
        let event = build_topic_service_event(
            &keys,
            "kukuri:topic1",
            "index",
            "public",
            "topic_service:kukuri:topic1:index:public",
            exp,
        )
        .expect("event");

        assert!(has_tag(
            &event.tags,
            "d",
            "topic_service:kukuri:topic1:index:public"
        ));
        assert!(has_tag(&event.tags, "t", "kukuri:topic1"));
        assert!(has_tag(&event.tags, "role", "index"));
        assert!(has_tag(&event.tags, "scope", "public"));
        assert!(has_tag(&event.tags, "k", "kukuri"));
        assert!(has_tag(&event.tags, "ver", "1"));
        assert!(has_tag(&event.tags, "exp", &exp.to_string()));

        let content: serde_json::Value =
            serde_json::from_str(&event.content).expect("content json");
        assert_eq!(
            content.get("schema").and_then(|v| v.as_str()),
            Some("kukuri-topic-service-v1")
        );
    }

    #[test]
    fn should_refresh_on_admin_config_notification_handles_payloads() {
        assert!(should_refresh_on_admin_config_notification(""));
        assert!(should_refresh_on_admin_config_notification("bootstrap"));
        assert!(should_refresh_on_admin_config_notification("bootstrap:42"));
        assert!(should_refresh_on_admin_config_notification(
            "topic_services:7"
        ));
        assert!(!should_refresh_on_admin_config_notification("index:12"));
    }

    #[test]
    fn topic_service_cleanup_mode_uses_delete_all_when_no_active_tags() {
        let active_tags: Vec<String> = Vec::new();
        assert_eq!(
            topic_service_cleanup_mode(&active_tags),
            TopicServiceCleanupMode::DeleteAll
        );
    }

    #[test]
    fn topic_service_cleanup_mode_uses_delete_stale_when_active_tags_exist() {
        let active_tags = vec!["topic_service:kukuri:topic1:index:public".to_string()];
        assert_eq!(
            topic_service_cleanup_mode(&active_tags),
            TopicServiceCleanupMode::DeleteStale
        );
    }

    #[tokio::test]
    async fn refresh_bootstrap_events_reflects_topic_services_into_db() {
        let _guard = db_test_lock().lock().await;
        let state = test_state(HashMap::new()).await;
        let topic_id = format!("kukuri:bootstrap-refresh-{}", Uuid::new_v4());
        let role = "index";
        let scope = "public";
        let d_tag = format!("topic_service:{topic_id}:{role}:{scope}");

        upsert_topic_service(&state.pool, &topic_id, role, scope, true).await;
        refresh_bootstrap_events(&state)
            .await
            .expect("refresh bootstrap events");

        let descriptor_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cn_bootstrap.events WHERE kind = 39000 AND d_tag = 'descriptor'",
        )
        .fetch_one(&state.pool)
        .await
        .expect("count descriptor events");
        assert_eq!(descriptor_count, 1);

        let row = sqlx::query(
            "SELECT topic_id, role, scope, event_json FROM cn_bootstrap.events \
             WHERE kind = 39001 AND d_tag = $1",
        )
        .bind(&d_tag)
        .fetch_optional(&state.pool)
        .await
        .expect("load topic service bootstrap event")
        .expect("topic service bootstrap event");

        let row_topic_id: String = row.try_get("topic_id").expect("topic_id");
        let row_role: String = row.try_get("role").expect("role");
        let row_scope: String = row.try_get("scope").expect("scope");
        let event_json: serde_json::Value = row.try_get("event_json").expect("event_json");

        assert_eq!(row_topic_id, topic_id);
        assert_eq!(row_role, role);
        assert_eq!(row_scope, scope);
        assert_eq!(event_json.get("kind").and_then(|v| v.as_i64()), Some(39001));

        remove_topic_service(&state.pool, &topic_id, role, scope).await;
        sqlx::query("DELETE FROM cn_bootstrap.events WHERE d_tag = $1")
            .bind(&d_tag)
            .execute(&state.pool)
            .await
            .expect("cleanup bootstrap event");
    }

    #[tokio::test]
    async fn refresh_bootstrap_events_deletes_stale_39001_when_topic_services_empty() {
        let _guard = db_test_lock().lock().await;
        let state = test_state(HashMap::new()).await;
        let active_rows = deactivate_all_active_topic_services(&state.pool).await;

        let stale_topic_id = format!("kukuri:bootstrap-stale-{}", Uuid::new_v4());
        let stale_d_tag = format!("topic_service:{stale_topic_id}:index:public");
        let expires_at = cn_core::auth::unix_seconds().expect("unix time") as i64 + 3600;
        let stale_event = build_topic_service_event(
            &state.keys,
            &stale_topic_id,
            "index",
            "public",
            &stale_d_tag,
            expires_at,
        )
        .expect("build stale topic service event");

        let mut tx = state.pool.begin().await.expect("begin transaction");
        upsert_bootstrap_event(
            &mut tx,
            &stale_event,
            39001,
            &stale_d_tag,
            Some(&stale_topic_id),
            Some("index"),
            Some("public"),
            expires_at,
        )
        .await
        .expect("insert stale bootstrap event");
        tx.commit().await.expect("commit stale event insert");

        let test_result: anyhow::Result<()> = async {
            let stale_before: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM cn_bootstrap.events WHERE d_tag = $1")
                    .bind(&stale_d_tag)
                    .fetch_one(&state.pool)
                    .await
                    .expect("count stale event before refresh");
            if stale_before != 1 {
                bail!("unexpected stale row count before refresh: {stale_before}");
            }

            refresh_bootstrap_events(&state)
                .await
                .expect("refresh bootstrap events");

            let stale_after: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM cn_bootstrap.events WHERE d_tag = $1")
                    .bind(&stale_d_tag)
                    .fetch_one(&state.pool)
                    .await
                    .expect("count stale event after refresh");
            if stale_after != 0 {
                bail!("stale 39001 rows still remain after refresh: {stale_after}");
            }

            Ok(())
        }
        .await;

        restore_active_topic_services(&state.pool, &active_rows).await;
        refresh_bootstrap_events(&state)
            .await
            .expect("refresh bootstrap events after restore");

        if let Err(err) = test_result {
            panic!("topic_services empty cleanup assertion failed: {err}");
        }
    }

    #[tokio::test]
    async fn healthz_contract_status_transitions_when_dependency_fails() {
        let _guard = db_test_lock().lock().await;
        let dependency_status = Arc::new(AtomicU16::new(StatusCode::OK.as_u16()));
        let (health_url, server_handle) =
            spawn_dependency_health_server(Arc::clone(&dependency_status)).await;
        let mut targets = HashMap::new();
        targets.insert("relay".to_string(), health_url);

        let state = test_state(targets).await;

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
    }

    #[tokio::test]
    async fn metrics_contract_prometheus_content_type_shape_compatible() {
        let route = "/metrics-contract";
        metrics::record_http_request(
            SERVICE_NAME,
            "GET",
            route,
            200,
            std::time::Duration::from_millis(5),
        );

        let response = metrics_endpoint().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok());
        assert_eq!(content_type, Some("text/plain; version=0.0.4"));

        let body = response_text(response).await;
        if !body.contains("cn_up{service=\"cn-bootstrap\"}") {
            let preview = body.lines().take(10).collect::<Vec<_>>().join("\n");
            panic!(
                "{}",
                anyhow!("metrics payload does not contain cn-bootstrap service gauge:\n{preview}")
            );
        }
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
