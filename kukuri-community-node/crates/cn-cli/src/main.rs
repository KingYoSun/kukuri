use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cn", version, about = "Kukuri community node CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    UserApi,
    AdminApi,
    Relay,
    Bootstrap,
    Migrate,
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    Admin {
        #[command(subcommand)]
        command: AdminCommand,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    Seed,
}

#[derive(Subcommand)]
enum AdminCommand {
    Bootstrap {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password: String,
    },
    ResetPassword {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::UserApi => {
            let config = cn_user_api::load_config()?;
            cn_user_api::run(config).await?;
        }
        Commands::AdminApi => {
            let config = cn_admin_api::load_config()?;
            cn_admin_api::run(config).await?;
        }
        Commands::Relay => {
            let config = cn_relay::load_config()?;
            cn_relay::run(config).await?;
        }
        Commands::Bootstrap => {
            let config = cn_bootstrap::load_config()?;
            cn_bootstrap::run(config).await?;
        }
        Commands::Migrate => {
            cn_core::logging::init("cn-cli");
            let database_url = cn_core::config::required_env("DATABASE_URL")?;
            let pool = cn_core::db::connect(&database_url).await?;
            cn_core::migrations::run(&pool).await?;
            tracing::info!("migrations applied");
        }
        Commands::Config { command } => {
            cn_core::logging::init("cn-cli");
            let database_url = cn_core::config::required_env("DATABASE_URL")?;
            let pool = cn_core::db::connect(&database_url).await?;
            match command {
                ConfigCommand::Seed => {
                    let seeded = cn_core::admin::seed_service_configs(&pool).await?;
                    if seeded.is_empty() {
                        tracing::info!("no new service configs were inserted");
                    } else {
                        tracing::info!(services = ?seeded, "service configs seeded");
                    }
                }
            }
        }
        Commands::Admin { command } => {
            cn_core::logging::init("cn-cli");
            let database_url = cn_core::config::required_env("DATABASE_URL")?;
            let pool = cn_core::db::connect(&database_url).await?;
            match command {
                AdminCommand::Bootstrap { username, password } => {
                    let created = cn_core::admin::bootstrap_admin(&pool, &username, &password)
                        .await?;
                    if created {
                        tracing::info!("admin user created");
                    } else {
                        tracing::info!("admin user already exists");
                    }
                }
                AdminCommand::ResetPassword { username, password } => {
                    cn_core::admin::reset_admin_password(&pool, &username, &password).await?;
                    tracing::info!("admin password reset");
                }
            }
        }
    }

    Ok(())
}
