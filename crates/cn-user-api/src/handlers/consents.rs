//! 同意(consents)。

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use kukuri_cn_core::{
    ApiResult, CommunityNodeConsentStatus, accept_consents, get_consent_status,
    require_bearer_pubkey,
};
use kukuri_cn_protocol::AcceptConsentsRequest;

use crate::errors::internal_error;
use crate::state::UserApiState;

pub(crate) async fn consent_status(
    State(state): State<UserApiState>,
    headers: HeaderMap,
) -> ApiResult<Json<CommunityNodeConsentStatus>> {
    let pubkey = require_bearer_pubkey(&state.pool, &state.jwt_config, &headers).await?;
    let status = get_consent_status(&state.pool, pubkey.as_str())
        .await
        .map_err(internal_error)?;
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
        .map_err(internal_error)?;
    Ok(Json(status))
}
