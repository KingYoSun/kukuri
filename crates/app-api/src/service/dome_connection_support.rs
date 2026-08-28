use crate::service::*;
use kukuri_core::SpatialContextV1;
use serde::{Deserialize, Serialize};

pub(crate) const PROPOSAL_PREFIX: &str = "metaverse/dome-connections/proposals";
pub(crate) const SELECTION_PREFIX: &str = "metaverse/dome-connections/selections";
pub(crate) const CONNECTION_PREFIX: &str = "metaverse/dome-connections/agreements";
pub(crate) const LOCAL_PROPOSAL_RATE_WINDOW_MS: i64 = 10 * 60 * 1_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DomeProposalStateDocV1 {
    pub(crate) proposal: DomeConnectionProposalV1,
    pub(crate) connection_id: String,
    pub(crate) proposal_envelope_id: EnvelopeId,
    pub(crate) proposer_agreement_envelope_id: EnvelopeId,
    pub(crate) terminal_reason: Option<DomeConnectionTerminalReasonV1>,
    pub(crate) terminal_event_envelope_id: Option<EnvelopeId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DomeSelectionStateDocV1 {
    pub(crate) selection: DomeProposalSelectionV1,
    pub(crate) envelope_id: EnvelopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DomeConnectionStateDocV1 {
    pub(crate) record: DomeConnectionRecordV1,
    pub(crate) proposer_agreement_envelope_id: EnvelopeId,
    pub(crate) receiver_agreement_envelope_id: EnvelopeId,
    pub(crate) lifecycle_envelope_id: EnvelopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DomeProposalTerminalEventV1 {
    pub(crate) proposal_id: String,
    pub(crate) spatial_context: SpatialContextV1,
    pub(crate) actor_pubkey: Pubkey,
    pub(crate) reason: DomeConnectionTerminalReasonV1,
    pub(crate) updated_at: i64,
}
