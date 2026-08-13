//! Shared wire contracts for Community Node index queries (Issue #663).
//!
//! The server and desktop runtime both use these types so field and variant
//! names cannot drift between the two sides of the HTTP boundary.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// Scope kinds supported by the Community Node index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum IndexScopeKind {
    PublicTopic,
    PrivateChannel,
}

/// indexing request の処理状態。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum IndexingRequestStatus {
    Pending,
    Approved,
    Rejected,
}

impl IndexingRequestStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            other => bail!("unknown indexing request status `{other}`"),
        }
    }
}

/// `POST /v1/indexing/requests` の wire request。
///
/// `kind` は server が stable な `INVALID_INDEXING_REQUEST` を返せるよう文字列のまま保持する。
/// private channel の secret を含むため TypeScript へは export しない。
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitIndexingRequestRequest {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub target_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_secret_hex: Option<String>,
}

impl std::fmt::Debug for SubmitIndexingRequestRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SubmitIndexingRequestRequest")
            .field("kind", &self.kind)
            .field("target_id", &self.target_id)
            .field(
                "channel_secret_hex",
                &self.channel_secret_hex.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SubmitIndexingRequestResponse {
    pub request_id: String,
    pub status: IndexingRequestStatus,
}

impl IndexScopeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PublicTopic => "public_topic",
            Self::PrivateChannel => "private_channel",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "public_topic" => Ok(Self::PublicTopic),
            "private_channel" => Ok(Self::PrivateChannel),
            other => bail!("unknown index scope kind `{other}`"),
        }
    }
}

/// Query parameters shared by search, discovery, and recommendations.
///
/// `scope_kind` and `scope_id` must either both be present or both be absent.
/// The HTTP handler validates that cross-field rule.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct IndexQueryParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

/// One projected index result.
///
/// `text` may contain derived tags and is not canonical post content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct IndexEntryView {
    pub scope_kind: IndexScopeKind,
    pub scope_id: String,
    pub object_id: String,
    pub author_pubkey: String,
    pub text: String,
    pub created_at: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct IndexQueryResponse {
    pub entries: Vec<IndexEntryView>,
}

/// Stable non-2xx JSON body returned by `cn-user-api`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
}
