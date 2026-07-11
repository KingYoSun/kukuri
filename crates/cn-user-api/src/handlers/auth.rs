//! 認証(challenge / verify)。

use axum::Json;
use axum::extract::State;
use kukuri_cn_core::{
    ApiResult, AuthChallengeResponse, AuthVerifyResponse, create_auth_challenge,
    load_admission_config, verify_auth_envelope_and_issue_token,
};
use kukuri_cn_protocol::{AuthChallengeRequest, AuthVerifyRequest};

use crate::errors::{AccountLifecycleError, AccountLifecycleOperation, account_lifecycle_error};
use crate::state::UserApiState;

pub(crate) async fn auth_challenge(
    State(state): State<UserApiState>,
    Json(request): Json<AuthChallengeRequest>,
) -> ApiResult<Json<AuthChallengeResponse>> {
    let response = create_auth_challenge(&state.pool, request.pubkey.as_str())
        .await
        .map_err(|source| {
            AccountLifecycleError::infrastructure(
                AccountLifecycleOperation::CreateAuthChallenge,
                source,
            )
        })
        .map_err(account_lifecycle_error)?;
    Ok(Json(response))
}

pub(crate) async fn auth_verify(
    State(state): State<UserApiState>,
    Json(request): Json<AuthVerifyRequest>,
) -> ApiResult<Json<AuthVerifyResponse>> {
    let admission_config = load_admission_config(&state.pool)
        .await
        .map_err(|source| {
            AccountLifecycleError::infrastructure(
                AccountLifecycleOperation::LoadAdmissionConfig,
                source,
            )
        })
        .map_err(account_lifecycle_error)?;
    let response = verify_auth_envelope_and_issue_token(
        &state.pool,
        &state.jwt_config,
        state.self_node.resolved_urls.public_base_url.as_str(),
        &request.auth_envelope_json,
        request.endpoint_id.as_deref(),
        request.addr_hint.as_deref(),
        admission_config.mode,
        request.invite_code.as_deref(),
    )
    .await
    .map_err(AccountLifecycleError::auth_verify)
    .map_err(account_lifecycle_error)?;
    Ok(Json(response))
}

/// auth/verify の失敗を HTTP 応答へマップする。admission 拒否(#383)は 403 + 専用コードで
/// 区別し、それ以外の署名・challenge 検証失敗は従来通り 401 AUTH_FAILED にする。
#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use kukuri_cn_core::AdmissionRejection;

    use crate::errors::{AccountLifecycleError, account_lifecycle_error, assert_error_contract};

    #[tokio::test]
    async fn auth_verify_error_contracts_are_stable() {
        assert_error_contract(
            account_lifecycle_error(AccountLifecycleError::auth_verify(
                AdmissionRejection::InviteExpired.into(),
            )),
            StatusCode::FORBIDDEN,
            "INVITE_EXPIRED",
            "the provided invite code has expired",
        )
        .await;
        assert_error_contract(
            account_lifecycle_error(AccountLifecycleError::auth_verify(anyhow::anyhow!(
                "signature verification failed"
            ))),
            StatusCode::UNAUTHORIZED,
            "AUTH_FAILED",
            "signature verification failed",
        )
        .await;
        assert_error_contract(
            account_lifecycle_error(AccountLifecycleError::auth_verify(
                sqlx::Error::RowNotFound.into(),
            )),
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "no rows returned by a query that expected to return at least one row",
        )
        .await;
    }
}
