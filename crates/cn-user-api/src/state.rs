//! user-api の実行時 state(DI)と構築。

use std::sync::Arc;

use anyhow::{Context, Result};
use kukuri_cn_core::{
    ChannelSecretCipher, DatabaseInitMode, JwtConfig, PgIndexEntryStore, TopicRendezvousStore,
    connect_postgres, initialize_database, initialize_database_for_runtime,
};
use kukuri_cn_indexer::{
    ArcadeDbConfig, ArcadeDbProjection, ArcadeDbRelationGraph, FailClosedIndexQuery, IndexQuery,
};
use kukuri_cn_operator::{
    CommunityNodeManifest, READINESS_CHECK_IDS, build_manifest, load_and_validate,
};
use kukuri_cn_protocol::{CommunityNodeBootstrapNode, CommunityNodeResolvedUrls};
use kukuri_cn_trust::{RelationStore, TrustParams};
use sqlx::postgres::PgPool;

use crate::config::UserApiConfig;

#[derive(Clone)]
pub struct UserApiState {
    pub(crate) pool: PgPool,
    pub(crate) rendezvous_store: TopicRendezvousStore,
    pub(crate) jwt_config: JwtConfig,
    pub(crate) self_node: CommunityNodeBootstrapNode,
    /// 公開する manifest(operator config が設定されている場合のみ)。
    pub(crate) manifest: Option<Arc<CommunityNodeManifest>>,
    /// private channel の indexing request で受け取る channel secret を at-rest 暗号化する cipher。
    /// 鍵 material(`COMMUNITY_NODE_CHANNEL_SECRET_KEY`)が未設定なら None で、private channel の
    /// indexing request は受け付けない(secret を平文保存しないため)。
    pub(crate) channel_secret_cipher: Option<Arc<ChannelSecretCipher>>,
    /// ユーザー向け search / discovery / recommendation の query 境界(#404)。
    /// fail-closed query gate(`FailClosedIndexQuery`)を通した読み口のみを持つ。
    /// None = 機能無効(`CommunityIndex` が `Availability::Planned` の既定状態)で、
    /// `/v1/index/*` は 404 を返す。
    pub(crate) index_query: Option<Arc<dyn IndexQuery>>,
    /// trust / relation read surface(#415 / ADR 0026)。
    /// None = 機能無効(`CommunityLocalTrust` が `Availability::Planned` の既定状態)で、
    /// `/v1/trust/*` / `/v1/relation/*` は 404 を返す。
    pub(crate) trust_read: Option<Arc<TrustReadState>>,
}

/// trust / relation read surface の依存一式(#415)。
///
/// trust の入力(risk signal)は Postgres(`UserApiState::pool`)から読み、relation は
/// graph backend(本番 = ArcadeDB、テスト = in-memory)から読む。
pub struct TrustReadState {
    /// trust 合成のパラメータ(operator 可変, ADR 0026 §6.2)。
    pub params: TrustParams,
    /// relation graph の読み口(graph-store 抽象境界, §6.1)。
    pub relation: Arc<dyn RelationStore>,
}

impl UserApiState {
    /// query 境界を差し替える(テスト用の in-memory 実装注入、または明示的な有効化)。
    ///
    /// 注入する実装は必ず fail-closed gate(`FailClosedIndexQuery`)を通したものにすること。
    pub fn with_index_query(mut self, index_query: Arc<dyn IndexQuery>) -> Self {
        self.index_query = Some(index_query);
        self
    }

    /// trust / relation read surface を差し替える(テスト用の in-memory relation 注入、
    /// または明示的な有効化)。
    pub fn with_trust_read(mut self, trust_read: Arc<TrustReadState>) -> Self {
        self.trust_read = Some(trust_read);
        self
    }
}

/// public manifest endpoint 用の最小 state。DB を必要としないため、
/// manifest 単独でテスト・配信できる。
#[derive(Clone)]
pub(crate) struct ManifestState {
    pub(crate) manifest: Option<Arc<CommunityNodeManifest>>,
}

pub async fn build_state(config: &UserApiConfig) -> Result<UserApiState> {
    let pool = connect_postgres(config.database_url.as_str()).await?;
    initialize_database(&pool).await?;
    build_state_from_pool(config, pool).await
}

pub(crate) async fn build_runtime_state(config: &UserApiConfig) -> Result<UserApiState> {
    let pool = connect_postgres(config.database_url.as_str()).await?;
    initialize_database_for_runtime(&pool, DatabaseInitMode::from_env()?).await?;
    build_state_from_pool(config, pool).await
}

async fn build_state_from_pool(config: &UserApiConfig, pool: PgPool) -> Result<UserApiState> {
    let rendezvous_store = TopicRendezvousStore::new(
        config.rendezvous_redis_url.as_str(),
        config.rendezvous_key_prefix.as_str(),
    )?;
    let manifest = load_manifest(config.operator_config_path.as_deref())?;
    let channel_secret_cipher = config
        .channel_secret_key
        .as_deref()
        .map(ChannelSecretCipher::from_key_material)
        .transpose()
        .context("invalid COMMUNITY_NODE_CHANNEL_SECRET_KEY")?
        .map(Arc::new);
    // 有効化の関門（#616）。環境変数が真でも、`cn-cli readiness` の全項目合格記録が
    // 無ければ索引・信頼の読み取り面を公開しない（該当 surface は 404 のまま）。
    // 記録の判定項目集合が現行と不一致（判定基準の変更後）も無効に倒す（安全側）。
    let readiness_activated = if config.index_query_enabled || config.trust_read_enabled {
        match kukuri_cn_core::latest_readiness_activation(&pool).await? {
            Some(activation) if activation.matches_check_ids(&READINESS_CHECK_IDS) => {
                tracing::info!(
                    activated_at = %activation.activated_at.to_rfc3339(),
                    "readiness の有効化記録を確認しました"
                );
                true
            }
            Some(activation) => {
                tracing::warn!(
                    activated_at = %activation.activated_at.to_rfc3339(),
                    "readiness の有効化記録の判定項目集合が現行と一致しないため、                     index / trust の読み取り面を公開しません（`cn-cli readiness` を再実行してください）"
                );
                false
            }
            None => {
                tracing::warn!(
                    "readiness の有効化記録が無いため、index / trust の読み取り面を公開しません                     （`cn-cli readiness` の全項目合格が必要です）"
                );
                false
            }
        }
    } else {
        false
    };

    // ユーザー向け index query(#404)。有効時のみ ArcadeDB(投影)+ Postgres(真実源)を
    // fail-closed gate(`FailClosedIndexQuery`)で束ねる。読み口はこの gate 以外に作らない。
    let index_query: Option<Arc<dyn IndexQuery>> =
        if config.index_query_enabled && readiness_activated {
            let projection = ArcadeDbProjection::new(ArcadeDbConfig::from_env())
                .context("failed to build ArcadeDB client for index query")?;
            let entries = PgIndexEntryStore::new(pool.clone());
            Some(Arc::new(FailClosedIndexQuery::new(
                Arc::new(projection),
                Arc::new(entries),
            )))
        } else {
            None
        };
    // trust / relation read surface(#415)。有効時のみ trust パラメータ(operator 可変)を
    // 検証つきで読み、relation graph(ArcadeDB。`cn-cli relation analyze` が構築する)へ接続する。
    let trust_read: Option<Arc<TrustReadState>> = if config.trust_read_enabled
        && readiness_activated
    {
        let params = TrustParams::from_env().context("invalid COMMUNITY_NODE_TRUST_* params")?;
        let relation = ArcadeDbRelationGraph::new(ArcadeDbConfig::from_env())
            .context("failed to build ArcadeDB client for relation graph")?;
        Some(Arc::new(TrustReadState {
            params,
            relation: Arc::new(relation),
        }))
    } else {
        None
    };
    Ok(UserApiState {
        pool,
        rendezvous_store,
        jwt_config: config.jwt_config.clone(),
        self_node: CommunityNodeBootstrapNode {
            base_url: config.base_url.clone(),
            resolved_urls: CommunityNodeResolvedUrls::new(
                config.public_base_url.clone(),
                config.connectivity_urls.clone(),
                Vec::new(),
            )?,
        },
        manifest,
        channel_secret_cipher,
        index_query,
        trust_read,
    })
}

/// operator config から公開 manifest を構築する。
///
/// config が指定されているのに読込・検証に失敗した場合は起動を失敗させる
/// (運営者の設定ミスを黙って無視せず、明示的に止める)。
fn load_manifest(path: Option<&std::path::Path>) -> Result<Option<Arc<CommunityNodeManifest>>> {
    let Some(path) = path else {
        return Ok(None);
    };
    let yaml = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read operator config at {}", path.display()))?;
    let resolved = load_and_validate(&yaml)
        .with_context(|| format!("invalid operator config at {}", path.display()))?;
    Ok(Some(Arc::new(build_manifest(&resolved))))
}
