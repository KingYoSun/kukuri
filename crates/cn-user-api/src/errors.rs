//! HTTP error mapping boundary.
//!
//! Domain errors are downcast before mapping. Only failures that remain unknown, or are explicitly
//! classified as infrastructure failures, fall back to `500 INTERNAL_ERROR`. This keeps expected
//! client failures from being accidentally promoted to 500 responses while retaining the existing
//! status/code/message wire contract.

use axum::http::StatusCode;
use kukuri_cn_core::{AdmissionRejection, ApiError, ChannelSecretConflict};

pub(crate) fn internal_error(error: impl std::fmt::Display) -> ApiError {
    ApiError::new(
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "INTERNAL_ERROR",
        error.to_string(),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AccountLifecycleOperation {
    CreateAuthChallenge,
    LoadAdmissionConfig,
    VerifyAuth,
    GetConsentStatus,
    AcceptConsents,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum AccountLifecycleError {
    #[error("{source}")]
    Infrastructure {
        operation: AccountLifecycleOperation,
        #[source]
        source: anyhow::Error,
    },
    #[error("{0}")]
    Admission(AdmissionRejection),
    #[error("{0}")]
    AuthFailed(anyhow::Error),
}

impl AccountLifecycleError {
    pub(crate) fn infrastructure(
        operation: AccountLifecycleOperation,
        source: anyhow::Error,
    ) -> Self {
        Self::Infrastructure { operation, source }
    }

    pub(crate) fn auth_verify(source: anyhow::Error) -> Self {
        let source = match source.downcast::<AdmissionRejection>() {
            Ok(rejection) => return Self::Admission(rejection),
            Err(source) => source,
        };
        if source.downcast_ref::<sqlx::Error>().is_some() {
            Self::infrastructure(AccountLifecycleOperation::VerifyAuth, source)
        } else {
            Self::AuthFailed(source)
        }
    }
}

pub(crate) fn account_lifecycle_error(error: AccountLifecycleError) -> ApiError {
    match error {
        AccountLifecycleError::Admission(rejection) => {
            ApiError::new(StatusCode::FORBIDDEN, rejection.code(), rejection.message())
        }
        AccountLifecycleError::AuthFailed(source) => {
            ApiError::new(StatusCode::UNAUTHORIZED, "AUTH_FAILED", source.to_string())
        }
        AccountLifecycleError::Infrastructure { operation, source } => {
            // HTTP 契約(500 / INTERNAL / 汎用文言)は変えず、原因箇所は tracing にだけ出す。
            tracing::error!(?operation, error = %source, "account lifecycle infrastructure failure");
            internal_error(source)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SupportEndpointOperation {
    LoadBootstrapNodes,
    LoadBootstrapSeedPeers,
    RefreshBootstrapPeer,
    RecordRendezvousHeartbeat,
    StoreReport,
}

#[derive(Debug, thiserror::Error)]
#[error("{source}")]
pub(crate) struct SupportEndpointError {
    operation: SupportEndpointOperation,
    #[source]
    source: anyhow::Error,
}

impl SupportEndpointError {
    pub(crate) fn new(operation: SupportEndpointOperation, source: anyhow::Error) -> Self {
        Self { operation, source }
    }
}

pub(crate) fn support_endpoint_error(error: SupportEndpointError) -> ApiError {
    let SupportEndpointError { operation, source } = error;
    tracing::error!(?operation, error = %source, "support endpoint infrastructure failure");
    internal_error(source)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndexingOperation {
    RegisterRequest,
    SearchScope,
    SearchAll,
    Discovery,
    Recommendations,
    FilterRelationVisibility,
    VerifyChannelMembership,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum IndexingError {
    #[error("{0}")]
    ChannelSecretConflict(ChannelSecretConflict),
    #[error("{0}")]
    InvalidChannelSecret(anyhow::Error),
    #[error("{source}")]
    Infrastructure {
        operation: IndexingOperation,
        #[source]
        source: anyhow::Error,
    },
}

impl IndexingError {
    pub(crate) fn channel_secret(source: anyhow::Error) -> Self {
        match source.downcast::<ChannelSecretConflict>() {
            Ok(conflict) => Self::ChannelSecretConflict(conflict),
            Err(source) => Self::InvalidChannelSecret(source),
        }
    }

    pub(crate) fn infrastructure(operation: IndexingOperation, source: anyhow::Error) -> Self {
        Self::Infrastructure { operation, source }
    }
}

pub(crate) fn indexing_error(error: IndexingError) -> ApiError {
    match error {
        IndexingError::ChannelSecretConflict(conflict) => ApiError::new(
            StatusCode::CONFLICT,
            "CHANNEL_SECRET_CONFLICT",
            conflict.to_string(),
        ),
        IndexingError::InvalidChannelSecret(source) => ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_CHANNEL_SECRET",
            source.to_string(),
        ),
        IndexingError::Infrastructure { operation, source } => {
            tracing::error!(?operation, error = %source, "indexing infrastructure failure");
            internal_error(source)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrustRelationOperation {
    LoadTrustInputs,
    LoadTrustPullInputs,
    CheckRelationVisibility,
    ReadPairwiseProximity,
    ReadNeighbors,
    FilterVisibleNeighbors,
    SetOptOut,
    GetOptOut,
    ClearOptOut,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TrustRelationError {
    #[error("{source}")]
    TrustRead {
        operation: TrustRelationOperation,
        #[source]
        source: anyhow::Error,
    },
    #[error("{source}")]
    RelationGraph {
        operation: TrustRelationOperation,
        #[source]
        source: anyhow::Error,
    },
    #[error("{source}")]
    RelationOptOut {
        operation: TrustRelationOperation,
        #[source]
        source: anyhow::Error,
    },
}

impl TrustRelationError {
    pub(crate) fn trust_read(operation: TrustRelationOperation, source: anyhow::Error) -> Self {
        Self::TrustRead { operation, source }
    }

    pub(crate) fn relation_graph(operation: TrustRelationOperation, source: anyhow::Error) -> Self {
        Self::RelationGraph { operation, source }
    }

    pub(crate) fn relation_opt_out(
        operation: TrustRelationOperation,
        source: anyhow::Error,
    ) -> Self {
        Self::RelationOptOut { operation, source }
    }
}

pub(crate) fn trust_relation_error(error: TrustRelationError) -> ApiError {
    // variant 種別(どの依存面の失敗か)と operation(どの読み書きか)を観測に乗せる。
    let (kind, operation, source) = match error {
        TrustRelationError::TrustRead { operation, source } => ("trust_read", operation, source),
        TrustRelationError::RelationGraph { operation, source } => {
            ("relation_graph", operation, source)
        }
        TrustRelationError::RelationOptOut { operation, source } => {
            ("relation_opt_out", operation, source)
        }
    };
    tracing::error!(kind, ?operation, error = %source, "trust/relation infrastructure failure");
    internal_error(source)
}

#[cfg(test)]
pub(crate) async fn assert_error_contract(
    error: ApiError,
    expected_status: axum::http::StatusCode,
    expected_code: &str,
    expected_message: &str,
) {
    use axum::response::IntoResponse;

    let response = error.into_response();
    assert_eq!(response.status(), expected_status);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read API error response body");
    let body: serde_json::Value =
        serde_json::from_slice(&body).expect("parse API error response body");
    assert_eq!(body["code"], expected_code);
    assert_eq!(body["message"], expected_message);
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use kukuri_cn_core::{ApiError, auth_required_error, consent_required_error};

    use super::{
        SupportEndpointError, SupportEndpointOperation, TrustRelationError, TrustRelationOperation,
        assert_error_contract, internal_error, support_endpoint_error, trust_relation_error,
    };

    #[tokio::test]
    async fn common_error_response_contracts_are_stable() {
        assert_error_contract(
            internal_error(anyhow::anyhow!("database unavailable")),
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "database unavailable",
        )
        .await;
        assert_error_contract(
            auth_required_error("bearer token is required"),
            StatusCode::UNAUTHORIZED,
            "AUTH_REQUIRED",
            "bearer token is required",
        )
        .await;
        assert_error_contract(
            consent_required_error("required policies must be accepted"),
            StatusCode::FORBIDDEN,
            "CONSENT_REQUIRED",
            "required policies must be accepted",
        )
        .await;
        assert_error_contract(
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_REQUEST",
                "request is malformed",
            ),
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            "request is malformed",
        )
        .await;
        assert_error_contract(
            ApiError::new(
                StatusCode::NOT_FOUND,
                "NOT_CONFIGURED",
                "capability is not configured",
            ),
            StatusCode::NOT_FOUND,
            "NOT_CONFIGURED",
            "capability is not configured",
        )
        .await;
    }

    #[tokio::test]
    async fn support_endpoint_infrastructure_errors_keep_internal_fallback() {
        for operation in [
            SupportEndpointOperation::LoadBootstrapNodes,
            SupportEndpointOperation::LoadBootstrapSeedPeers,
            SupportEndpointOperation::RefreshBootstrapPeer,
            SupportEndpointOperation::RecordRendezvousHeartbeat,
            SupportEndpointOperation::StoreReport,
        ] {
            assert_error_contract(
                support_endpoint_error(SupportEndpointError::new(
                    operation,
                    anyhow::anyhow!("backend unavailable"),
                )),
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "backend unavailable",
            )
            .await;
        }
    }

    #[tokio::test]
    async fn trust_relation_infrastructure_errors_keep_internal_fallback() {
        for error in [
            TrustRelationError::trust_read(
                TrustRelationOperation::LoadTrustInputs,
                anyhow::anyhow!("trust store unavailable"),
            ),
            TrustRelationError::relation_graph(
                TrustRelationOperation::ReadPairwiseProximity,
                anyhow::anyhow!("relation graph unavailable"),
            ),
            TrustRelationError::relation_opt_out(
                TrustRelationOperation::SetOptOut,
                anyhow::anyhow!("opt-out store unavailable"),
            ),
        ] {
            let message = error.to_string();
            assert_error_contract(
                trust_relation_error(error),
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                message.as_str(),
            )
            .await;
        }
    }
}
