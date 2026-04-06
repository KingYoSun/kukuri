use anyhow::Result;
use serde_json::Value;
use sqlx::postgres::PgPool;

use crate::{AuthRolloutConfig, COMMUNITY_NODE_AUTH_SERVICE_NAME};

pub async fn ensure_default_auth_rollout(pool: &PgPool) -> Result<()> {
    let config_json = serde_json::to_value(AuthRolloutConfig::default())?;
    sqlx::query(
        "INSERT INTO cn_admin.service_configs (service_name, version, config_json)
         VALUES ($1, 1, $2)
         ON CONFLICT (service_name) DO NOTHING",
    )
    .bind(COMMUNITY_NODE_AUTH_SERVICE_NAME)
    .bind(config_json)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn load_auth_rollout(pool: &PgPool, service_name: &str) -> Result<AuthRolloutConfig> {
    let value = sqlx::query_scalar::<_, Value>(
        "SELECT config_json FROM cn_admin.service_configs WHERE service_name = $1",
    )
    .bind(service_name)
    .fetch_optional(pool)
    .await?;
    match value {
        Some(value) => Ok(serde_json::from_value(value).unwrap_or_default()),
        None => Ok(AuthRolloutConfig::default()),
    }
}

pub async fn store_auth_rollout(
    pool: &PgPool,
    service_name: &str,
    rollout: &AuthRolloutConfig,
) -> Result<()> {
    let config_json = serde_json::to_value(rollout)?;
    sqlx::query(
        "INSERT INTO cn_admin.service_configs (service_name, version, config_json)
         VALUES ($1, 1, $2)
         ON CONFLICT (service_name) DO UPDATE
         SET version = cn_admin.service_configs.version + 1,
             config_json = EXCLUDED.config_json,
             updated_at = NOW()",
    )
    .bind(service_name)
    .bind(config_json)
    .execute(pool)
    .await?;
    Ok(())
}
