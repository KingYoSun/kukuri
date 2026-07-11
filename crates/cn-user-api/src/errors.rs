//! ドメイン横断の共通エラー写像。ドメイン固有の写像(auth の admission 拒否、
//! channel secret の衝突など)は各ハンドラファイルに置く。

use kukuri_cn_core::ApiError;

pub(crate) fn internal_error(error: impl std::fmt::Display) -> ApiError {
    ApiError::new(
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "INTERNAL_ERROR",
        error.to_string(),
    )
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
