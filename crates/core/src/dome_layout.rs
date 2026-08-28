use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::dome_hosting::{hosting_tags, session_tags, verify_envelope_content};
use crate::{
    DomeCustomizationV1, DomeHostingLeaseV1, KukuriEnvelope, KukuriKeys, MetaversePersistentPropV1,
    Pubkey, sign_envelope_json, validate_dome_customization,
};

pub const DOME_LAYOUT_COMMIT_MIN_INTERVAL_MILLIS: i64 = 30_000;

const LAYOUT_CANDIDATE_KIND: &str = "dome-layout-candidate";
const LAYOUT_COMMIT_KIND: &str = "dome-layout-commit";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DomeLayoutCandidateV1 {
    pub operation_id: String,
    pub instance_id: String,
    pub instance_generation: u64,
    pub lease_epoch: u64,
    pub session_id: String,
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub host_pubkey: Pubkey,
    pub base_manifest_revision: u64,
    pub snapshot_sequence: u64,
    pub captured_at: i64,
    pub persistent_props: Vec<MetaversePersistentPropV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedDomeLayoutCandidateV1 {
    pub candidate: DomeLayoutCandidateV1,
    pub envelope: KukuriEnvelope,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DomeLayoutCommitV1 {
    pub operation_id: String,
    pub instance_id: String,
    pub instance_generation: u64,
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub owner_pubkey: Pubkey,
    pub base_manifest_revision: u64,
    pub next_manifest_revision: u64,
    pub candidate_digest: String,
    pub manifest_blob_hash: String,
    pub committed_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedDomeLayoutCommitV1 {
    pub commit: DomeLayoutCommitV1,
    pub envelope: KukuriEnvelope,
}

pub fn dome_layout_candidate_digest(candidate: &DomeLayoutCandidateV1) -> Result<String> {
    let bytes = serde_json::to_vec(candidate).context("failed to encode Dome layout candidate")?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

pub fn build_signed_dome_layout_candidate(
    host_keys: &KukuriKeys,
    lease: &DomeHostingLeaseV1,
    candidate: DomeLayoutCandidateV1,
) -> Result<SignedDomeLayoutCandidateV1> {
    validate_dome_layout_candidate(lease, &candidate)?;
    if lease.host.signing_pubkey() != &host_keys.public_key()
        || candidate.host_pubkey != host_keys.public_key()
    {
        bail!("Dome layout candidate host must match lease target");
    }
    let envelope = sign_envelope_json(
        host_keys,
        LAYOUT_CANDIDATE_KIND,
        session_tags(
            &candidate.instance_id,
            candidate.lease_epoch,
            &candidate.session_id,
        ),
        &candidate,
    )?;
    Ok(SignedDomeLayoutCandidateV1 {
        candidate,
        envelope,
    })
}

pub fn verify_signed_dome_layout_candidate(
    signed: &SignedDomeLayoutCandidateV1,
    lease: &DomeHostingLeaseV1,
    session_id: &str,
) -> Result<()> {
    validate_dome_layout_candidate(lease, &signed.candidate)?;
    if signed.candidate.session_id != session_id {
        bail!("Dome layout candidate session is stale");
    }
    verify_envelope_content(
        &signed.envelope,
        LAYOUT_CANDIDATE_KIND,
        lease.host.signing_pubkey(),
        &signed.candidate,
    )
}

pub fn build_signed_dome_layout_commit(
    owner_keys: &KukuriKeys,
    lease: &DomeHostingLeaseV1,
    candidate: &SignedDomeLayoutCandidateV1,
    commit: DomeLayoutCommitV1,
) -> Result<SignedDomeLayoutCommitV1> {
    verify_signed_dome_layout_candidate(candidate, lease, &candidate.candidate.session_id)?;
    validate_dome_layout_commit(lease, &candidate.candidate, &commit)?;
    if commit.owner_pubkey != owner_keys.public_key() || commit.owner_pubkey != lease.owner_pubkey {
        bail!("Dome layout commit owner must match signer and lease");
    }
    let envelope = sign_envelope_json(
        owner_keys,
        LAYOUT_COMMIT_KIND,
        hosting_tags(&commit.instance_id, lease.epoch, &commit.operation_id),
        &commit,
    )?;
    Ok(SignedDomeLayoutCommitV1 { commit, envelope })
}

pub fn verify_signed_dome_layout_commit(
    signed: &SignedDomeLayoutCommitV1,
    lease: &DomeHostingLeaseV1,
    candidate: &SignedDomeLayoutCandidateV1,
) -> Result<()> {
    verify_signed_dome_layout_candidate(candidate, lease, &candidate.candidate.session_id)?;
    validate_dome_layout_commit(lease, &candidate.candidate, &signed.commit)?;
    verify_envelope_content(
        &signed.envelope,
        LAYOUT_COMMIT_KIND,
        &lease.owner_pubkey,
        &signed.commit,
    )
}

fn validate_dome_layout_candidate(
    lease: &DomeHostingLeaseV1,
    candidate: &DomeLayoutCandidateV1,
) -> Result<()> {
    if candidate.operation_id.trim().is_empty()
        || candidate.instance_id != lease.instance_id
        || candidate.instance_generation != lease.instance_generation
        || candidate.lease_epoch != lease.epoch
        || candidate.session_id.trim().is_empty()
        || candidate.host_pubkey != *lease.host.signing_pubkey()
        || candidate.base_manifest_revision != lease.manifest_version
        || candidate.snapshot_sequence == 0
    {
        bail!("Dome layout candidate does not match active lease");
    }
    let customization = DomeCustomizationV1 {
        persistent_props: candidate.persistent_props.clone(),
        ..DomeCustomizationV1::default()
    };
    validate_dome_customization(&customization)
}

fn validate_dome_layout_commit(
    lease: &DomeHostingLeaseV1,
    candidate: &DomeLayoutCandidateV1,
    commit: &DomeLayoutCommitV1,
) -> Result<()> {
    if commit.operation_id != candidate.operation_id
        || commit.instance_id != lease.instance_id
        || commit.instance_generation != lease.instance_generation
        || commit.owner_pubkey != lease.owner_pubkey
        || commit.base_manifest_revision != candidate.base_manifest_revision
        || commit.next_manifest_revision != commit.base_manifest_revision.saturating_add(1)
        || commit.manifest_blob_hash.trim().is_empty()
        || commit.candidate_digest != dome_layout_candidate_digest(candidate)?
        || commit.committed_at < candidate.captured_at
    {
        bail!("Dome layout commit does not match candidate and active lease");
    }
    Ok(())
}
