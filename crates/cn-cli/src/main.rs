use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand, ValueEnum};
use kukuri_cn_core::{
    AdmissionMode, AuthMode, AuthRolloutConfig, COMMUNITY_NODE_AUTH_SERVICE_NAME, add_allowlist,
    ban_subscriber, connect_postgres, get_community_node_report, initialize_database,
    issue_invite_code, list_allowlist, list_banned, list_community_node_reports, list_invite_codes,
    load_admission_config, migrate_postgres, remove_allowlist, revoke_invite_code,
    seed_default_policies, set_admission_mode, store_auth_rollout, unban_subscriber,
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
    /// 入会制御（招待 / whitelist / ban）を運用する（#383）。
    Admission {
        #[command(subcommand)]
        action: AdmissionAction,
    },
}

#[derive(Debug, Subcommand)]
enum AdmissionAction {
    /// 現在の入会モードを表示する。
    Show,
    /// 入会モードを設定する（open / invite / whitelist）。
    SetMode {
        #[arg(long)]
        mode: AdmissionModeArg,
    },
    /// 招待コードを操作する。
    Invite {
        #[command(subcommand)]
        action: InviteAction,
    },
    /// 手動許可リスト（whitelist）を操作する。
    Allow {
        #[command(subcommand)]
        action: AllowAction,
    },
    /// ブラックリスト（ban）を操作する。
    Ban {
        #[command(subcommand)]
        action: BanAction,
    },
}

#[derive(Debug, Subcommand)]
enum InviteAction {
    /// 招待コードを発行する（平文はこのとき一度だけ表示される）。
    Issue {
        /// 任意のラベル。
        #[arg(long)]
        label: Option<String>,
        /// 使用可能回数。未指定なら無制限。
        #[arg(long)]
        max_uses: Option<i32>,
        /// 失効時刻（RFC3339 または epoch 秒）。未指定なら無期限。
        #[arg(long)]
        expires_at: Option<String>,
    },
    /// 発行済み招待コードを新着順で一覧する（hash のみ表示）。
    List,
    /// 招待コード（平文）を取り消す。
    Revoke {
        #[arg(long)]
        code: String,
    },
}

#[derive(Debug, Subcommand)]
enum AllowAction {
    /// pubkey を許可リストへ追加する。
    Add {
        #[arg(long)]
        pubkey: String,
        #[arg(long)]
        label: Option<String>,
    },
    /// pubkey を許可リストから削除する。
    Remove {
        #[arg(long)]
        pubkey: String,
    },
    /// 許可リストを一覧する。
    List,
}

#[derive(Debug, Subcommand)]
enum BanAction {
    /// pubkey を ban する（既存利用者の token も即時失効する）。
    Add {
        #[arg(long)]
        pubkey: String,
    },
    /// pubkey の ban を解除する。
    Remove {
        #[arg(long)]
        pubkey: String,
    },
    /// ban 済み pubkey を一覧する。
    List,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum AdmissionModeArg {
    Open,
    Invite,
    Whitelist,
}

impl From<AdmissionModeArg> for AdmissionMode {
    fn from(value: AdmissionModeArg) -> Self {
        match value {
            AdmissionModeArg::Open => AdmissionMode::Open,
            AdmissionModeArg::Invite => AdmissionMode::Invite,
            AdmissionModeArg::Whitelist => AdmissionMode::Whitelist,
        }
    }
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
        Command::Admission { action } => {
            run_admission(&pool, action).await?;
        }
    }

    Ok(())
}

async fn run_admission(pool: &sqlx::PgPool, action: AdmissionAction) -> Result<()> {
    // admission state を読み書きするテーブルが揃っていることを保証する。
    initialize_database(pool).await?;
    match action {
        AdmissionAction::Show => {
            let config = load_admission_config(pool).await?;
            println!("admission mode: {}", config.mode.as_str());
        }
        AdmissionAction::SetMode { mode } => {
            let mode = AdmissionMode::from(mode);
            set_admission_mode(pool, mode).await?;
            println!("admission mode updated: {}", mode.as_str());
        }
        AdmissionAction::Invite { action } => match action {
            InviteAction::Issue {
                label,
                max_uses,
                expires_at,
            } => {
                let expires_at = expires_at
                    .map(|value| parse_enforce_at(value.as_str()))
                    .transpose()?
                    .map(|timestamp| {
                        DateTime::<Utc>::from_timestamp(timestamp, 0)
                            .context("invalid expires_at timestamp")
                    })
                    .transpose()?;
                let code = issue_invite_code(pool, label.as_deref(), max_uses, expires_at).await?;
                println!("invite code issued (store it now; it will not be shown again):");
                println!("{code}");
            }
            InviteAction::List => {
                let codes = list_invite_codes(pool).await?;
                if codes.is_empty() {
                    println!("no invite codes");
                } else {
                    println!("{} invite code(s):", codes.len());
                    for code in codes {
                        let max_uses = code
                            .max_uses
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "unlimited".to_string());
                        let expires_at = code
                            .expires_at
                            .map(format_timestamp)
                            .unwrap_or_else(|| "never".to_string());
                        let revoked_at = code
                            .revoked_at
                            .map(format_timestamp)
                            .unwrap_or_else(|| "-".to_string());
                        println!(
                            "{}  uses={}/{}  expires={}  revoked={}  label={}",
                            code.code_hash,
                            code.used_count,
                            max_uses,
                            expires_at,
                            revoked_at,
                            code.label.as_deref().unwrap_or("-"),
                        );
                    }
                }
            }
            InviteAction::Revoke { code } => {
                if revoke_invite_code(pool, code.as_str()).await? {
                    println!("invite code revoked");
                } else {
                    println!("invite code not found or already revoked");
                }
            }
        },
        AdmissionAction::Allow { action } => match action {
            AllowAction::Add { pubkey, label } => {
                add_allowlist(pool, pubkey.as_str(), label.as_deref()).await?;
                println!("allowlisted {pubkey}");
            }
            AllowAction::Remove { pubkey } => {
                if remove_allowlist(pool, pubkey.as_str()).await? {
                    println!("removed {pubkey} from allowlist");
                } else {
                    println!("{pubkey} was not on the allowlist");
                }
            }
            AllowAction::List => {
                let entries = list_allowlist(pool).await?;
                if entries.is_empty() {
                    println!("allowlist is empty");
                } else {
                    println!("{} allowlisted pubkey(s):", entries.len());
                    for entry in entries {
                        println!(
                            "{}  created={}  label={}",
                            entry.pubkey,
                            format_timestamp(entry.created_at),
                            entry.label.as_deref().unwrap_or("-"),
                        );
                    }
                }
            }
        },
        AdmissionAction::Ban { action } => match action {
            BanAction::Add { pubkey } => {
                ban_subscriber(pool, pubkey.as_str()).await?;
                println!("banned {pubkey}");
            }
            BanAction::Remove { pubkey } => {
                if unban_subscriber(pool, pubkey.as_str()).await? {
                    println!("unbanned {pubkey}");
                } else {
                    println!("{pubkey} was not banned");
                }
            }
            BanAction::List => {
                let entries = list_banned(pool).await?;
                if entries.is_empty() {
                    println!("no banned subscribers");
                } else {
                    println!("{} banned subscriber(s):", entries.len());
                    for entry in entries {
                        println!(
                            "{}  created={}",
                            entry.pubkey,
                            format_timestamp(entry.created_at)
                        );
                    }
                }
            }
        },
    }
    Ok(())
}

fn format_timestamp(timestamp: i64) -> String {
    DateTime::<Utc>::from_timestamp(timestamp, 0)
        .map(|value| value.to_rfc3339())
        .unwrap_or_else(|| timestamp.to_string())
}

fn parse_enforce_at(value: &str) -> Result<i64> {
    if let Ok(timestamp) = value.parse::<i64>() {
        return Ok(timestamp);
    }
    let parsed = DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("failed to parse RFC3339 timestamp `{value}`"))?;
    Ok(parsed.with_timezone(&Utc).timestamp())
}
