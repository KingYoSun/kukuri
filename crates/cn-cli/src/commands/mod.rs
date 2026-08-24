use anyhow::Result;
use sqlx::PgPool;

use crate::Command;

mod admission;
mod database;
mod indexing;
pub(crate) mod moderation;
mod readiness;
mod readiness_runtime;
mod relation;
mod reports;
mod rights_requests;
mod transmission_prevention;

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
        Command::RightsRequests { action } => rights_requests::run(pool, action).await,
        Command::Admission { action } => admission::run(pool, action).await,
        Command::SupportedTopic { action } => indexing::run_supported_topic(pool, action).await,
        Command::IndexingRequest { action } => indexing::run_indexing_request(pool, action).await,
        Command::Relation { action } => relation::run(pool, action).await,
        Command::Moderation { action } => moderation::run(pool, action).await,
        Command::TransmissionPrevention { action } => {
            transmission_prevention::run(pool, action).await
        }
        Command::Readiness {
            config,
            profile,
            probe_ttl_secs,
            force_probe,
            indexer_status_url,
            ingest_max_age_secs,
            relation_max_age_secs,
        } => {
            readiness::run(
                pool,
                &config,
                &profile,
                probe_ttl_secs,
                force_probe,
                &indexer_status_url,
                ingest_max_age_secs,
                relation_max_age_secs,
            )
            .await
        }
    }
}
