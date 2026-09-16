//! タイムライン向け content advisory 一括照会(#1056 / ADR 0046 §6.3 / ADR 0028 §8.12)。
//!
//! `POST /v1/advisories/lookup` は認証 + 同意済みの client から可視の post id / blob hash を受け、
//! **この node 自身が発行した** nsfw / objectionable の advisory だけを返す。読み取りのみで、
//! index scope・verdict・signal を変更しない。相対 trust / relation は返さない。
//! rate limit は全 route に掛かる共通 layer(`apply_rate_limit`)で適用される。

use std::collections::BTreeSet;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use kukuri_cn_core::{
    ApiError, ApiResult, list_content_advisories_for_subjects, require_bearer_identity,
    require_consents,
};
use kukuri_cn_protocol::{
    ADVISORY_LOOKUP_MAX_SUBJECTS, AdvisoryLookupRequest, AdvisoryLookupResponse,
    INDEX_QUERY_NOT_ACTIVATED_CODE, INDEX_QUERY_NOT_CONFIGURED_CODE, INVALID_ADVISORY_LOOKUP_CODE,
    is_advisory_blob_hash,
};

use crate::errors::{IndexingError, IndexingOperation, indexing_error};
use crate::state::UserApiState;

/// 一括照会の前処理: 機能ゲート(索引未提供 / 有効化失効 / manifest 無しは 404)+ 認証 + 同意。
///
/// advisory は索引の判定(verdict)に伴って発行されるため、索引参照と同じ構成・有効化条件と
/// 安定コードを使う。発行元 = 公開 manifest の `node_id`(起動時に署名鍵と一致検査済み。#706)。
async fn require_advisory_lookup(state: &UserApiState, headers: &HeaderMap) -> ApiResult<String> {
    if state.index_query.is_none() {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            INDEX_QUERY_NOT_CONFIGURED_CODE,
            "this community node does not provide content advisories",
        ));
    }
    let issuer_node_id = state
        .manifest
        .as_ref()
        .map(|manifest| manifest.node_id.trim().to_string())
        .filter(|node_id| !node_id.is_empty())
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                INDEX_QUERY_NOT_CONFIGURED_CODE,
                "this community node does not publish an issuer identity",
            )
        })?;
    if !state.readiness_activation_is_valid().await {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            INDEX_QUERY_NOT_ACTIVATED_CODE,
            "this community node index activation is not current",
        ));
    }
    let identity = require_bearer_identity(&state.pool, &state.jwt_config, headers).await?;
    let _ = require_consents(&state.pool, identity.pubkey.as_str()).await?;
    Ok(issuer_node_id)
}

fn invalid(message: impl Into<String>) -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        INVALID_ADVISORY_LOOKUP_CODE,
        message,
    )
}

/// 要求の識別子を正規化する(前後空白除去・重複除去)。空・上限超過・形式不正は 400。
fn normalize_subjects(
    request: AdvisoryLookupRequest,
) -> Result<(Vec<String>, Vec<String>), ApiError> {
    if request.subject_count() > ADVISORY_LOOKUP_MAX_SUBJECTS {
        return Err(invalid(format!(
            "at most {ADVISORY_LOOKUP_MAX_SUBJECTS} subjects can be looked up at once"
        )));
    }
    let mut post_ids = BTreeSet::new();
    for post_id in request.post_ids {
        let post_id = post_id.trim();
        if post_id.is_empty() || post_id.len() > 256 {
            return Err(invalid("post_ids must be non-empty identifiers"));
        }
        post_ids.insert(post_id.to_string());
    }
    let mut blob_hashes = BTreeSet::new();
    for hash in request.blob_hashes {
        let hash = hash.trim().to_ascii_lowercase();
        if !is_advisory_blob_hash(hash.as_str()) {
            return Err(invalid("blob_hashes must be 64 hex characters"));
        }
        blob_hashes.insert(hash);
    }
    if post_ids.is_empty() && blob_hashes.is_empty() {
        return Err(invalid("at least one post id or blob hash is required"));
    }
    Ok((
        post_ids.into_iter().collect(),
        blob_hashes.into_iter().collect(),
    ))
}

pub(crate) async fn lookup_content_advisories(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Json(request): Json<AdvisoryLookupRequest>,
) -> ApiResult<Json<AdvisoryLookupResponse>> {
    let issuer_node_id = require_advisory_lookup(&state, &headers).await?;
    let (post_ids, blob_hashes) = normalize_subjects(request)?;
    let now = chrono::Utc::now().to_rfc3339();
    let advisories = list_content_advisories_for_subjects(
        &state.pool,
        issuer_node_id.as_str(),
        post_ids.as_slice(),
        blob_hashes.as_slice(),
        now.as_str(),
    )
    .await
    .map_err(|source| IndexingError::infrastructure(IndexingOperation::LookupAdvisories, source))
    .map_err(indexing_error)?;
    Ok(Json(AdvisoryLookupResponse { advisories }))
}
