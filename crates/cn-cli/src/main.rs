use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand, ValueEnum};
use kukuri_cn_core::{
    AuthMode, AuthRolloutConfig, COMMUNITY_NODE_AUTH_SERVICE_NAME, connect_postgres,
    get_community_node_report, initialize_database, list_community_node_reports, migrate_postgres,
    seed_default_policies, store_auth_rollout,
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
    Prepare,
    Migrate,
    SeedPolicies,
    SetAuthRollout {
        #[arg(long, default_value = COMMUNITY_NODE_AUTH_SERVICE_NAME)]
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
    /// 受信した通報（#370）を確認する。
    Reports {
        #[command(subcommand)]
        action: ReportsAction,
    },
}

#[derive(Debug, Subcommand)]
enum ReportsAction {
    /// 受信した通報を新着順で一覧する。
    List {
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long, default_value_t = 0)]
        offset: i64,
    },
    /// 単一の通報を ID で表示する。
    Show {
        #[arg(long)]
        id: String,
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
        Command::Prepare => {
            initialize_database(&pool).await?;
            println!("database prepared");
        }
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
        Command::Reports { action } => match action {
            ReportsAction::List { limit, offset } => {
                let reports = list_community_node_reports(&pool, limit, offset).await?;
                if reports.is_empty() {
                    println!("no reports");
                } else {
                    println!(
                        "{} report(s) (limit={} offset={}):",
                        reports.len(),
                        limit,
                        offset
                    );
                    for report in reports {
                        println!(
                            "{}  {}  {}/{}  capability={}  reason={}  status={}",
                            report.created_at.to_rfc3339(),
                            report.id,
                            report.subject_kind,
                            report.subject_id,
                            report.capability,
                            report.reason,
                            report.status,
                        );
                    }
                }
            }
            ReportsAction::Show { id } => {
                match get_community_node_report(&pool, id.as_str()).await? {
                    Some(report) => {
                        println!("id:               {}", report.id);
                        println!("created_at:       {}", report.created_at.to_rfc3339());
                        println!("status:           {}", report.status);
                        println!("subject_kind:     {}", report.subject_kind);
                        println!("subject_id:       {}", report.subject_id);
                        println!("capability:       {}", report.capability);
                        println!("reason:           {}", report.reason);
                        println!(
                            "details:          {}",
                            report.details.as_deref().unwrap_or("-")
                        );
                        println!(
                            "reporter_contact: {}",
                            report.reporter_contact.as_deref().unwrap_or("-")
                        );
                    }
                    None => println!("report not found: {id}"),
                }
            }
        },
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
