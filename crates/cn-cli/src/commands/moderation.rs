//! moderation advisory（risk signal）の appeal / operator レビュー運用（#420 / ADR 0028）。
//!
//! `edit` / `reissue` は operator レビュー機能。env
//! `COMMUNITY_NODE_SAFETY_OPERATOR_REVIEW=true` の明示的有効化が無ければ cn-core 層で
//! 拒否される（optional 機能の fail-closed）。是正は node-local advisory に対して行い、
//! user の canonical state は変更しない。

use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;

use kukuri_cn_core::{
    RiskSignalCorrection, RiskSignalMetadataEdit, StoredRiskSignal, dispute_risk_signal,
    edit_risk_signal_detection_metadata, get_risk_signal, list_risk_signals, parse_bool_env,
    reissue_corrected_risk_signal, update_risk_signal_appeal_status,
};
use kukuri_cn_safety::{AppealStatus, SafetyCategory, Severity, Visibility};

use crate::{ModerationCliAction, SafetyCategoryArg, SeverityArg, VisibilityArg};

/// operator レビュー有効化フラグの env 名（operator config
/// `safety.moderation.operator_review` に対応）。
pub(crate) const OPERATOR_REVIEW_ENV: &str = "COMMUNITY_NODE_SAFETY_OPERATOR_REVIEW";

pub(super) async fn run(pool: &PgPool, action: ModerationCliAction) -> Result<()> {
    match action {
        ModerationCliAction::ListSignals { limit, offset } => {
            let signals = list_risk_signals(pool, limit, offset).await?;
            if signals.is_empty() {
                println!("no risk signals");
            } else {
                println!(
                    "{} risk signal(s) (limit={} offset={}):",
                    signals.len(),
                    limit,
                    offset
                );
                for stored in signals {
                    println!(
                        "{}  {}  {:?}/{}  category={:?}  severity={:?}  basis={:?}  \
                         visibility={:?}  appeal={:?}  expires_at={}",
                        stored.persisted_at.to_rfc3339(),
                        stored.id,
                        stored.signal.target,
                        stored.signal.target_id,
                        stored.signal.category,
                        stored.signal.severity,
                        stored.signal.basis,
                        stored.signal.visibility,
                        stored.signal.appeal_status.unwrap_or_default(),
                        stored.signal.expires_at.as_deref().unwrap_or("-"),
                    );
                }
            }
        }
        ModerationCliAction::Show { id } => match get_risk_signal(pool, &id).await? {
            Some(stored) => print_signal(&stored),
            None => println!("risk signal not found: {id}"),
        },
        ModerationCliAction::Dispute { id } => {
            let stored = dispute_risk_signal(pool, &id).await?;
            println!("disputed:");
            print_signal(&stored);
        }
        ModerationCliAction::Clear { id } => {
            let stored = update_risk_signal_appeal_status(pool, &id, AppealStatus::Cleared).await?;
            println!("cleared (trust contribution reverts on the next read):");
            print_signal(&stored);
        }
        ModerationCliAction::Reject { id } => {
            let stored = update_risk_signal_appeal_status(pool, &id, AppealStatus::None).await?;
            println!("rejected (contribution retained):");
            print_signal(&stored);
        }
        ModerationCliAction::Edit {
            id,
            category,
            severity,
            confidence,
            expires_at,
        } => {
            let edit = RiskSignalMetadataEdit {
                category: category.map(category_from_arg),
                severity: severity.map(severity_from_arg),
                confidence,
                expires_at,
            };
            let enabled = parse_bool_env(OPERATOR_REVIEW_ENV, false)?;
            let stored = edit_risk_signal_detection_metadata(pool, &id, &edit, enabled).await?;
            println!("edited:");
            print_signal(&stored);
        }
        ModerationCliAction::Reissue {
            id,
            category,
            severity,
            confidence,
            visibility,
        } => {
            let correction = RiskSignalCorrection {
                category: category.map(category_from_arg),
                severity: severity.map(severity_from_arg),
                confidence,
                visibility: visibility.map(visibility_from_arg),
            };
            let enabled = parse_bool_env(OPERATOR_REVIEW_ENV, false)?;
            let now = Utc::now().to_rfc3339();
            let stored =
                reissue_corrected_risk_signal(pool, &id, &correction, &now, enabled).await?;
            println!("reissued (previous signal `{id}` expired at {now}):");
            print_signal(&stored);
        }
    }
    Ok(())
}

fn print_signal(stored: &StoredRiskSignal) {
    println!("id:            {}", stored.id);
    println!("issuer:        {}", stored.issuer_node_id);
    println!("persisted_at:  {}", stored.persisted_at.to_rfc3339());
    println!(
        "target:        {:?}/{}",
        stored.signal.target, stored.signal.target_id
    );
    println!("category:      {:?}", stored.signal.category);
    println!("severity:      {:?}", stored.signal.severity);
    println!("basis:         {:?}", stored.signal.basis);
    println!("visibility:    {:?}", stored.signal.visibility);
    println!(
        "confidence:    {}",
        stored
            .signal
            .confidence
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "appeal_status: {:?}",
        stored.signal.appeal_status.unwrap_or_default()
    );
    println!(
        "expires_at:    {}",
        stored.signal.expires_at.as_deref().unwrap_or("-")
    );
}

fn category_from_arg(arg: SafetyCategoryArg) -> SafetyCategory {
    match arg {
        SafetyCategoryArg::Csam => SafetyCategory::Csam,
        SafetyCategoryArg::Cse => SafetyCategory::Cse,
        SafetyCategoryArg::Grooming => SafetyCategory::Grooming,
        SafetyCategoryArg::Nsfw => SafetyCategory::Nsfw,
        SafetyCategoryArg::Spam => SafetyCategory::Spam,
        SafetyCategoryArg::Malware => SafetyCategory::Malware,
        SafetyCategoryArg::Phishing => SafetyCategory::Phishing,
    }
}

fn severity_from_arg(arg: SeverityArg) -> Severity {
    match arg {
        SeverityArg::Critical => Severity::Critical,
        SeverityArg::High => Severity::High,
        SeverityArg::Medium => Severity::Medium,
        SeverityArg::Low => Severity::Low,
    }
}

fn visibility_from_arg(arg: VisibilityArg) -> Visibility {
    match arg {
        VisibilityArg::Local => Visibility::Local,
        VisibilityArg::SubscribedNodes => Visibility::SubscribedNodes,
        VisibilityArg::Public => Visibility::Public,
    }
}
