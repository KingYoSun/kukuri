use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand, ValueEnum};
use kukuri_cn_core::{
    AuthMode, AuthRolloutConfig, RELAY_SERVICE_NAME, connect_postgres, initialize_database,
    migrate_postgres, seed_default_policies, store_auth_rollout,
};

#[derive(Debug, Parser)]
struct Cli {
    #[arg(long, env = "COMMUNITY_NODE_DATABASE_URL")]
    database_url: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Migrate,
    SeedPolicies,
    SetAuthRollout {
        #[arg(long, default_value = RELAY_SERVICE_NAME)]
        service: String,
        #[arg(long)]
        mode: AuthModeArg,
        #[arg(long)]
        enforce_at: Option<String>,
        #[arg(long, default_value_t = 900)]
        grace_seconds: i64,
        #[arg(long, default_value_t = 10)]
        ws_auth_timeout_seconds: i64,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum AuthModeArg {
    Off,
    Required,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let pool = connect_postgres(cli.database_url.as_str()).await?;

    match cli.command {
        Command::Migrate => {
            migrate_postgres(&pool).await?;
            println!("migrations applied");
        }
        Command::SeedPolicies => {
            initialize_database(&pool).await?;
            seed_default_policies(&pool).await?;
            println!("policies seeded");
        }
        Command::SetAuthRollout {
            service,
            mode,
            enforce_at,
            grace_seconds,
            ws_auth_timeout_seconds,
        } => {
            migrate_postgres(&pool).await?;
            let enforce_at = enforce_at
                .map(|value| parse_enforce_at(value.as_str()))
                .transpose()?;
            let rollout = AuthRolloutConfig {
                mode: match mode {
                    AuthModeArg::Off => AuthMode::Off,
                    AuthModeArg::Required => AuthMode::Required,
                },
                enforce_at,
                grace_seconds,
                ws_auth_timeout_seconds,
            };
            store_auth_rollout(&pool, service.as_str(), &rollout).await?;
            println!(
                "auth rollout updated service={} mode={:?} enforce_at={:?} grace_seconds={} ws_auth_timeout_seconds={}",
                service,
                rollout.mode,
                rollout.enforce_at,
                rollout.grace_seconds,
                rollout.ws_auth_timeout_seconds
            );
        }
    }

    Ok(())
}

fn parse_enforce_at(value: &str) -> Result<i64> {
    if let Ok(timestamp) = value.parse::<i64>() {
        return Ok(timestamp);
    }
    let parsed = DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("failed to parse RFC3339 timestamp `{value}`"))?;
    Ok(parsed.with_timezone(&Utc).timestamp())
}
