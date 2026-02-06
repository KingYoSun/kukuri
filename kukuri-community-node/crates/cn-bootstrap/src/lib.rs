use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use cn_core::{config, db, http, logging, metrics, node_key, nostr, server, service_config};
use nostr_sdk::prelude::Keys;
use serde::Serialize;
use serde_json::json;
use sqlx::{Pool, Postgres, Row};
use std::path::PathBuf;
use std::time::Duration;

const SERVICE_NAME: &str = "cn-bootstrap";

#[derive(Clone)]
struct AppState {
    pool: Pool<Postgres>,
    keys: Keys,
    refresh_interval: Duration,
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
    let state = AppState {
        pool: pool.clone(),
        keys,
        refresh_interval: Duration::from_secs(config.refresh_interval_seconds),
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
    loop {
        if let Err(err) = refresh_bootstrap_events(&state).await {
            tracing::error!(error = %err, "bootstrap refresh failed");
        }
        tokio::time::sleep(state.refresh_interval).await;
    }
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

    if !active_tags.is_empty() {
        sqlx::query("DELETE FROM cn_bootstrap.events WHERE kind = 39001 AND d_tag <> ALL($1)")
            .bind(&active_tags)
            .execute(&mut *tx)
            .await?;
    }

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
    match db::check_ready(&state.pool).await {
        Ok(_) => (
            StatusCode::OK,
            Json(HealthStatus {
                status: "ok".into(),
            }),
        ),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthStatus {
                status: "unavailable".into(),
            }),
        ),
    }
}

async fn metrics_endpoint() -> impl IntoResponse {
    metrics::metrics_response(SERVICE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nostr_sdk::prelude::Keys;
    use serde_json::json;

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
}
