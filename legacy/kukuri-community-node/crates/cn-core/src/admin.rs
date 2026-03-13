use anyhow::{anyhow, Result};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand_core::OsRng;
use serde_json::json;
use sqlx::types::Json;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

fn read_trimmed_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
}

fn bootstrap_descriptor_http_url() -> String {
    read_trimmed_env("BOOTSTRAP_DESCRIPTOR_HTTP_URL")
        .or_else(|| read_trimmed_env("PUBLIC_BASE_URL"))
        .unwrap_or_else(|| "http://localhost:8080".to_string())
}

fn bootstrap_descriptor_ws_url() -> String {
    read_trimmed_env("BOOTSTRAP_DESCRIPTOR_WS_URL")
        .or_else(|| read_trimmed_env("RELAY_PUBLIC_URL"))
        .unwrap_or_else(|| "ws://localhost:8082/relay".to_string())
}

fn should_sync_bootstrap_descriptor() -> bool {
    [
        "BOOTSTRAP_DESCRIPTOR_HTTP_URL",
        "BOOTSTRAP_DESCRIPTOR_WS_URL",
        "PUBLIC_BASE_URL",
        "RELAY_PUBLIC_URL",
    ]
    .into_iter()
    .any(|name| read_trimmed_env(name).is_some())
}

fn default_bootstrap_service_config(
    descriptor_http_url: &str,
    descriptor_ws_url: &str,
) -> serde_json::Value {
    json!({
        "auth": {
            "mode": "off",
            "enforce_at": null,
            "grace_seconds": 900
        },
        "descriptor": {
            "name": "Kukuri Community Node",
            "roles": ["bootstrap", "relay"],
            "endpoints": {
                "http": descriptor_http_url,
                "ws": descriptor_ws_url
            },
            "policy_url": "",
            "jurisdiction": "",
            "contact": ""
        },
        "exp": {
            "descriptor_days": 7,
            "topic_hours": 48
        }
    })
}

fn merge_bootstrap_descriptor_config(
    existing: serde_json::Value,
    descriptor_http_url: &str,
    descriptor_ws_url: &str,
) -> serde_json::Value {
    let mut config = if existing.is_object() {
        existing
    } else {
        default_bootstrap_service_config(descriptor_http_url, descriptor_ws_url)
    };

    let Some(root) = config.as_object_mut() else {
        return default_bootstrap_service_config(descriptor_http_url, descriptor_ws_url);
    };

    let descriptor = root
        .entry("descriptor")
        .or_insert_with(|| json!({ "endpoints": {} }));
    if !descriptor.is_object() {
        *descriptor = json!({ "endpoints": {} });
    }

    let Some(descriptor_obj) = descriptor.as_object_mut() else {
        return default_bootstrap_service_config(descriptor_http_url, descriptor_ws_url);
    };
    let endpoints = descriptor_obj
        .entry("endpoints")
        .or_insert_with(|| json!({}));
    if !endpoints.is_object() {
        *endpoints = json!({});
    }

    if let Some(endpoints_obj) = endpoints.as_object_mut() {
        endpoints_obj.insert("http".to_string(), json!(descriptor_http_url));
        endpoints_obj.insert("ws".to_string(), json!(descriptor_ws_url));
    }

    config
}

async fn upsert_bootstrap_descriptor_config(
    pool: &Pool<Postgres>,
    descriptor_http_url: &str,
    descriptor_ws_url: &str,
) -> Result<u64> {
    let existing = sqlx::query_scalar::<_, Json<serde_json::Value>>(
        "SELECT config_json FROM cn_admin.service_configs WHERE service = 'bootstrap'",
    )
    .fetch_optional(pool)
    .await?;
    let config_json = existing
        .map(|Json(value)| {
            merge_bootstrap_descriptor_config(value, descriptor_http_url, descriptor_ws_url)
        })
        .unwrap_or_else(|| {
            default_bootstrap_service_config(descriptor_http_url, descriptor_ws_url)
        });

    let result = sqlx::query(
        "INSERT INTO cn_admin.service_configs \
         (service, version, config_json, updated_by) \
         VALUES ($1, 1, $2, $3) \
         ON CONFLICT (service) DO UPDATE SET \
           config_json = EXCLUDED.config_json, \
           updated_by = EXCLUDED.updated_by, \
           updated_at = now()",
    )
    .bind("bootstrap")
    .bind(Json(config_json))
    .bind("system")
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn bootstrap_admin(
    pool: &Pool<Postgres>,
    username: &str,
    password: &str,
) -> Result<bool> {
    if username.trim().is_empty() {
        return Err(anyhow!("username is required"));
    }
    if password.is_empty() {
        return Err(anyhow!("password is required"));
    }

    let mut tx = pool.begin().await?;
    let existing =
        sqlx::query_scalar::<_, String>("SELECT admin_user_id FROM cn_admin.admin_users LIMIT 1")
            .fetch_optional(&mut *tx)
            .await?;

    if existing.is_some() {
        tx.rollback().await?;
        return Ok(false);
    }

    let admin_user_id = Uuid::new_v4().to_string();
    let password_hash = hash_password(password)?;

    sqlx::query(
        "INSERT INTO cn_admin.admin_users          (admin_user_id, username, password_hash, is_active)          VALUES ($1, $2, $3, TRUE)",
    )
    .bind(&admin_user_id)
    .bind(username)
    .bind(&password_hash)
    .execute(&mut *tx)
    .await?;

    let diff = json!({"created": true, "username": username});
    sqlx::query(
        "INSERT INTO cn_admin.audit_logs          (actor_admin_user_id, action, target, diff_json, request_id)          VALUES ($1, $2, $3, $4, $5)",
    )
    .bind("system")
    .bind("admin.bootstrap")
    .bind(format!("admin_user:{username}"))
    .bind(Json(diff))
    .bind("bootstrap")
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(true)
}

pub async fn reset_admin_password(
    pool: &Pool<Postgres>,
    username: &str,
    password: &str,
) -> Result<()> {
    if username.trim().is_empty() {
        return Err(anyhow!("username is required"));
    }
    if password.is_empty() {
        return Err(anyhow!("password is required"));
    }

    let password_hash = hash_password(password)?;

    let result =
        sqlx::query("UPDATE cn_admin.admin_users SET password_hash = $1 WHERE username = $2")
            .bind(password_hash)
            .bind(username)
            .execute(pool)
            .await?;

    if result.rows_affected() == 0 {
        return Err(anyhow!("admin user not found"));
    }

    let diff = json!({"reset": true, "username": username});
    sqlx::query(
        "INSERT INTO cn_admin.audit_logs          (actor_admin_user_id, action, target, diff_json, request_id)          VALUES ($1, $2, $3, $4, $5)",
    )
    .bind("system")
    .bind("admin.reset_password")
    .bind(format!("admin_user:{username}"))
    .bind(Json(diff))
    .bind("reset")
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn seed_service_configs(pool: &Pool<Postgres>) -> Result<Vec<String>> {
    let descriptor_http_url = bootstrap_descriptor_http_url();
    let descriptor_ws_url = bootstrap_descriptor_ws_url();
    let seeds = vec![
        (
            "relay",
            json!({
                "auth": {
                    "mode": "off",
                    "enforce_at": null,
                    "grace_seconds": 900,
                    "ws_auth_timeout_seconds": 10
                },
                "limits": {
                    "max_event_bytes": 32768,
                    "max_tags": 200
                },
                "rate_limit": {
                    "enabled": true,
                    "ws": {
                        "events_per_minute": 120,
                        "reqs_per_minute": 60,
                        "conns_per_minute": 30
                    }
                },
                "retention": {
                    "events_days": 30,
                    "tombstone_days": 180,
                    "dedupe_days": 180,
                    "outbox_days": 30,
                    "cleanup_interval_seconds": 3600
                }
            }),
        ),
        (
            "user-api",
            json!({
                "rate_limit": {
                    "enabled": true,
                    "auth_per_minute": 20,
                    "public_per_minute": 120,
                    "protected_per_minute": 120
                }
            }),
        ),
        (
            "admin-api",
            json!({"session_cookie": true, "session_ttl_seconds": 86400}),
        ),
        (
            "index",
            json!({
                "enabled": true,
                "consumer": { "batch_size": 200, "poll_interval_seconds": 5 },
                "reindex": { "poll_interval_seconds": 30 },
                "expiration": { "sweep_interval_seconds": 300 }
            }),
        ),
        (
            "moderation",
            json!({
                "enabled": false,
                "consumer": { "batch_size": 200, "poll_interval_seconds": 5 },
                "queue": { "max_attempts": 3, "retry_delay_seconds": 30 },
                "rules": { "max_labels_per_event": 5 },
                "llm": {
                    "enabled": false,
                    "provider": "disabled",
                    "external_send_enabled": false,
                    "send_scope": {
                        "public": true,
                        "invite": false,
                        "friend": false,
                        "friend_plus": false
                    },
                    "storage": {
                        "persist_decisions": true,
                        "persist_request_snapshots": false
                    },
                    "retention": {
                        "decision_days": 90,
                        "snapshot_days": 7
                    },
                    "truncate_chars": 2000,
                    "mask_pii": true,
                    "max_requests_per_day": 0,
                    "max_cost_per_day": 0.0,
                    "max_concurrency": 1
                }
            }),
        ),
        (
            "trust",
            json!({
                "enabled": false,
                "consumer": { "batch_size": 200, "poll_interval_seconds": 5 },
                "report_based": {
                    "window_days": 30,
                    "report_weight": 1.0,
                    "label_weight": 1.0,
                    "score_normalization": 10.0
                },
                "communication_density": {
                    "window_days": 30,
                    "score_normalization": 20.0,
                    "interaction_weights": {
                        "1": 1.0,
                        "6": 0.5,
                        "7": 0.3
                    }
                },
                "assertion": { "exp_seconds": 86400 },
                "jobs": {
                    "schedule_poll_seconds": 30,
                    "report_based_interval_seconds": 86400,
                    "communication_interval_seconds": 86400
                }
            }),
        ),
    ];

    let mut inserted = Vec::new();
    for (service, config_json) in seeds {
        let result = sqlx::query(
            "INSERT INTO cn_admin.service_configs \
             (service, version, config_json, updated_by) \
             VALUES ($1, 1, $2, $3) \
             ON CONFLICT (service) DO NOTHING",
        )
        .bind(service)
        .bind(Json(config_json))
        .bind("system")
        .execute(pool)
        .await?;

        if result.rows_affected() > 0 {
            inserted.push(service.to_string());
        }
    }

    let bootstrap_rows_affected = if should_sync_bootstrap_descriptor() {
        upsert_bootstrap_descriptor_config(pool, &descriptor_http_url, &descriptor_ws_url).await?
    } else {
        sqlx::query(
            "INSERT INTO cn_admin.service_configs \
             (service, version, config_json, updated_by) \
             VALUES ($1, 1, $2, $3) \
             ON CONFLICT (service) DO NOTHING",
        )
        .bind("bootstrap")
        .bind(Json(default_bootstrap_service_config(
            &descriptor_http_url,
            &descriptor_ws_url,
        )))
        .bind("system")
        .execute(pool)
        .await?
        .rows_affected()
    };
    if bootstrap_rows_affected > 0 {
        inserted.push("bootstrap".to_string());
    }

    Ok(inserted)
}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| anyhow!("argon2 hash failed: {err}"))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(password: &str, hashed: &str) -> Result<bool> {
    let parsed =
        PasswordHash::new(hashed).map_err(|err| anyhow!("invalid password hash: {err}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub async fn log_audit(
    pool: &Pool<Postgres>,
    actor: &str,
    action: &str,
    target: &str,
    diff: Option<serde_json::Value>,
    request_id: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_admin.audit_logs          (actor_admin_user_id, action, target, diff_json, request_id)          VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(actor)
    .bind(action)
    .bind(target)
    .bind(diff.map(Json))
    .bind(request_id)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        bootstrap_descriptor_http_url, bootstrap_descriptor_ws_url,
        merge_bootstrap_descriptor_config,
    };
    use serde_json::json;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn with_env(vars: &[(&str, Option<&str>)], f: impl FnOnce()) {
        let _guard = env_lock().lock().expect("env lock");
        let saved = vars
            .iter()
            .map(|(name, _)| ((*name).to_string(), std::env::var(name).ok()))
            .collect::<Vec<_>>();

        for (name, value) in vars {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }

        f();

        for (name, value) in saved {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }

    #[test]
    fn bootstrap_descriptor_http_url_uses_public_base_url_when_override_missing() {
        with_env(
            &[
                ("BOOTSTRAP_DESCRIPTOR_HTTP_URL", None),
                ("PUBLIC_BASE_URL", Some("https://api.kukuri.app/")),
            ],
            || {
                assert_eq!(bootstrap_descriptor_http_url(), "https://api.kukuri.app");
            },
        );
    }

    #[test]
    fn bootstrap_descriptor_ws_url_uses_relay_public_url_when_override_missing() {
        with_env(
            &[
                ("BOOTSTRAP_DESCRIPTOR_WS_URL", None),
                ("RELAY_PUBLIC_URL", Some("wss://relay.kukuri.app/relay/")),
            ],
            || {
                assert_eq!(
                    bootstrap_descriptor_ws_url(),
                    "wss://relay.kukuri.app/relay"
                );
            },
        );
    }

    #[test]
    fn merge_bootstrap_descriptor_config_preserves_existing_non_descriptor_fields() {
        let existing = json!({
            "auth": {
                "mode": "required",
                "grace_seconds": 120
            },
            "descriptor": {
                "name": "Existing",
                "roles": ["bootstrap"],
                "endpoints": {
                    "http": "http://localhost:8080",
                    "ws": "ws://localhost:8082/relay"
                }
            },
            "exp": {
                "descriptor_days": 14,
                "topic_hours": 96
            }
        });

        let merged = merge_bootstrap_descriptor_config(
            existing,
            "https://api.kukuri.app",
            "wss://relay.kukuri.app/relay",
        );

        assert_eq!(merged["auth"]["mode"], json!("required"));
        assert_eq!(merged["auth"]["grace_seconds"], json!(120));
        assert_eq!(merged["exp"]["descriptor_days"], json!(14));
        assert_eq!(
            merged["descriptor"]["endpoints"]["http"],
            json!("https://api.kukuri.app")
        );
        assert_eq!(
            merged["descriptor"]["endpoints"]["ws"],
            json!("wss://relay.kukuri.app/relay")
        );
    }
}
