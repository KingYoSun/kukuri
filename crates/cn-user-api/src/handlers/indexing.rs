//! indexing request の受付(#413)と、ユーザー向け index query(#404)。
//! route 上も /v1/indexing(登録)と /v1/index(検索)で対になっているため同居させる。

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use kukuri_cn_core::{
    ApiError, ApiResult, IndexScopeKind, filter_relation_visible, get_channel_secret,
    insert_indexing_request, register_channel_secret, require_bearer_identity, require_consents,
};
use kukuri_cn_indexer::IndexQuery;
use kukuri_cn_protocol::{
    CHANNEL_MEMBERSHIP_REQUIRED_CODE, CHANNEL_MEMBERSHIP_SECRET_HEADER, IndexEntryView,
    IndexQueryParams, IndexQueryResponse, SubmitIndexingRequestRequest,
    SubmitIndexingRequestResponse,
};

use crate::errors::{IndexingError, IndexingOperation, indexing_error};
use crate::state::{RelationVisibilityState, UserApiState};

/// user からの indexing request を受け付けて保存する(#413 / ADR 0025 §2.2 / §6.3)。
///
/// 認証済み(bearer)+ consent 済み user のみ要求できる。request は index を保証しない: operator が
/// supported set に入れ、さらに safety verdict が `allow` の content だけが index される多段ゲートの
/// 入口である。
///
/// - public topic: target_id(topic_id)を pending request として保存する。
/// - private channel: channel secret(capability)の提示が必須。secret を提示できること自体を channel
///   権限の証明とみなす(ADR 0025 §6.3。CN は新権限体系を作らない)。secret は at-rest 暗号化して保存し、
///   cn-indexer が Model C と同じ機構で `channel::` replica を sync する。channel secret 暗号鍵が未設定の
///   node は private channel request を受け付けない(平文保存しないため)。
pub(crate) async fn submit_indexing_request(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Json(request): Json<SubmitIndexingRequestRequest>,
) -> ApiResult<Json<SubmitIndexingRequestResponse>> {
    let identity = require_bearer_identity(&state.pool, &state.jwt_config, &headers).await?;
    let _ = require_consents(&state.pool, identity.pubkey.as_str()).await?;

    let kind = match request.kind.trim() {
        "public_topic" => IndexScopeKind::PublicTopic,
        "private_channel" => IndexScopeKind::PrivateChannel,
        _ => {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_INDEXING_REQUEST",
                "kind must be `public_topic` or `private_channel`",
            ));
        }
    };
    let target_id = request.target_id.trim();
    if target_id.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_INDEXING_REQUEST",
            "target_id is required",
        ));
    }

    // private channel は capability(secret)の提示が必須。これが権限の証明を兼ねる。
    if kind == IndexScopeKind::PrivateChannel {
        let secret_hex = request
            .channel_secret_hex
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let Some(secret_hex) = secret_hex else {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "CHANNEL_SECRET_REQUIRED",
                "private channel indexing requires the channel secret",
            ));
        };
        // channel secret を平文保存しないため、暗号鍵未設定の node は受け付けない。
        let Some(cipher) = state.channel_secret_cipher.as_ref() else {
            return Err(ApiError::new(
                StatusCode::NOT_FOUND,
                "CHANNEL_INDEXING_NOT_CONFIGURED",
                "this community node does not accept private channel indexing requests",
            ));
        };
        // first-writer-wins: 別 requester が別 secret で既存 capability を上書きできないようにする。
        // 同一 secret の再提示は冪等。別 secret による乗っ取りは 409 で拒否する。
        register_channel_secret(&state.pool, cipher, target_id, secret_hex)
            .await
            .map_err(IndexingError::channel_secret)
            .map_err(indexing_error)?;
    }

    let stored = insert_indexing_request(&state.pool, identity.pubkey.as_str(), kind, target_id)
        .await
        .map_err(|source| IndexingError::infrastructure(IndexingOperation::RegisterRequest, source))
        .map_err(indexing_error)?;
    Ok(Json(SubmitIndexingRequestResponse {
        request_id: stored.id,
        status: stored.status,
    }))
}

fn index_query_response(entries: Vec<kukuri_cn_indexer::IndexedEntry>) -> IndexQueryResponse {
    IndexQueryResponse {
        entries: entries
            .into_iter()
            .map(|entry| IndexEntryView {
                scope_kind: entry.scope_kind,
                scope_id: entry.scope_id,
                object_id: entry.object_id,
                author_pubkey: entry.author_pubkey,
                text: entry.text,
                created_at: entry.created_at,
            })
            .collect(),
    }
}

/// index query 共通の前処理: 機能ゲート(未構成なら 404)+ 認証 + consent。
///
/// query 境界(`FailClosedIndexQuery`)を返す。`CommunityIndex` capability が
/// `Availability::Planned` の既定状態では index query は構成されず、この node は
/// search / discovery / recommendation を提供しない。
async fn require_index_query(
    state: &UserApiState,
    headers: &HeaderMap,
) -> ApiResult<(Arc<dyn IndexQuery>, Arc<RelationVisibilityState>, String)> {
    let Some(index_query) = state.index_query.clone() else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "INDEX_QUERY_NOT_CONFIGURED",
            "this community node does not provide index queries",
        ));
    };
    let Some(relation_visibility) = state.relation_visibility.clone() else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "RELATION_VISIBILITY_NOT_CONFIGURED",
            "this community node does not provide relation distance opt-out",
        ));
    };
    if !state.readiness_activation_is_valid().await {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "INDEX_QUERY_NOT_ACTIVATED",
            "this community node index activation is not current",
        ));
    }
    let identity = require_bearer_identity(&state.pool, &state.jwt_config, headers).await?;
    let _ = require_consents(&state.pool, identity.pubkey.as_str()).await?;
    Ok((index_query, relation_visibility, identity.pubkey))
}

async fn filter_index_entries(
    state: &UserApiState,
    relation_visibility: &RelationVisibilityState,
    viewer_pubkey: &str,
    entries: Vec<kukuri_cn_indexer::IndexedEntry>,
) -> ApiResult<Vec<kukuri_cn_indexer::IndexedEntry>> {
    let authors: Vec<String> = entries
        .iter()
        .map(|entry| entry.author_pubkey.clone())
        .collect();
    let visible = filter_relation_visible(
        &state.pool,
        relation_visibility.relation.as_ref(),
        viewer_pubkey,
        authors.as_slice(),
        relation_visibility.min_proximity,
    )
    .await
    .map_err(|source| {
        IndexingError::infrastructure(IndexingOperation::FilterRelationVisibility, source)
    })
    .map_err(indexing_error)?;
    let visible: std::collections::HashSet<String> = visible.into_iter().collect();
    Ok(entries
        .into_iter()
        .filter(|entry| visible.contains(&entry.author_pubkey))
        .collect())
}

/// 非公開チャンネル範囲指定読みの所属証明(#711 / ADR 0025 §6.3)。
///
/// 提示された channel secret を保存済み capability の復号値と定数時間比較する。
/// 「秘密値の提示が権限の証明」(申請側と同じ原則)を read にも適用し、新しい権限体系は
/// 作らない。未提示・不一致・チャンネル未登録・暗号鍵未設定は同一の安定コードで拒否し、
/// 非所属者に索引の存在有無を漏らさない。提示値・保存値はログへ出さない。
async fn require_channel_membership(
    state: &UserApiState,
    headers: &HeaderMap,
    channel_id: &str,
) -> ApiResult<()> {
    let denied = || {
        ApiError::new(
            StatusCode::FORBIDDEN,
            CHANNEL_MEMBERSHIP_REQUIRED_CODE,
            "private channel index queries require the channel secret of a participant",
        )
    };
    let presented = headers
        .get(CHANNEL_MEMBERSHIP_SECRET_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let Some(presented) = presented else {
        return Err(denied());
    };
    let Some(cipher) = state.channel_secret_cipher.as_ref() else {
        return Err(denied());
    };
    let stored = get_channel_secret(&state.pool, cipher, channel_id)
        .await
        .map_err(|source| {
            IndexingError::infrastructure(IndexingOperation::VerifyChannelMembership, source)
        })
        .map_err(indexing_error)?;
    let Some(stored) = stored else {
        return Err(denied());
    };
    if !constant_time_str_eq(stored.namespace_secret_hex.as_str(), presented) {
        return Err(denied());
    }
    Ok(())
}

fn constant_time_str_eq(expected: &str, supplied: &str) -> bool {
    if expected.len() != supplied.len() || expected.is_empty() {
        return false;
    }
    expected
        .bytes()
        .zip(supplied.bytes())
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

/// `scope_kind` / `scope_id` パラメータの組を解釈する。
///
/// 両方指定 = scope 内読み、両方無指定 = 横断。片方のみは 400。
fn parse_index_scope_params(
    params: &IndexQueryParams,
) -> Result<Option<(IndexScopeKind, String)>, ApiError> {
    let scope_kind = params
        .scope_kind
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let scope_id = params
        .scope_id
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    match (scope_kind, scope_id) {
        (None, None) => Ok(None),
        (Some(kind), Some(id)) => {
            let kind = IndexScopeKind::parse(kind).map_err(|error| {
                ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_INDEX_QUERY",
                    error.to_string(),
                )
            })?;
            Ok(Some((kind, id.to_string())))
        }
        _ => Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_INDEX_QUERY",
            "scope_kind and scope_id must be provided together",
        )),
    }
}

/// limit パラメータ(未指定は既定 20。gate 側で `MAX_QUERY_LIMIT` に丸められる)。
fn index_query_limit(params: &IndexQueryParams) -> usize {
    params.limit.unwrap_or(20)
}

/// ユーザー向け検索(#404 / ADR 0025 §2.7)。
///
/// `scope_kind` + `scope_id` 指定で topic 内検索(基本 UX)、無指定で supported set 横断検索
/// (別画面)。結果は fail-closed query gate を通った `allow` verdict の entry のみ。
pub(crate) async fn index_search(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Query(params): Query<IndexQueryParams>,
) -> ApiResult<Json<IndexQueryResponse>> {
    let (index_query, relation_visibility, viewer_pubkey) =
        require_index_query(&state, &headers).await?;
    let query = params
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_INDEX_QUERY",
                "q is required",
            )
        })?;
    let limit = index_query_limit(&params);
    let entries = match parse_index_scope_params(&params)? {
        Some((scope_kind, scope_id)) => {
            if scope_kind == IndexScopeKind::PrivateChannel {
                require_channel_membership(&state, &headers, scope_id.as_str()).await?;
            }
            index_query
                .search_scope(scope_kind, scope_id.as_str(), query, limit)
                .await
                .map_err(|source| {
                    IndexingError::infrastructure(IndexingOperation::SearchScope, source)
                })
                .map_err(indexing_error)?
        }
        None => index_query
            .search_all(query, limit)
            .await
            .map_err(|source| IndexingError::infrastructure(IndexingOperation::SearchAll, source))
            .map_err(indexing_error)?,
    };
    let entries = filter_index_entries(
        &state,
        relation_visibility.as_ref(),
        viewer_pubkey.as_str(),
        entries,
    )
    .await?;
    Ok(Json(index_query_response(entries)))
}

/// discovery(新着列挙。#404)。scope 指定で topic 内、無指定で supported set 横断。
///
/// ranking / 関連度スコアリングの具体は ADR 0025 §4 でスコープ外のため、最小 surface として
/// created_at 降順の新着を返す。critical / 非 allow verdict は fail-closed gate で入らない。
pub(crate) async fn index_discovery(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Query(params): Query<IndexQueryParams>,
) -> ApiResult<Json<IndexQueryResponse>> {
    let (index_query, relation_visibility, viewer_pubkey) =
        require_index_query(&state, &headers).await?;
    let limit = index_query_limit(&params);
    let scope = parse_index_scope_params(&params)?;
    if let Some((IndexScopeKind::PrivateChannel, scope_id)) = scope.as_ref() {
        require_channel_membership(&state, &headers, scope_id.as_str()).await?;
    }
    let entries = index_query
        .list_recent(scope.as_ref().map(|(kind, id)| (*kind, id.as_str())), limit)
        .await
        .map_err(|source| IndexingError::infrastructure(IndexingOperation::Discovery, source))
        .map_err(indexing_error)?;
    let entries = filter_index_entries(
        &state,
        relation_visibility.as_ref(),
        viewer_pubkey.as_str(),
        entries,
    )
    .await?;
    Ok(Json(index_query_response(entries)))
}

/// recommendation(#404)。supported set 横断の新着列挙を最小 surface として返す。
///
/// ranking アルゴリズムの具体は ADR 0025 §4 でスコープ外。critical verdict が recommendation に
/// 入らないことは fail-closed gate(真実源 + 最新 verdict 突合)が保証する。
pub(crate) async fn index_recommendations(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Query(params): Query<IndexQueryParams>,
) -> ApiResult<Json<IndexQueryResponse>> {
    let (index_query, relation_visibility, viewer_pubkey) =
        require_index_query(&state, &headers).await?;
    let limit = index_query_limit(&params);
    let entries = index_query
        .list_recent(None, limit)
        .await
        .map_err(|source| IndexingError::infrastructure(IndexingOperation::Recommendations, source))
        .map_err(indexing_error)?;
    let entries = filter_index_entries(
        &state,
        relation_visibility.as_ref(),
        viewer_pubkey.as_str(),
        entries,
    )
    .await?;
    Ok(Json(index_query_response(entries)))
}

/// channel secret 登録失敗を HTTP 応答へマップする。
///
/// 既存 capability と異なる secret での上書き(乗っ取り試行)は 409、hex 形式不正等は 400。
#[cfg(test)]
mod error_contract_tests {
    use axum::http::StatusCode;
    use kukuri_cn_core::ChannelSecretConflict;

    use crate::errors::{IndexingError, IndexingOperation, assert_error_contract, indexing_error};

    #[tokio::test]
    async fn channel_secret_error_contracts_are_stable() {
        assert_error_contract(
            indexing_error(IndexingError::channel_secret(
                ChannelSecretConflict::AlreadyRegistered.into(),
            )),
            StatusCode::CONFLICT,
            "CHANNEL_SECRET_CONFLICT",
            "a different channel capability is already registered for this channel",
        )
        .await;
        assert_error_contract(
            indexing_error(IndexingError::channel_secret(anyhow::anyhow!(
                "channel secret must be 32 bytes"
            ))),
            StatusCode::BAD_REQUEST,
            "INVALID_CHANNEL_SECRET",
            "channel secret must be 32 bytes",
        )
        .await;

        for operation in [
            IndexingOperation::RegisterRequest,
            IndexingOperation::SearchScope,
            IndexingOperation::SearchAll,
            IndexingOperation::Discovery,
            IndexingOperation::Recommendations,
            IndexingOperation::FilterRelationVisibility,
        ] {
            assert_error_contract(
                indexing_error(IndexingError::infrastructure(
                    operation,
                    anyhow::anyhow!("index backend unavailable"),
                )),
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "index backend unavailable",
            )
            .await;
        }
    }
}
