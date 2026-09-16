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

/// この配備がリスク判定に載せる発行元識別子を標準出力へ 1 行で表示する(#706)。
///
/// 導出は cn-indexer の signer および cn-user-api の起動時検査と同じ
/// [`kukuri_cn_safety_runtime::expected_issuer_node_id_from_env`] を使う。
/// 署名鍵は env からしか読まず、出力には公開鍵 hex(または明示識別子)だけを含める。
pub(crate) fn print_issuer_node_id() -> Result<()> {
    let issuer = kukuri_cn_safety_runtime::expected_issuer_node_id_from_env()
        .map_err(|error| anyhow::anyhow!("{error}"))?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "発行元識別子を導出できません。COMMUNITY_NODE_SAFETY_SIGNING_KEY(署名鍵)または \
                 COMMUNITY_NODE_SAFETY_ISSUER_NODE_ID(署名無効時の明示識別子)を設定してください"
            )
        })?;
    println!("{issuer}");
    Ok(())
}

pub(super) async fn run(pool: &PgPool, action: ModerationCliAction) -> Result<()> {
    match action {
        ModerationCliAction::IssuerNodeId => print_issuer_node_id()?,
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
                         visibility={:?}  appeal={:?}  expires_at={}  operator_adjusted={}",
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
                        format_operator_adjustment(&stored),
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
    print!("{}", format_signal(stored));
}

/// `show` などの詳細表示。operator が値を確定した行は印と訂正前の category を示す（#1058）。
fn format_signal(stored: &StoredRiskSignal) -> String {
    let mut out = String::new();
    let mut line = |label: &str, value: String| {
        out.push_str(&format!("{:<15}{value}\n", format!("{label}:")));
    };
    line("id", stored.id.clone());
    line("issuer", stored.issuer_node_id.clone());
    line("persisted_at", stored.persisted_at.to_rfc3339());
    line(
        "target",
        format!("{:?}/{}", stored.signal.target, stored.signal.target_id),
    );
    line("category", format!("{:?}", stored.signal.category));
    line("severity", format!("{:?}", stored.signal.severity));
    line("basis", format!("{:?}", stored.signal.basis));
    line("visibility", format!("{:?}", stored.signal.visibility));
    line(
        "confidence",
        stored
            .signal
            .confidence
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string()),
    );
    line(
        "appeal_status",
        format!("{:?}", stored.signal.appeal_status.unwrap_or_default()),
    );
    line(
        "expires_at",
        stored
            .signal
            .expires_at
            .clone()
            .unwrap_or_else(|| "-".to_string()),
    );
    line("operator_adj", format_operator_adjustment(stored));
    out
}

/// operator 確定の印（#1058）。未訂正は `-`、確定済みは時刻と訂正前の category。
fn format_operator_adjustment(stored: &StoredRiskSignal) -> String {
    match (stored.operator_adjusted_at, stored.operator_origin_category) {
        (Some(at), Some(origin)) => format!("{} (origin_category={origin:?})", at.to_rfc3339()),
        (Some(at), None) => at.to_rfc3339(),
        (None, _) => "-".to_string(),
    }
}

fn category_from_arg(arg: SafetyCategoryArg) -> SafetyCategory {
    match arg {
        SafetyCategoryArg::Csam => SafetyCategory::Csam,
        SafetyCategoryArg::Cse => SafetyCategory::Cse,
        SafetyCategoryArg::Grooming => SafetyCategory::Grooming,
        SafetyCategoryArg::Nsfw => SafetyCategory::Nsfw,
        SafetyCategoryArg::Objectionable => SafetyCategory::Objectionable,
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

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use kukuri_cn_safety::{Basis, RiskSignalTarget, SafetyRiskSignal};

    use super::*;

    fn stored(operator_adjusted_at: Option<DateTime<Utc>>) -> StoredRiskSignal {
        StoredRiskSignal {
            id: "sig-1".to_string(),
            issuer_node_id: "issuer".to_string(),
            signal: SafetyRiskSignal {
                target: RiskSignalTarget::PostId,
                target_id: "post-1".to_string(),
                category: SafetyCategory::Spam,
                severity: Severity::Low,
                basis: Basis::ClassifierScore,
                confidence: Some(20),
                visibility: Visibility::Local,
                expires_at: None,
                appeal_status: Some(AppealStatus::None),
            },
            persisted_at: "2026-09-15T00:00:00Z".parse().unwrap(),
            operator_origin_category: operator_adjusted_at.map(|_| SafetyCategory::Nsfw),
            operator_adjusted_at,
        }
    }

    /// #1058 AC-3: `moderation show` で operator が確定した行と訂正前の category が判別できる。
    #[test]
    fn show_marks_operator_adjusted_signal() {
        let adjusted = stored(Some("2026-09-16T01:02:03Z".parse().unwrap()));
        let text = format_signal(&adjusted);
        assert!(
            text.contains("operator_adj:  2026-09-16T01:02:03+00:00 (origin_category=Nsfw)"),
            "{text}"
        );
        assert!(text.contains("category:      Spam"), "{text}");

        let plain = format_signal(&stored(None));
        assert!(plain.contains("operator_adj:  -"), "{plain}");
    }
}
