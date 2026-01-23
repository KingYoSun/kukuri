use anyhow::{anyhow, Result};
use argon2::password_hash::{PasswordHasher, SaltString};
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
    let existing = sqlx::query_scalar::<_, String>(
        "SELECT admin_user_id FROM cn_admin.admin_users LIMIT 1",
    )
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

    let result = sqlx::query(
        "UPDATE cn_admin.admin_users SET password_hash = $1 WHERE username = $2",
    )
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
        ("relay", json!({"auth_required": false})),
        ("bootstrap", json!({"auth_required": false})),
        ("user-api", json!({"rate_limit": {"enabled": false}})),
        ("admin-api", json!({"session_cookie": true})),
        ("index", json!({"enabled": false})),
        ("moderation", json!({"enabled": false})),
        ("trust", json!({"enabled": false})),
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

fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| anyhow!("argon2 hash failed: {}", err))?
        .to_string();
    Ok(hash)
}
