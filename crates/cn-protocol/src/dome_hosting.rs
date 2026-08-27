use kukuri_core::{
    DomeHostingStateKindV1, DomeInstanceManifestV1, DomePresetManifestV1,
    SignedDomeHostingAcceptanceV1, SignedDomeHostingActivationV1, SignedDomeHostingCloseV1,
    SignedDomeHostingLeaseV1, SignedDomePhysicsSnapshotV1, SignedDomeSessionInputV1,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeHostingAssignmentRequest {
    pub signed_lease: SignedDomeHostingLeaseV1,
    pub instance_manifest: DomeInstanceManifestV1,
    pub preset_manifest: DomePresetManifestV1,
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
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomeHostingSessionSnapshotResponse {
    pub signed_snapshot: SignedDomePhysicsSnapshotV1,
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
}
