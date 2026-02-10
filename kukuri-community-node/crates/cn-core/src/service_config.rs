use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::postgres::PgListener;
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

const ADMIN_CONFIG_CHANNEL: &str = "cn_admin_config";
const LISTENER_RETRY_INTERVAL_SECONDS: u64 = 5;

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
            AuthMode::Required => self.enforce_at.map(|ts| now >= ts).unwrap_or(true),
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
        .map(|v| match v {
            "required" => AuthMode::Required,
            _ => AuthMode::Off,
        })
        .unwrap_or(AuthMode::Off);

    let enforce_at = auth
        .and_then(|auth| auth.get("enforce_at"))
        .and_then(|v| v.as_i64());
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
        config_json: initial.map(|cfg| cfg.config_json).unwrap_or(default_config),
    };

    let state = Arc::new(RwLock::new(snapshot));
    let initial_listener = connect_admin_config_listener(&pool, service).await;
    let state_ref = Arc::clone(&state);
    tokio::spawn(async move {
        let poll_interval = if poll_interval.is_zero() {
            Duration::from_secs(1)
        } else {
            poll_interval
        };
        let mut poll_timer = time::interval(poll_interval);
        poll_timer.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
        poll_timer.tick().await;

        let listener_retry = Duration::from_secs(LISTENER_RETRY_INTERVAL_SECONDS);
        let mut listener = initial_listener;

        loop {
            if let Some(active_listener) = listener.as_mut() {
                let mut listener_failed = false;
                tokio::select! {
                    _ = poll_timer.tick() => {
                        refresh_service_config(&pool, service, &state_ref, "poll").await;
                    }
                    notification = active_listener.recv() => {
                        match notification {
                            Ok(notification) => {
                                let payload = notification.payload();
                                if should_refresh_on_admin_config_notification(payload, service) {
                                    refresh_service_config(&pool, service, &state_ref, "notify").await;
                                }
                            }
                            Err(err) => {
                                tracing::warn!(
                                    service = service,
                                    channel = ADMIN_CONFIG_CHANNEL,
                                    error = %err,
                                    "service config listener disconnected; will reconnect"
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
                    _ = poll_timer.tick() => {
                        refresh_service_config(&pool, service, &state_ref, "poll").await;
                    }
                    _ = time::sleep(listener_retry) => {
                        listener = connect_admin_config_listener(&pool, service).await;
                    }
                }
            }
        }
    });

    Ok(ServiceConfigHandle { state })
}

async fn refresh_service_config(
    pool: &Pool<Postgres>,
    service: &str,
    state_ref: &Arc<RwLock<ServiceConfigSnapshot>>,
    trigger: &str,
) {
    match load_service_config(pool, service).await {
        Ok(Some(cfg)) => {
            let mut guard = state_ref.write().await;
            if cfg.version != guard.version {
                *guard = ServiceConfigSnapshot {
                    version: cfg.version,
                    config_json: cfg.config_json,
                };
                tracing::info!(
                    service = service,
                    version = guard.version,
                    trigger = trigger,
                    "service config updated"
                );
            }
        }
        Ok(None) => {}
        Err(err) => {
            tracing::warn!(
                service = service,
                trigger = trigger,
                error = %err,
                "service config refresh failed"
            );
        }
    }
}

async fn connect_admin_config_listener(pool: &Pool<Postgres>, service: &str) -> Option<PgListener> {
    let mut listener = match PgListener::connect_with(pool).await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::warn!(
                service = service,
                channel = ADMIN_CONFIG_CHANNEL,
                error = %err,
                "failed to connect service config listener"
            );
            return None;
        }
    };

    if let Err(err) = listener.listen(ADMIN_CONFIG_CHANNEL).await {
        tracing::warn!(
            service = service,
            channel = ADMIN_CONFIG_CHANNEL,
            error = %err,
            "failed to subscribe service config listener"
        );
        return None;
    }

    tracing::info!(
        service = service,
        channel = ADMIN_CONFIG_CHANNEL,
        "service config listener subscribed"
    );
    Some(listener)
}

fn should_refresh_on_admin_config_notification(payload: &str, service: &str) -> bool {
    let notified_service = payload.split(':').next().unwrap_or_default().trim();
    notified_service.is_empty() || notified_service.eq_ignore_ascii_case(service)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sqlx::postgres::PgPoolOptions;
    use std::sync::OnceLock;
    use tokio::sync::{Mutex, OnceCell};
    use tokio::time::timeout;

    static MIGRATIONS: OnceCell<()> = OnceCell::const_new();
    const WATCH_NOTIFY_SERVICE: &str = "cn-core-watch-notify";
    const WATCH_POLL_SERVICE: &str = "cn-core-watch-poll";

    fn database_url() -> String {
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://cn:cn_password@localhost:5432/cn".to_string())
    }

    fn db_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    async fn ensure_migrated(pool: &Pool<Postgres>) {
        MIGRATIONS
            .get_or_init(|| async {
                crate::migrations::run(pool)
                    .await
                    .expect("run community-node migrations");
            })
            .await;
    }

    async fn test_pool() -> Pool<Postgres> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url())
            .await
            .expect("connect database");
        ensure_migrated(&pool).await;
        pool
    }

    async fn clear_service_config(pool: &Pool<Postgres>, service: &str) {
        sqlx::query("DELETE FROM cn_admin.service_configs WHERE service = $1")
            .bind(service)
            .execute(pool)
            .await
            .expect("clear service config");
    }

    async fn upsert_service_config(
        pool: &Pool<Postgres>,
        service: &str,
        config_json: Value,
        notify_payload: Option<&str>,
    ) -> i64 {
        let mut tx = pool.begin().await.expect("begin transaction");
        let current = sqlx::query_scalar::<_, i64>(
            "SELECT version FROM cn_admin.service_configs WHERE service = $1",
        )
        .bind(service)
        .fetch_optional(&mut *tx)
        .await
        .expect("select service config version");

        let next_version = if let Some(current_version) = current {
            let next = current_version + 1;
            sqlx::query(
                "UPDATE cn_admin.service_configs                  SET config_json = $1, version = $2, updated_at = NOW(), updated_by = $3                  WHERE service = $4",
            )
            .bind(&config_json)
            .bind(next)
            .bind("cn-core-test")
            .bind(service)
            .execute(&mut *tx)
            .await
            .expect("update service config");
            next
        } else {
            sqlx::query(
                "INSERT INTO cn_admin.service_configs                  (service, version, config_json, updated_by)                  VALUES ($1, 1, $2, $3)",
            )
            .bind(service)
            .bind(&config_json)
            .bind("cn-core-test")
            .execute(&mut *tx)
            .await
            .expect("insert service config");
            1
        };

        if let Some(payload) = notify_payload {
            sqlx::query("SELECT pg_notify('cn_admin_config', $1)")
                .bind(payload)
                .execute(&mut *tx)
                .await
                .expect("notify service config");
        }

        tx.commit().await.expect("commit transaction");
        next_version
    }

    async fn wait_for_version(
        handle: &ServiceConfigHandle,
        expected_version: i64,
        timeout_duration: Duration,
    ) -> ServiceConfigSnapshot {
        timeout(timeout_duration, async {
            loop {
                let snapshot = handle.get().await;
                if snapshot.version == expected_version {
                    return snapshot;
                }
                time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("wait for service config update")
    }

    #[test]
    fn should_refresh_on_admin_config_notification_matches_expected_service() {
        assert!(should_refresh_on_admin_config_notification("", "relay"));
        assert!(should_refresh_on_admin_config_notification(
            "relay", "relay"
        ));
        assert!(should_refresh_on_admin_config_notification(
            "relay:12", "relay"
        ));
        assert!(should_refresh_on_admin_config_notification(
            " ReLaY : 15",
            "relay"
        ));
        assert!(!should_refresh_on_admin_config_notification(
            "trust:7", "relay"
        ));
    }

    #[tokio::test]
    async fn watch_service_config_updates_on_admin_notify() {
        let _guard = db_test_lock().lock().await;
        let pool = test_pool().await;
        clear_service_config(&pool, WATCH_NOTIFY_SERVICE).await;

        upsert_service_config(
            &pool,
            WATCH_NOTIFY_SERVICE,
            json!({ "mode": "initial" }),
            None,
        )
        .await;

        let handle = watch_service_config(
            pool.clone(),
            WATCH_NOTIFY_SERVICE,
            json!({ "mode": "default" }),
            Duration::from_secs(30),
        )
        .await
        .expect("start config watch");

        let initial = wait_for_version(&handle, 1, Duration::from_secs(2)).await;
        assert_eq!(
            initial
                .config_json
                .get("mode")
                .and_then(|value| value.as_str()),
            Some("initial")
        );

        upsert_service_config(
            &pool,
            WATCH_NOTIFY_SERVICE,
            json!({ "mode": "updated-by-notify" }),
            Some("cn-core-watch-notify:2"),
        )
        .await;

        let updated = wait_for_version(&handle, 2, Duration::from_secs(2)).await;
        assert_eq!(
            updated
                .config_json
                .get("mode")
                .and_then(|value| value.as_str()),
            Some("updated-by-notify")
        );

        clear_service_config(&pool, WATCH_NOTIFY_SERVICE).await;
    }

    #[tokio::test]
    async fn watch_service_config_poll_fallback_updates_without_notify() {
        let _guard = db_test_lock().lock().await;
        let pool = test_pool().await;
        clear_service_config(&pool, WATCH_POLL_SERVICE).await;

        upsert_service_config(
            &pool,
            WATCH_POLL_SERVICE,
            json!({ "mode": "initial" }),
            None,
        )
        .await;

        let handle = watch_service_config(
            pool.clone(),
            WATCH_POLL_SERVICE,
            json!({ "mode": "default" }),
            Duration::from_millis(200),
        )
        .await
        .expect("start config watch");

        let initial = wait_for_version(&handle, 1, Duration::from_secs(2)).await;
        assert_eq!(
            initial
                .config_json
                .get("mode")
                .and_then(|value| value.as_str()),
            Some("initial")
        );

        upsert_service_config(
            &pool,
            WATCH_POLL_SERVICE,
            json!({ "mode": "updated-by-poll" }),
            None,
        )
        .await;

        let updated = wait_for_version(&handle, 2, Duration::from_secs(3)).await;
        assert_eq!(
            updated
                .config_json
                .get("mode")
                .and_then(|value| value.as_str()),
            Some("updated-by-poll")
        );

        clear_service_config(&pool, WATCH_POLL_SERVICE).await;
    }
}
