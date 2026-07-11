//! HTTP error mapping boundary.
//!
//! Domain errors are downcast before mapping. Only failures that remain unknown, or are explicitly
//! classified as infrastructure failures, fall back to `500 INTERNAL_ERROR`. This keeps expected
//! client failures from being accidentally promoted to 500 responses while retaining the existing
//! status/code/message wire contract.

use axum::http::StatusCode;
use kukuri_cn_core::{AdmissionRejection, ApiError};

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
            let _ = operation;
            internal_error(source)
        }
    }
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

    use super::{assert_error_contract, internal_error};

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
}
