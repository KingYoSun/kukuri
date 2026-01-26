use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Pool, Postgres, Row};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub service: String,
    pub version: i64,
    pub config_json: Value,
}

#[derive(Debug, Clone)]
pub struct ServiceConfigSnapshot {
    pub version: i64,
    pub config_json: Value,
}

#[derive(Clone)]
pub struct ServiceConfigHandle {
    state: Arc<RwLock<ServiceConfigSnapshot>>,
}

impl ServiceConfigHandle {
    pub async fn get(&self) -> ServiceConfigSnapshot {
        self.state.read().await.clone()
    }
}

pub fn static_handle(config_json: Value) -> ServiceConfigHandle {
    let snapshot = ServiceConfigSnapshot {
        version: 0,
        config_json,
    };
    ServiceConfigHandle {
        state: Arc::new(RwLock::new(snapshot)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    Off,
    Required,
}

#[derive(Debug, Clone, Copy)]
pub struct AuthConfig {
    pub mode: AuthMode,
    pub enforce_at: Option<i64>,
    pub grace_seconds: i64,
    pub ws_auth_timeout_seconds: i64,
}

impl AuthConfig {
    pub fn requires_auth(&self, now: i64) -> bool {
        match self.mode {
            AuthMode::Off => false,
            AuthMode::Required => {
                self.enforce_at.map(|ts| now >= ts).unwrap_or(true)
            }
        }
    }

    pub fn disconnect_deadline(&self) -> Option<i64> {
        self.enforce_at
            .and_then(|ts| ts.checked_add(self.grace_seconds.max(0)))
    }
}

pub async fn load_service_config(
    pool: &Pool<Postgres>,
    service: &str,
) -> Result<Option<ServiceConfig>> {
    let row = sqlx::query(
        "SELECT service, version, config_json FROM cn_admin.service_configs WHERE service = $1",
    )
    .bind(service)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        Ok(Some(ServiceConfig {
            service: row.try_get("service")?,
            version: row.try_get("version")?,
            config_json: row.try_get("config_json")?,
        }))
    } else {
        Ok(None)
    }
}

pub fn auth_config_from_json(value: &Value) -> AuthConfig {
    let auth = value.get("auth").and_then(|v| v.as_object());
    let mode = auth
        .and_then(|auth| auth.get("mode"))
        .and_then(|v| v.as_str())
        .and_then(|v| match v {
            "required" => Some(AuthMode::Required),
            _ => Some(AuthMode::Off),
        })
        .unwrap_or(AuthMode::Off);

    let enforce_at = auth.and_then(|auth| auth.get("enforce_at")).and_then(|v| v.as_i64());
    let grace_seconds = auth
        .and_then(|auth| auth.get("grace_seconds"))
        .and_then(|v| v.as_i64())
        .unwrap_or(900);
    let ws_auth_timeout_seconds = auth
        .and_then(|auth| auth.get("ws_auth_timeout_seconds"))
        .and_then(|v| v.as_i64())
        .unwrap_or(10);

    AuthConfig {
        mode,
        enforce_at,
        grace_seconds,
        ws_auth_timeout_seconds,
    }
}

pub async fn watch_service_config(
    pool: Pool<Postgres>,
    service: &'static str,
    default_config: Value,
    poll_interval: Duration,
) -> Result<ServiceConfigHandle> {
    let initial = load_service_config(&pool, service).await?;
    let snapshot = ServiceConfigSnapshot {
        version: initial.as_ref().map(|cfg| cfg.version).unwrap_or(0),
        config_json: initial
            .map(|cfg| cfg.config_json)
            .unwrap_or(default_config),
    };

    let state = Arc::new(RwLock::new(snapshot));
    let state_ref = Arc::clone(&state);
    tokio::spawn(async move {
        loop {
            match load_service_config(&pool, service).await {
                Ok(Some(cfg)) => {
                    let mut guard = state_ref.write().await;
                    if cfg.version != guard.version {
                        *guard = ServiceConfigSnapshot {
                            version: cfg.version,
                            config_json: cfg.config_json,
                        };
                        tracing::info!(service = service, version = guard.version, "service config updated");
                    }
                }
                Ok(None) => {}
                Err(err) => {
                    tracing::warn!(service = service, error = %err, "service config poll failed");
                }
            }
            time::sleep(poll_interval).await;
        }
    });

    Ok(ServiceConfigHandle { state })
}
