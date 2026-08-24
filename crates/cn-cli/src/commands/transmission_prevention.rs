use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use kukuri_cn_core::{
    NewTransmissionPrevention, TransmissionPreventionBasis, TransmissionPreventionCapability,
    apply_transmission_prevention, get_active_transmission_prevention, initialize_database,
    release_transmission_prevention,
};

use crate::{
    TransmissionPreventionAction, TransmissionPreventionBasisArg,
    TransmissionPreventionCapabilityArg,
};

pub(super) async fn run(pool: &PgPool, action: TransmissionPreventionAction) -> Result<()> {
    initialize_database(pool).await?;
    match action {
        TransmissionPreventionAction::Apply {
            actor,
            subject_kind,
            subject_id,
            basis,
            capabilities,
            expires_at,
            related_report_id,
        } => {
            let expires_at = expires_at
                .map(|value| {
                    DateTime::parse_from_rfc3339(value.as_str())
                        .map(|value| value.with_timezone(&Utc))
                        .context("--expires-at must be RFC3339")
                })
                .transpose()?;
            let mutation = apply_transmission_prevention(
                pool,
                actor.as_str(),
                &NewTransmissionPrevention {
                    subject_kind,
                    subject_id,
                    basis: basis.into(),
                    capabilities: capabilities.into_iter().map(Into::into).collect(),
                    expires_at,
                    related_report_id,
                },
            )
            .await?;
            println!(
                "transmission prevention applied: id={} subject={}/{} removed_index_scopes={} audit={}",
                mutation.decision.id,
                mutation.decision.subject_kind,
                mutation.decision.subject_id,
                mutation.removed_index_scopes.len(),
                mutation.audit.id,
            );
        }
        TransmissionPreventionAction::Release {
            actor,
            subject_kind,
            subject_id,
            reason,
        } => {
            let mutation = release_transmission_prevention(
                pool,
                actor.as_str(),
                subject_kind.as_str(),
                subject_id.as_str(),
                reason.as_str(),
            )
            .await?;
            println!(
                "transmission prevention released: id={} audit={} (fresh ingest required)",
                mutation.decision.id, mutation.audit.id,
            );
        }
        TransmissionPreventionAction::Status {
            subject_kind,
            subject_id,
        } => match get_active_transmission_prevention(
            pool,
            subject_kind.as_str(),
            subject_id.as_str(),
        )
        .await?
        {
            Some(value) => println!("{}", serde_json::to_string_pretty(&value)?),
            None => println!("no active transmission-prevention decision"),
        },
    }
    Ok(())
}

impl From<TransmissionPreventionBasisArg> for TransmissionPreventionBasis {
    fn from(value: TransmissionPreventionBasisArg) -> Self {
        match value {
            TransmissionPreventionBasisArg::Copyright => Self::Copyright,
            TransmissionPreventionBasisArg::Privacy => Self::Privacy,
            TransmissionPreventionBasisArg::PersonalityRights => Self::PersonalityRights,
            TransmissionPreventionBasisArg::Trademark => Self::Trademark,
            TransmissionPreventionBasisArg::OtherRights => Self::OtherRights,
        }
    }
}

impl From<TransmissionPreventionCapabilityArg> for TransmissionPreventionCapability {
    fn from(value: TransmissionPreventionCapabilityArg) -> Self {
        match value {
            TransmissionPreventionCapabilityArg::CommunityIndex => Self::CommunityIndex,
            TransmissionPreventionCapabilityArg::Search => Self::Search,
            TransmissionPreventionCapabilityArg::Discovery => Self::Discovery,
            TransmissionPreventionCapabilityArg::Recommendation => Self::Recommendation,
            TransmissionPreventionCapabilityArg::Moderation => Self::Moderation,
            TransmissionPreventionCapabilityArg::BlobCache => Self::BlobCache,
        }
    }
}
