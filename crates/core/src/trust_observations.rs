//! CN へ提供するブロック / ミュート観測の envelope（ADR 0026 §8.3、#1061）。
//!
//! block は author replica に書かれる既存の `block-edge` envelope をそのまま観測として使う。
//! mute は端末内の状態（ADR 0022）なので、提供時だけ `mute-observation` envelope に署名する。
//! この envelope は docs sync・gossip・author replica に書かない。

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::crypto::{now_timestamp_millis, validate_pubkey};
use crate::profile::{BlockEdgeStatus, parse_block_edge};
use crate::{EnvelopeId, KukuriEnvelope, KukuriKeys, Pubkey};

pub const MUTE_OBSERVATION_KIND: &str = "mute-observation";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum MuteObservationStatus {
    Active,
    Revoked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KukuriMuteObservationEnvelopeContentV1 {
    pub subject_pubkey: Pubkey,
    pub target_pubkey: Pubkey,
    pub status: MuteObservationStatus,
}

pub fn build_mute_observation_envelope(
    keys: &KukuriKeys,
    target_pubkey: &Pubkey,
    status: MuteObservationStatus,
) -> Result<KukuriEnvelope> {
    let subject_pubkey = keys.public_key();
    if subject_pubkey == *target_pubkey {
        bail!("self mute observation is not allowed");
    }
    let content = KukuriMuteObservationEnvelopeContentV1 {
        subject_pubkey: subject_pubkey.clone(),
        target_pubkey: target_pubkey.clone(),
        status,
    };
    let created_at = now_timestamp_millis()?;
    let encoded = serde_json::to_string(&content).context("failed to encode envelope content")?;
    crate::sign_envelope_at(
        keys,
        MUTE_OBSERVATION_KIND,
        vec![
            vec!["subject".into(), subject_pubkey.as_str().to_string()],
            vec!["target".into(), target_pubkey.as_str().to_string()],
            vec!["object".into(), MUTE_OBSERVATION_KIND.into()],
        ],
        encoded,
        created_at,
    )
}

/// 観測の種別。同一 observer → target で両方あれば、評価側で強い方を採用する。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TrustObservationKind {
    Block,
    Mute,
}

impl TrustObservationKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Mute => "mute",
        }
    }
}

/// 署名検証済みの観測 1 件。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustObservation {
    pub observer_pubkey: Pubkey,
    pub target_pubkey: Pubkey,
    pub kind: TrustObservationKind,
    pub active: bool,
    /// envelope の created_at（ミリ秒）。
    pub observed_at: i64,
    pub envelope_id: EnvelopeId,
}

/// envelope を観測として検証する。block-edge / mute-observation 以外は `Ok(None)`。
///
/// 署名・id を検証し、subject（observer）が署名者と一致しない envelope は拒否する。
pub fn parse_trust_observation(envelope: &KukuriEnvelope) -> Result<Option<TrustObservation>> {
    match envelope.kind.as_str() {
        "block-edge" => {
            envelope.verify()?;
            let Some(edge) = parse_block_edge(envelope)? else {
                return Ok(None);
            };
            Ok(Some(TrustObservation {
                observer_pubkey: edge.subject_pubkey,
                target_pubkey: edge.target_pubkey,
                kind: TrustObservationKind::Block,
                active: edge.status == BlockEdgeStatus::Active,
                observed_at: edge.updated_at,
                envelope_id: edge.envelope_id,
            }))
        }
        MUTE_OBSERVATION_KIND => {
            envelope.verify()?;
            let content: KukuriMuteObservationEnvelopeContentV1 =
                serde_json::from_str(&envelope.content)
                    .context("failed to parse mute observation envelope")?;
            validate_pubkey(content.subject_pubkey.as_str())
                .context("invalid mute observation subject pubkey")?;
            validate_pubkey(content.target_pubkey.as_str())
                .context("invalid mute observation target pubkey")?;
            if content.subject_pubkey != envelope.pubkey {
                bail!("mute observation subject pubkey must match envelope signer");
            }
            if content.subject_pubkey == content.target_pubkey {
                bail!("self mute observation is not allowed");
            }
            Ok(Some(TrustObservation {
                observer_pubkey: content.subject_pubkey,
                target_pubkey: content.target_pubkey,
                kind: TrustObservationKind::Mute,
                active: content.status == MuteObservationStatus::Active,
                observed_at: envelope.created_at,
                envelope_id: envelope.id.clone(),
            }))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_block_edge_envelope, generate_keys};

    #[test]
    fn mute_observation_round_trips_and_verifies_signer() {
        let observer = generate_keys();
        let target = generate_keys().public_key();
        let envelope =
            build_mute_observation_envelope(&observer, &target, MuteObservationStatus::Active)
                .expect("sign");
        let observation = parse_trust_observation(&envelope)
            .expect("parse")
            .expect("observation");
        assert_eq!(observation.observer_pubkey, observer.public_key());
        assert_eq!(observation.target_pubkey, target);
        assert_eq!(observation.kind, TrustObservationKind::Mute);
        assert!(observation.active);
        assert_eq!(observation.observed_at, envelope.created_at);
    }

    #[test]
    fn block_edge_is_accepted_as_block_observation() {
        let observer = generate_keys();
        let target = generate_keys().public_key();
        let envelope =
            build_block_edge_envelope(&observer, &target, BlockEdgeStatus::Revoked).expect("sign");
        let observation = parse_trust_observation(&envelope)
            .expect("parse")
            .expect("observation");
        assert_eq!(observation.kind, TrustObservationKind::Block);
        assert!(!observation.active);
    }

    #[test]
    fn tampered_or_foreign_subject_observation_is_rejected() {
        let observer = generate_keys();
        let other = generate_keys();
        let target = generate_keys().public_key();
        let mut envelope =
            build_mute_observation_envelope(&observer, &target, MuteObservationStatus::Active)
                .expect("sign");
        envelope.content = serde_json::to_string(&KukuriMuteObservationEnvelopeContentV1 {
            subject_pubkey: other.public_key(),
            target_pubkey: target.clone(),
            status: MuteObservationStatus::Active,
        })
        .expect("encode");
        assert!(parse_trust_observation(&envelope).is_err());

        // 別人の鍵で「observer が subject」の内容に署名しても、署名者不一致で拒否する。
        let forged = crate::sign_envelope_at(
            &other,
            MUTE_OBSERVATION_KIND,
            Vec::new(),
            serde_json::to_string(&KukuriMuteObservationEnvelopeContentV1 {
                subject_pubkey: observer.public_key(),
                target_pubkey: target,
                status: MuteObservationStatus::Active,
            })
            .expect("encode"),
            1,
        )
        .expect("sign");
        assert!(parse_trust_observation(&forged).is_err());
    }

    #[test]
    fn unrelated_envelope_kind_is_not_an_observation() {
        let keys = generate_keys();
        let envelope =
            crate::sign_envelope_at(&keys, "identity-profile", Vec::new(), "{}".into(), 1)
                .expect("sign");
        assert!(parse_trust_observation(&envelope).expect("parse").is_none());
    }
}
