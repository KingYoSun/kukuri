//! 通報と異議申し立ての共有通信契約。

use serde::{Deserialize, Serialize};

/// 異議申し立ての対象となるリスク判定。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CommunityNodeReportAppeal {
    pub risk_signal_id: String,
}

/// `POST /v1/report` の要求本文。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct CommunityNodeReportRequest {
    #[serde(default)]
    pub subject_kind: String,
    #[serde(default)]
    pub subject_id: String,
    #[serde(default)]
    pub capability: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reporter_contact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appeal: Option<CommunityNodeReportAppeal>,
}

/// `POST /v1/report` の成功応答。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct CommunityNodeReportResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disputed_risk_signal_id: Option<String>,
}
