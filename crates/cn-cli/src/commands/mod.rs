use anyhow::Result;
use sqlx::PgPool;

use crate::Command;

mod admission;
mod database;
mod indexing;
mod moderation;
mod relation;
mod reports;

pub(crate) async fn dispatch(pool: &PgPool, command: Command) -> Result<()> {
    match command {
        Command::Prepare => database::prepare(pool).await,
        Command::Migrate => database::migrate(pool).await,
        Command::SeedPolicies => database::seed_policies(pool).await,
        Command::SetAuthRollout {
            service,
            mode,
            enforce_at,
            grace_seconds,
            ws_auth_timeout_seconds,
        } => {
            database::set_auth_rollout(
                pool,
                service,
                mode,
                enforce_at,
                grace_seconds,
                ws_auth_timeout_seconds,
            )
            .await
        }
        Command::Reports { action } => reports::run(pool, action).await,
        Command::Admission { action } => admission::run(pool, action).await,
        Command::SupportedTopic { action } => indexing::run_supported_topic(pool, action).await,
        Command::IndexingRequest { action } => indexing::run_indexing_request(pool, action).await,
        Command::Relation { action } => relation::run(pool, action).await,
        Command::Moderation { action } => moderation::run(pool, action).await,
    }
}
