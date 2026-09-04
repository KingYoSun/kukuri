//! trust / relation read surface(#415 / ADR 0026)。

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use kukuri_cn_core::{
    ApiError, ApiResult, clear_relation_optout, filter_relation_visible, get_relation_optout,
    list_trust_risk_inputs, relation_pair_is_suppressed, require_bearer_identity, require_consents,
    set_relation_optout,
};
use kukuri_cn_protocol::{
    RELATION_NOT_FOUND_CODE, RELATION_VISIBILITY_NOT_ACTIVATED_CODE,
    RELATION_VISIBILITY_NOT_CONFIGURED_CODE, RelationNeighborsResponse, RelationOptoutResponse,
    RelationReadResponse, TRUST_READ_NOT_ACTIVATED_CODE, TRUST_READ_NOT_CONFIGURED_CODE,
    TrustUserReadResponse, normalize_pubkey,
};
use kukuri_cn_safety::RiskSignalTarget;
use kukuri_cn_trust::{
    PullAudience, UniformRelationWeight, build_trust_read, cross_node_trust_disclosure,
};
use serde::Deserialize;

use crate::errors::{TrustRelationError, TrustRelationOperation, trust_relation_error};
use crate::state::{RelationVisibilityState, TrustReadState, UserApiState};

/// trust / relation read 共通の前処理: 機能ゲート(未構成なら 404)+ 認証 + consent。
///
/// `CommunityLocalTrust` capability が `Availability::Planned` の既定状態では構成されず、
/// この node は trust / relation read を提供しない。認証は challenge への鍵署名を検証して
/// 発行された bearer(`BearerIdentity`)であり、**viewer = bearer の pubkey に固定**される
/// (`viewer_relative_read_requires_authenticated_viewer` / `relation_read_requires_authenticated_viewer`。
/// 他人を viewer に指定する手段を持たない = なりすまし防止)。
async fn require_trust_read(
    state: &UserApiState,
    headers: &HeaderMap,
) -> ApiResult<(Arc<TrustReadState>, String)> {
    let Some(trust_read) = state.trust_read.clone() else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            TRUST_READ_NOT_CONFIGURED_CODE,
            "this community node does not provide trust / relation reads",
        ));
    };
    if !state.readiness_activation_is_valid().await {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            TRUST_READ_NOT_ACTIVATED_CODE,
            "this community node trust activation is not current",
        ));
    }
    let identity = require_bearer_identity(&state.pool, &state.jwt_config, headers).await?;
    let _ = require_consents(&state.pool, identity.pubkey.as_str()).await?;
    Ok((trust_read, identity.pubkey))
}

/// distance opt-out 設定・判定の共通前処理。index だけを有効にした node でも利用できる。
async fn require_relation_visibility(
    state: &UserApiState,
    headers: &HeaderMap,
) -> ApiResult<(Arc<RelationVisibilityState>, String)> {
    let Some(relation_visibility) = state.relation_visibility.clone() else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            RELATION_VISIBILITY_NOT_CONFIGURED_CODE,
            "this community node does not provide relation distance opt-out",
        ));
    };
    if !state.readiness_activation_is_valid().await {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            RELATION_VISIBILITY_NOT_ACTIVATED_CODE,
            "this community node relation activation is not current",
        ));
    }
    let identity = require_bearer_identity(&state.pool, &state.jwt_config, headers).await?;
    let _ = require_consents(&state.pool, identity.pubkey.as_str()).await?;
    Ok((relation_visibility, identity.pubkey))
}

fn parse_target_pubkey(raw: &str) -> Result<String, ApiError> {
    normalize_pubkey(raw).map_err(|error| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_TRUST_QUERY",
            error.to_string(),
        )
    })
}

/// per-user trust read(ADR 0026 §2.3 / §6.2)。
///
/// 絶対成分(viewer 非依存・relation 非依存・減衰なし)+ 相対成分(viewer / cluster 相対・
/// relation 重み付け・半減期減衰)+ 合成 trust を、寄与 signal の根拠つきで返す。
/// 相対成分の relation 重みは observer-attributed 観測の producer(非決定論的 moderation,
/// ADR 0028 系)実装まで一様 1.0(重み付けの seam は scoring 層で固定済み)。
pub(crate) async fn trust_user_read(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Path(pubkey): Path<String>,
) -> ApiResult<Json<TrustUserReadResponse>> {
    let (trust_read, viewer_pubkey) = require_trust_read(&state, &headers).await?;
    let target = parse_target_pubkey(pubkey.as_str())?;
    let now = chrono::Utc::now();
    let inputs = list_trust_risk_inputs(
        &state.pool,
        RiskSignalTarget::UserPubkey,
        target.as_str(),
        now.to_rfc3339().as_str(),
    )
    .await
    .map_err(|source| {
        TrustRelationError::trust_read(TrustRelationOperation::LoadTrustInputs, source)
    })
    .map_err(trust_relation_error)?;
    let view = build_trust_read(
        target.as_str(),
        &inputs,
        now,
        &trust_read.params,
        &UniformRelationWeight::default(),
    );
    Ok(Json(TrustUserReadResponse {
        viewer_pubkey,
        view,
    }))
}

/// cross-node pull(ADR 0026 §6.3)。
///
/// **confirmed(known-hash / provider-verdict)な絶対成分のみ**を根拠つきで返す。
/// 相対成分・relation・suspected は visibility に依らず返さない。`visibility` は
/// アクセス範囲: `Local` は返さず(既定)、`SubscribedNodes` は bearer で subscriber と
/// 認証できた要求者のみ、`Public` は匿名でも返す。絶対成分は viewer 非依存のため
/// viewer 証明は要さない(bearer は audience 判定のみに使う)。
pub(crate) async fn trust_pull(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Path(pubkey): Path<String>,
) -> ApiResult<Json<kukuri_cn_trust::CrossNodeTrustDisclosure>> {
    let Some(_) = state.trust_read.clone() else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            TRUST_READ_NOT_CONFIGURED_CODE,
            "this community node does not provide trust / relation reads",
        ));
    };
    if !state.readiness_activation_is_valid().await {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            TRUST_READ_NOT_ACTIVATED_CODE,
            "this community node trust activation is not current",
        ));
    }
    // bearer が有効なsubscriberはcurrent consent成立後だけSubscribedNodesへ昇格する。
    // 認証できない要求は匿名としてPublic visibilityだけを維持する。
    let audience = match require_bearer_identity(&state.pool, &state.jwt_config, &headers).await {
        Ok(identity) => {
            let _ = require_consents(&state.pool, identity.pubkey.as_str()).await?;
            PullAudience::SubscribedNodes
        }
        Err(_) => PullAudience::Public,
    };
    let target = parse_target_pubkey(pubkey.as_str())?;
    let now = chrono::Utc::now();
    let inputs = list_trust_risk_inputs(
        &state.pool,
        RiskSignalTarget::UserPubkey,
        target.as_str(),
        now.to_rfc3339().as_str(),
    )
    .await
    .map_err(|source| {
        TrustRelationError::trust_read(TrustRelationOperation::LoadTrustPullInputs, source)
    })
    .map_err(trust_relation_error)?;
    Ok(Json(cross_node_trust_disclosure(
        target.as_str(),
        &inputs,
        audience,
        now,
    )))
}

/// relation read の応答(pairwise cluster proximity。根拠つき)。
/// pairwise relation read(ADR 0026 §2.4)。viewer = bearer identity。
///
/// どちらかが distance opt-out を選択し、この node の境界より遠い場合だけ、edge が無い場合と
/// 同じ 404 を返す。選択状態そのものは相手へ漏らさない。
pub(crate) async fn relation_user_read(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Path(target): Path<String>,
) -> ApiResult<Json<RelationReadResponse>> {
    let (trust_read, viewer_pubkey) = require_trust_read(&state, &headers).await?;
    let relation_visibility = state.relation_visibility.clone().ok_or_else(|| {
        ApiError::new(
            StatusCode::NOT_FOUND,
            RELATION_VISIBILITY_NOT_CONFIGURED_CODE,
            "this community node does not provide relation distance opt-out",
        )
    })?;
    let target = parse_target_pubkey(target.as_str())?;
    let not_found = || {
        ApiError::new(
            StatusCode::NOT_FOUND,
            RELATION_NOT_FOUND_CODE,
            "no relation observed for this pair",
        )
    };
    let proximity = trust_read
        .relation
        .pairwise_proximity(viewer_pubkey.as_str(), target.as_str())
        .await
        .map_err(|source| {
            TrustRelationError::relation_graph(
                TrustRelationOperation::ReadPairwiseProximity,
                source,
            )
        })
        .map_err(trust_relation_error)?
        .ok_or_else(not_found)?;
    if relation_pair_is_suppressed(
        &state.pool,
        viewer_pubkey.as_str(),
        target.as_str(),
        Some(proximity.score),
        relation_visibility.min_proximity,
    )
    .await
    .map_err(|source| {
        TrustRelationError::relation_opt_out(
            TrustRelationOperation::CheckRelationVisibility,
            source,
        )
    })
    .map_err(trust_relation_error)?
    {
        return Err(not_found());
    }
    Ok(Json(RelationReadResponse {
        viewer_pubkey,
        target_pubkey: target,
        proximity,
    }))
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RelationNeighborsParams {
    #[serde(default)]
    limit: Option<usize>,
}

/// discovery / surfacing 用の近接近傍(ADR 0026 §6.1 `neighbors`)。viewer = bearer identity。
///
/// viewer または候補が opt-out 済みで、距離境界外の user だけを結果から除外する。
pub(crate) async fn relation_neighbors(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Query(params): Query<RelationNeighborsParams>,
) -> ApiResult<Json<RelationNeighborsResponse>> {
    let (trust_read, viewer_pubkey) = require_trust_read(&state, &headers).await?;
    let relation_visibility = state.relation_visibility.clone().ok_or_else(|| {
        ApiError::new(
            StatusCode::NOT_FOUND,
            RELATION_VISIBILITY_NOT_CONFIGURED_CODE,
            "this community node does not provide relation distance opt-out",
        )
    })?;
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let neighbors = trust_read
        .relation
        .neighbors(viewer_pubkey.as_str(), limit)
        .await
        .map_err(|source| {
            TrustRelationError::relation_graph(TrustRelationOperation::ReadNeighbors, source)
        })
        .map_err(trust_relation_error)?;
    let neighbors = filter_relation_visible(
        &state.pool,
        relation_visibility.relation.as_ref(),
        viewer_pubkey.as_str(),
        neighbors.as_slice(),
        relation_visibility.min_proximity,
    )
    .await
    .map_err(|source| {
        TrustRelationError::relation_graph(TrustRelationOperation::FilterVisibleNeighbors, source)
    })
    .map_err(trust_relation_error)?;
    Ok(Json(RelationNeighborsResponse {
        viewer_pubkey,
        neighbors,
    }))
}

fn relation_optout_response(
    pubkey: String,
    opted_out_at: Option<chrono::DateTime<chrono::Utc>>,
    min_proximity: f64,
) -> RelationOptoutResponse {
    RelationOptoutResponse {
        pubkey,
        opted_out: opted_out_at.is_some(),
        opted_out_at: opted_out_at.map(|at| at.to_rfc3339()),
        min_proximity,
    }
}

/// 本人の distance opt-out 状態と、この node の距離境界を返す。
pub(crate) async fn relation_optout_get(
    State(state): State<UserApiState>,
    headers: HeaderMap,
) -> ApiResult<Json<RelationOptoutResponse>> {
    let (relation_visibility, pubkey) = require_relation_visibility(&state, &headers).await?;
    let opted_out_at = get_relation_optout(&state.pool, pubkey.as_str())
        .await
        .map_err(|source| {
            TrustRelationError::relation_opt_out(TrustRelationOperation::GetOptOut, source)
        })
        .map_err(trust_relation_error)?;
    Ok(Json(relation_optout_response(
        pubkey,
        opted_out_at,
        relation_visibility.min_proximity,
    )))
}

/// 「見えない」opt-out の設定(ADR 0026 §2.6 / §6.3)。
///
/// **自分自身のみ**設定できる(bearer identity に固定)。可逆(DELETE で解除)で、
/// trust には影響しない(troll 判定回避の手段にしない)。social graph canonical の削除でもない。
pub(crate) async fn relation_optout_set(
    State(state): State<UserApiState>,
    headers: HeaderMap,
) -> ApiResult<Json<RelationOptoutResponse>> {
    let (relation_visibility, pubkey) = require_relation_visibility(&state, &headers).await?;
    set_relation_optout(&state.pool, pubkey.as_str())
        .await
        .map_err(|source| {
            TrustRelationError::relation_opt_out(TrustRelationOperation::SetOptOut, source)
        })
        .map_err(trust_relation_error)?;
    let opted_out_at = get_relation_optout(&state.pool, pubkey.as_str())
        .await
        .map_err(|source| {
            TrustRelationError::relation_opt_out(TrustRelationOperation::GetOptOut, source)
        })
        .map_err(trust_relation_error)?;
    Ok(Json(RelationOptoutResponse {
        pubkey,
        opted_out: true,
        opted_out_at: opted_out_at.map(|at| at.to_rfc3339()),
        min_proximity: relation_visibility.min_proximity,
    }))
}

/// 「見えない」opt-out の解除(可逆性の実装)。
pub(crate) async fn relation_optout_clear(
    State(state): State<UserApiState>,
    headers: HeaderMap,
) -> ApiResult<Json<RelationOptoutResponse>> {
    let (relation_visibility, pubkey) = require_relation_visibility(&state, &headers).await?;
    clear_relation_optout(&state.pool, pubkey.as_str())
        .await
        .map_err(|source| {
            TrustRelationError::relation_opt_out(TrustRelationOperation::ClearOptOut, source)
        })
        .map_err(trust_relation_error)?;
    Ok(Json(RelationOptoutResponse {
        pubkey,
        opted_out: false,
        opted_out_at: None,
        min_proximity: relation_visibility.min_proximity,
    }))
}
