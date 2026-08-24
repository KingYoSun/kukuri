//! 権利侵害申出の公開通信契約。

use serde::{Deserialize, Serialize};

pub const RIGHTS_REQUEST_SCOPE_PATH: &str = "/v1/rights-requests/scope";
pub const RIGHTS_REQUEST_CREATE_PATH: &str = "/v1/rights-requests";
pub const RIGHTS_REQUEST_STATUS_PATH: &str = "/v1/rights-requests/status";
pub const RIGHTS_REQUEST_WITHDRAW_PATH: &str = "/v1/rights-requests/withdraw";
pub const RIGHTS_REQUEST_FORM_PATH: &str = "/rights-requests/new";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum RightsRequesterKind {
    RightsHolder,
    Representative,
    RightsManagementOrganization,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum RightsCategory {
    Copyright,
    Privacy,
    PersonalityRights,
    Trademark,
    OtherRights,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum EvidenceReferenceKind {
    Url,
    Hash,
    Identifier,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EvidenceReference {
    pub kind: EvidenceReferenceKind,
    pub value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum RightsRequestScopeStatus {
    VerifiedScope,
    UnverifiedScope,
    OutOfScope,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum RightsRequestStatus {
    Received,
    NeedsInformation,
    Reviewing,
    SenderContacting,
    Actioned,
    Declined,
    OutOfScope,
    Withdrawn,
}

/// 申出画面より先に表示する、当該 node の対応範囲。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RightsRequestScopeResponse {
    pub scope_revision: String,
    pub node_name: String,
    pub available_actions: Vec<String>,
    pub unavailable_actions: Vec<String>,
    pub initial_response_target_days: u32,
    pub acknowledgement: String,
}

/// `POST /v1/rights-requests` の要求本文。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct RightsRequestCreateRequest {
    pub scope_revision: String,
    pub scope_acknowledged: bool,
    pub requester_kind: RightsRequesterKind,
    pub requester_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    pub email: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub represented_rights_holder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority_basis: Option<String>,
    pub rights_category: RightsCategory,
    pub rights_basis: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_work_description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_work_reference: Option<String>,
    pub subject_kind: String,
    pub subject_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_url: Option<String>,
    pub infringement_description: String,
    pub no_permission_statement: bool,
    #[serde(default)]
    pub evidence_references: Vec<EvidenceReference>,
    #[serde(default)]
    pub requested_capabilities: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RightsRequestCreateResponse {
    pub reference_id: String,
    pub tracking_secret: String,
    pub scope_status: RightsRequestScopeStatus,
    pub status: RightsRequestStatus,
    pub received_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RightsRequestAccessRequest {
    pub reference_id: String,
    pub tracking_secret: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct RightsRequestStatusResponse {
    pub reference_id: String,
    pub scope_status: RightsRequestScopeStatus,
    pub status: RightsRequestStatus,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_message: Option<String>,
}
