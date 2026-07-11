use anyhow::Result;
use sqlx::PgPool;

use kukuri_cn_cli::parse_enforce_at;
use kukuri_cn_core::{
    AuthMode, AuthRolloutConfig, initialize_database, migrate_postgres, seed_default_policies,
    store_auth_rollout,
};

use crate::AuthModeArg;

pub(super) async fn prepare(pool: &PgPool) -> Result<()> {
    initialize_database(pool).await?;
    println!("database prepared");
    Ok(())
}

pub(super) async fn migrate(pool: &PgPool) -> Result<()> {
    migrate_postgres(pool).await?;
    println!("migrations applied");
    Ok(())
}

pub(super) async fn seed_policies(pool: &PgPool) -> Result<()> {
    initialize_database(pool).await?;
    seed_default_policies(pool).await?;
    println!("policies seeded");
    Ok(())
}

pub(super) async fn set_auth_rollout(
    pool: &PgPool,
    service: String,
    mode: AuthModeArg,
    enforce_at: Option<String>,
    grace_seconds: i64,
    ws_auth_timeout_seconds: i64,
) -> Result<()> {
    migrate_postgres(pool).await?;
    let rollout = AuthRolloutConfig {
        mode: match mode {
            AuthModeArg::Off => AuthMode::Off,
            AuthModeArg::Required => AuthMode::Required,
        },
        enforce_at: enforce_at
            .map(|value| parse_enforce_at(value.as_str()))
            .transpose()?,
        grace_seconds,
        ws_auth_timeout_seconds,
    };
    store_auth_rollout(pool, service.as_str(), &rollout).await?;
    println!(
        "auth rollout updated service={} mode={:?} enforce_at={:?} grace_seconds={} ws_auth_timeout_seconds={}",
        service,
        rollout.mode,
        rollout.enforce_at,
        rollout.grace_seconds,
        rollout.ws_auth_timeout_seconds
    );
    Ok(())
}
