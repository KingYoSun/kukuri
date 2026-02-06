use anyhow::{anyhow, Result};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand_core::OsRng;
use serde_json::json;
use sqlx::types::Json;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

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
    .bind(format!("admin_user:{}", username))
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
    .bind(format!("admin_user:{}", username))
    .bind(Json(diff))
    .bind("reset")
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn seed_service_configs(pool: &Pool<Postgres>) -> Result<Vec<String>> {
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
            "bootstrap",
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
                        "http": "http://localhost:8080",
                        "ws": "ws://localhost:8082/relay"
                    },
                    "policy_url": "",
                    "jurisdiction": "",
                    "contact": ""
                },
                "exp": {
                    "descriptor_days": 7,
                    "topic_hours": 48
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
                "attestation": { "exp_seconds": 86400 },
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
            "INSERT INTO cn_admin.service_configs              (service, version, config_json, updated_by)              VALUES ($1, 1, $2, $3)              ON CONFLICT (service) DO NOTHING",
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

    Ok(inserted)
}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| anyhow!("argon2 hash failed: {}", err))?
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
