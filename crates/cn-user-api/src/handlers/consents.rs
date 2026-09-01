//! 同意(consents)。

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use kukuri_cn_core::{
    ApiResult, accept_consents, get_consent_status, list_policies, require_bearer_pubkey,
};
use kukuri_cn_protocol::{
    AcceptConsentsRequest, CommunityNodeConsentStatus, CommunityNodePoliciesResponse,
};

use crate::errors::{AccountLifecycleError, AccountLifecycleOperation, account_lifecycle_error};
use crate::state::UserApiState;

/// 認証不要の公開 policy カタログ(#857)。同意提示に必要な文書一覧・本文・版だけを
/// 返し、ユーザー固有情報を含まない。同意記録の読み書き(status / accept)は
/// 引き続き bearer 認証を要求する。
pub(crate) async fn public_policies(
    State(state): State<UserApiState>,
) -> ApiResult<Json<CommunityNodePoliciesResponse>> {
    let policies = list_policies(&state.pool)
        .await
        .map_err(|source| {
            AccountLifecycleError::infrastructure(AccountLifecycleOperation::ListPolicies, source)
        })
        .map_err(account_lifecycle_error)?;
    Ok(Json(CommunityNodePoliciesResponse { policies }))
}

pub(crate) async fn consent_status(
    State(state): State<UserApiState>,
    headers: HeaderMap,
) -> ApiResult<Json<CommunityNodeConsentStatus>> {
    let pubkey = require_bearer_pubkey(&state.pool, &state.jwt_config, &headers).await?;
    let status = get_consent_status(&state.pool, pubkey.as_str())
        .await
        .map_err(|source| {
            AccountLifecycleError::infrastructure(
                AccountLifecycleOperation::GetConsentStatus,
                source,
            )
        })
        .map_err(account_lifecycle_error)?;
    Ok(Json(status))
}

pub(crate) async fn accept_consents_handler(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Json(request): Json<AcceptConsentsRequest>,
) -> ApiResult<Json<CommunityNodeConsentStatus>> {
    let pubkey = require_bearer_pubkey(&state.pool, &state.jwt_config, &headers).await?;
    let status = accept_consents(&state.pool, pubkey.as_str(), &request.policy_slugs)
        .await
        .map_err(|source| {
            AccountLifecycleError::infrastructure(AccountLifecycleOperation::AcceptConsents, source)
        })
        .map_err(account_lifecycle_error)?;
    Ok(Json(status))
}
