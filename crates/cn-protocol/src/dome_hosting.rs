use kukuri_core::{
    DomeHostingStateKindV1, DomeInstanceManifestV1, DomePresetManifestV1, DomeSpatialAccessProofV1,
    DomeTransitionAdmissionRequestV1, DomeTransitionAdmissionTicketV1,
    MetaverseResourceBudgetConfig, MetaverseResourceMetricsV1, SignedDomeHostingAcceptanceV1,
    SignedDomeHostingActivationV1, SignedDomeHostingCloseV1, SignedDomeHostingLeaseV1,
    SignedDomeLayoutCandidateV1, SignedDomePhysicsSnapshotV1, SignedDomeSessionInputV1,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeHostingAssignmentRequest {
    pub signed_lease: SignedDomeHostingLeaseV1,
    pub instance_manifest: DomeInstanceManifestV1,
    pub preset_manifest: DomePresetManifestV1,
    pub asset_blobs: Vec<DomeHostingAssetBlob>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeHostingAssetBlob {
    pub blob_hash: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeHostingAssignmentResponse {
    pub signed_acceptance: SignedDomeHostingAcceptanceV1,
    pub state: DomeHostingStateKindV1,
    pub session_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeHostingActivationRequest {
    pub instance_id: String,
    pub signed_activation: SignedDomeHostingActivationV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeHostingReleaseRequest {
    pub instance_id: String,
    pub signed_close: SignedDomeHostingCloseV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeHostingSessionInputRequest {
    pub signed_input: SignedDomeSessionInputV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_proof: Option<DomeSpatialAccessProofV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeHostingSessionSnapshotResponse {
    pub signed_snapshot: SignedDomePhysicsSnapshotV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeTransitionPrepareRequest {
    pub request: DomeTransitionAdmissionRequestV1,
    pub access_proof: DomeSpatialAccessProofV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeTransitionPrepareResponse {
    pub ticket: DomeTransitionAdmissionTicketV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeTransitionCommitRequest {
    pub ticket: DomeTransitionAdmissionTicketV1,
    pub position: [i64; 3],
    pub rotation: [i64; 3],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeTransitionAbortRequest {
    pub ticket: DomeTransitionAdmissionTicketV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeTransitionMutationResponse {
    pub transition_id: String,
    pub state: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeHostingLayoutCandidateRequest {
    pub instance_id: String,
    pub operation_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeHostingLayoutCandidateResponse {
    pub signed_candidate: SignedDomeLayoutCandidateV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeHostingSnapshotResyncRequest {
    pub instance_id: String,
    pub after_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeHostingSnapshotResyncResponse {
    pub snapshots: Vec<SignedDomePhysicsSnapshotV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeHostingStatusResponse {
    pub instance_id: String,
    pub state: DomeHostingStateKindV1,
    pub lease_epoch: u64,
    pub session_id: Option<String>,
    pub participants: u32,
    pub sleeping: bool,
    pub expires_at: i64,
    pub resource_budget: MetaverseResourceBudgetConfig,
    pub resource_metrics: MetaverseResourceMetricsV1,
}
