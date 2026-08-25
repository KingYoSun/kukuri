//! Community Node の区分別保持設定。

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetentionConfig {
    #[serde(default = "default_connection_logs_days")]
    pub connection_logs_days: u32,
    #[serde(default = "default_moderation_logs_days")]
    pub moderation_logs_days: u32,
    #[serde(default = "default_report_days")]
    pub report_days: u32,
    #[serde(default = "default_report_contact_days")]
    pub report_contact_days: u32,
    #[serde(default = "default_rights_request_active_days")]
    pub rights_request_active_days: u32,
    #[serde(default = "default_rights_request_resolved_days")]
    pub rights_request_resolved_days: u32,
    #[serde(default = "default_rights_request_rejected_days")]
    pub rights_request_rejected_days: u32,
    #[serde(default = "default_rights_request_contact_days")]
    pub rights_request_contact_days: u32,
    #[serde(default = "default_rights_request_identity_days")]
    pub rights_request_identity_days: u32,
    #[serde(default = "default_rights_request_evidence_days")]
    pub rights_request_evidence_days: u32,
    #[serde(default = "default_rights_request_history_days")]
    pub rights_request_history_days: u32,
    #[serde(default = "default_operator_audit_days")]
    pub operator_audit_days: u32,
    #[serde(default = "default_moderation_event_days")]
    pub moderation_event_days: u32,
    #[serde(default = "default_risk_signal_days")]
    pub risk_signal_days: u32,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            connection_logs_days: default_connection_logs_days(),
            moderation_logs_days: default_moderation_logs_days(),
            report_days: default_report_days(),
            report_contact_days: default_report_contact_days(),
            rights_request_active_days: default_rights_request_active_days(),
            rights_request_resolved_days: default_rights_request_resolved_days(),
            rights_request_rejected_days: default_rights_request_rejected_days(),
            rights_request_contact_days: default_rights_request_contact_days(),
            rights_request_identity_days: default_rights_request_identity_days(),
            rights_request_evidence_days: default_rights_request_evidence_days(),
            rights_request_history_days: default_rights_request_history_days(),
            operator_audit_days: default_operator_audit_days(),
            moderation_event_days: default_moderation_event_days(),
            risk_signal_days: default_risk_signal_days(),
        }
    }
}

fn default_connection_logs_days() -> u32 {
    30
}
fn default_moderation_logs_days() -> u32 {
    180
}
fn default_report_days() -> u32 {
    180
}
fn default_report_contact_days() -> u32 {
    90
}
fn default_rights_request_active_days() -> u32 {
    730
}
fn default_rights_request_resolved_days() -> u32 {
    365
}
fn default_rights_request_rejected_days() -> u32 {
    180
}
fn default_rights_request_contact_days() -> u32 {
    180
}
fn default_rights_request_identity_days() -> u32 {
    180
}
fn default_rights_request_evidence_days() -> u32 {
    180
}
fn default_rights_request_history_days() -> u32 {
    365
}
fn default_operator_audit_days() -> u32 {
    365
}
fn default_moderation_event_days() -> u32 {
    180
}
fn default_risk_signal_days() -> u32 {
    180
}

pub(crate) fn validate_retention(retention: &RetentionConfig) -> Result<()> {
    for (name, days) in [
        ("connection_logs_days", retention.connection_logs_days),
        ("moderation_logs_days", retention.moderation_logs_days),
        ("report_days", retention.report_days),
        ("report_contact_days", retention.report_contact_days),
        (
            "rights_request_active_days",
            retention.rights_request_active_days,
        ),
        (
            "rights_request_resolved_days",
            retention.rights_request_resolved_days,
        ),
        (
            "rights_request_rejected_days",
            retention.rights_request_rejected_days,
        ),
        (
            "rights_request_contact_days",
            retention.rights_request_contact_days,
        ),
        (
            "rights_request_identity_days",
            retention.rights_request_identity_days,
        ),
        (
            "rights_request_evidence_days",
            retention.rights_request_evidence_days,
        ),
        (
            "rights_request_history_days",
            retention.rights_request_history_days,
        ),
        ("operator_audit_days", retention.operator_audit_days),
        ("moderation_event_days", retention.moderation_event_days),
        ("risk_signal_days", retention.risk_signal_days),
    ] {
        if days == 0 || days > 3_650 {
            bail!("retention.{name} は 1-3650 日で指定してください");
        }
    }
    Ok(())
}
