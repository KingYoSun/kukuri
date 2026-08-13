use std::fmt;

use chrono::Utc;
use kukuri_cn_protocol::{
    ApiErrorBody, INDEX_DISCOVERY_PATH, INDEX_RECOMMENDATIONS_PATH, INDEX_SEARCH_PATH,
    IndexQueryParams, IndexQueryResponse, IndexScopeKind, normalize_http_url,
};
use kukuri_store::{ContentObservationRow, ContentObservationStore};
use reqwest::{StatusCode, header::RETRY_AFTER};
use serde::{Deserialize, Serialize};

use super::{community_node_http_client, load_community_node_token};
use crate::runtime::DesktopRuntime;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct CommunityNodeIndexQueryRequest {
    pub base_url: String,
    pub query: Option<String>,
    pub scope_kind: Option<IndexScopeKind>,
    pub scope_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct CommunityNodeIndexQueryError {
    pub code: String,
    pub message: String,
    pub status: Option<u16>,
    pub retry_after_seconds: Option<u64>,
}

impl CommunityNodeIndexQueryError {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            status: None,
            retry_after_seconds: None,
        }
    }

    fn from_response(
        status: StatusCode,
        retry_after_seconds: Option<u64>,
        body: Option<ApiErrorBody>,
    ) -> Self {
        let fallback_code = match status {
            StatusCode::UNAUTHORIZED => "AUTH_REQUIRED",
            StatusCode::FORBIDDEN => "CONSENT_REQUIRED",
            StatusCode::TOO_MANY_REQUESTS => "RATE_LIMITED",
            _ => "INDEX_QUERY_FAILED",
        };
        let fallback_message = format!("community node index request failed with {status}");
        Self {
            code: body
                .as_ref()
                .map_or_else(|| fallback_code.to_string(), |body| body.code.clone()),
            message: body.map_or(fallback_message, |body| body.message),
            status: Some(status.as_u16()),
            retry_after_seconds,
        }
    }
}

impl fmt::Display for CommunityNodeIndexQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CommunityNodeIndexQueryError {}

#[derive(Clone, Copy)]
pub(crate) enum IndexOperation {
    Search,
    Discovery,
    Recommendations,
}

impl IndexOperation {
    fn path(self) -> &'static str {
        match self {
            Self::Search => INDEX_SEARCH_PATH,
            Self::Discovery => INDEX_DISCOVERY_PATH,
            Self::Recommendations => INDEX_RECOMMENDATIONS_PATH,
        }
    }

    fn observation_capability(self) -> &'static str {
        match self {
            Self::Search | Self::Discovery => "community_index",
            Self::Recommendations => "recommendation",
        }
    }
}

impl DesktopRuntime {
    pub(crate) async fn query_community_node_index(
        &self,
        operation: IndexOperation,
        request: CommunityNodeIndexQueryRequest,
    ) -> Result<IndexQueryResponse, CommunityNodeIndexQueryError> {
        let base_url = normalize_http_url(request.base_url.as_str()).map_err(|error| {
            CommunityNodeIndexQueryError::new("INVALID_COMMUNITY_NODE_URL", error.to_string())
        })?;
        self.require_community_node(base_url.as_str())
            .await
            .map_err(|error| {
                CommunityNodeIndexQueryError::new(
                    "COMMUNITY_NODE_NOT_CONFIGURED",
                    error.to_string(),
                )
            })?;

        let has_scope_kind = request.scope_kind.is_some();
        let has_scope_id = request
            .scope_id
            .as_deref()
            .is_some_and(|scope_id| !scope_id.trim().is_empty());
        if has_scope_kind != has_scope_id {
            return Err(CommunityNodeIndexQueryError::new(
                "INVALID_INDEX_QUERY",
                "scope_kind and scope_id must be specified together",
            ));
        }
        if matches!(operation, IndexOperation::Search)
            && request
                .query
                .as_deref()
                .is_none_or(|query| query.trim().is_empty())
        {
            return Err(CommunityNodeIndexQueryError::new(
                "INVALID_INDEX_QUERY",
                "search query must not be empty",
            ));
        }

        self.ensure_community_node_session(base_url.as_str())
            .await
            .map_err(|error| {
                CommunityNodeIndexQueryError::new(
                    "COMMUNITY_NODE_SESSION_FAILED",
                    error.to_string(),
                )
            })?;
        let token = load_community_node_token(&self.db_path, self.identity_mode, base_url.as_str())
            .map_err(|error| {
                CommunityNodeIndexQueryError::new("AUTH_TOKEN_LOAD_FAILED", error.to_string())
            })?
            .ok_or_else(|| {
                CommunityNodeIndexQueryError::new(
                    "AUTH_REQUIRED",
                    "community node authentication is required",
                )
            })?;
        let params = IndexQueryParams {
            q: request.query,
            scope_kind: request
                .scope_kind
                .map(|scope_kind| scope_kind.as_str().to_string()),
            scope_id: request.scope_id,
            limit: request.limit,
        };

        let response = match self
            .send_community_node_index_query(
                base_url.as_str(),
                operation,
                &params,
                token.access_token.as_str(),
            )
            .await
        {
            Err(error) if error.status == Some(StatusCode::UNAUTHORIZED.as_u16()) => {
                let refreshed = self
                    .request_community_node_authentication_token(base_url.as_str())
                    .await
                    .map_err(|error| {
                        CommunityNodeIndexQueryError::new(
                            "COMMUNITY_NODE_REAUTHENTICATION_FAILED",
                            error.to_string(),
                        )
                    })?;
                self.send_community_node_index_query(
                    base_url.as_str(),
                    operation,
                    &params,
                    refreshed.access_token.as_str(),
                )
                .await
            }
            result => result,
        }?;
        self.record_index_observations(base_url.as_str(), operation, &response)
            .await?;
        Ok(response)
    }

    async fn record_index_observations(
        &self,
        base_url: &str,
        operation: IndexOperation,
        response: &IndexQueryResponse,
    ) -> Result<(), CommunityNodeIndexQueryError> {
        let observed_at = Utc::now().timestamp_millis();
        for entry in &response.entries {
            let observations = [
                ("post", entry.object_id.as_str()),
                ("profile", entry.author_pubkey.as_str()),
            ];
            for (subject_kind, subject_id) in observations {
                self.store
                    .put_content_observation(ContentObservationRow {
                        subject_kind: subject_kind.to_string(),
                        subject_id: subject_id.to_string(),
                        node_base_url: base_url.to_string(),
                        capability: operation.observation_capability().to_string(),
                        observed_at,
                    })
                    .await
                    .map_err(|error| {
                        CommunityNodeIndexQueryError::new(
                            "INDEX_OBSERVATION_STORE_FAILED",
                            error.to_string(),
                        )
                    })?;
            }
        }
        Ok(())
    }

    async fn send_community_node_index_query(
        &self,
        base_url: &str,
        operation: IndexOperation,
        params: &IndexQueryParams,
        access_token: &str,
    ) -> Result<IndexQueryResponse, CommunityNodeIndexQueryError> {
        let client = community_node_http_client().map_err(|error| {
            CommunityNodeIndexQueryError::new("INDEX_HTTP_CLIENT_FAILED", error.to_string())
        })?;
        let response = client
            .get(format!("{base_url}{}", operation.path()))
            .bearer_auth(access_token)
            .query(params)
            .send()
            .await
            .map_err(|error| {
                CommunityNodeIndexQueryError::new("INDEX_QUERY_TRANSPORT_FAILED", error.to_string())
            })?;
        let status = response.status();
        let retry_after_seconds = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        if !status.is_success() {
            let body = response.json::<ApiErrorBody>().await.ok();
            return Err(CommunityNodeIndexQueryError::from_response(
                status,
                retry_after_seconds,
                body,
            ));
        }
        response
            .json::<IndexQueryResponse>()
            .await
            .map_err(|error| {
                CommunityNodeIndexQueryError::new("INDEX_QUERY_DECODE_FAILED", error.to_string())
            })
    }
}
