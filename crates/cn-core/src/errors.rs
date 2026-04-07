use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::config::USER_API_BEARER_CHALLENGE;

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    headers: Vec<(HeaderName, HeaderValue)>,
}

pub type ApiResult<T> = std::result::Result<T, ApiError>;

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            headers: Vec::new(),
        }
    }

    pub fn with_header(
        mut self,
        name: impl Into<HeaderName>,
        value: impl TryInto<HeaderValue>,
    ) -> Self {
        if let Ok(value) = value.try_into() {
            self.headers.push((name.into(), value));
        }
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = (
            self.status,
            axum::Json(json!({
                "code": self.code,
                "message": self.message,
            })),
        )
            .into_response();
        for (name, value) in self.headers {
            response.headers_mut().insert(name, value);
        }
        response
    }
}

pub fn auth_required_error(message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED", message).with_header(
        HeaderName::from_static("www-authenticate"),
        USER_API_BEARER_CHALLENGE,
    )
}

pub fn consent_required_error(message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::FORBIDDEN, "CONSENT_REQUIRED", message)
}
