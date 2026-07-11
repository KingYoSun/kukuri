use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use kukuri_cn_cli::{format_timestamp, parse_enforce_at};
use kukuri_cn_core::{
    AdmissionMode, add_allowlist, ban_subscriber, initialize_database, issue_invite_code,
    list_allowlist, list_banned, list_invite_codes, load_admission_config, remove_allowlist,
    revoke_invite_code, set_admission_mode, unban_subscriber,
};

use crate::{AdmissionAction, AllowAction, BanAction, InviteAction};

pub(super) async fn run(pool: &PgPool, action: AdmissionAction) -> Result<()> {
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
        AdmissionAction::Invite { action } => run_invite(pool, action).await?,
        AdmissionAction::Allow { action } => run_allow(pool, action).await?,
        AdmissionAction::Ban { action } => run_ban(pool, action).await?,
    }
    Ok(())
}

async fn run_invite(pool: &PgPool, action: InviteAction) -> Result<()> {
    match action {
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
    }
    Ok(())
}

async fn run_allow(pool: &PgPool, action: AllowAction) -> Result<()> {
    match action {
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
    }
    Ok(())
}

async fn run_ban(pool: &PgPool, action: BanAction) -> Result<()> {
    match action {
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
    }
    Ok(())
}
