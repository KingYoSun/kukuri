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
