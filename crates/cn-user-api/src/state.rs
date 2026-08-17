//! user-api の実行時 state(DI)と構築。

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use kukuri_cn_core::{
    ChannelSecretCipher, DatabaseInitMode, JwtConfig, PgIndexEntryStore, TopicRendezvousStore,
    connect_postgres, initialize_database, initialize_database_for_runtime,
    latest_readiness_activation, readiness_context_fingerprint,
};
use kukuri_cn_indexer::{
    ArcadeDbConfig, ArcadeDbProjection, ArcadeDbRelationGraph, FailClosedIndexQuery, IndexQuery,
};
use kukuri_cn_operator::{
    CommunityNodeManifest, READINESS_CHECK_IDS, build_manifest, generate_all, load_and_validate,
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
    /// manifest が指す公開開示文書。operator config と同じ入力から決定論的に生成する。
    pub(crate) public_disclosures: Arc<BTreeMap<String, String>>,
    /// private channel の indexing request で受け取る channel secret を at-rest 暗号化する cipher。
    /// 鍵 material(`COMMUNITY_NODE_CHANNEL_SECRET_KEY`)が未設定なら None で、private channel の
    /// indexing request は受け付けない(secret を平文保存しないため)。
    pub(crate) channel_secret_cipher: Option<Arc<ChannelSecretCipher>>,
    /// ユーザー向け search / discovery / recommendation の query 境界(#404)。
    /// fail-closed query gate(`FailClosedIndexQuery`)を通した読み口のみを持つ。
    /// None = 設定無効。readiness activation は起動後も変化するため、各requestで検査する。
    pub(crate) index_query: Option<Arc<dyn IndexQuery>>,
    /// trust / relation read surface(#415 / ADR 0026)。
    /// None = 設定無効。readiness activation は起動後も変化するため、各requestで検査する。
    pub(crate) trust_read: Option<Arc<TrustReadState>>,
    /// user / post surfacing に適用する node-local distance opt-out 判定依存。
    pub(crate) relation_visibility: Option<Arc<RelationVisibilityState>>,
    readiness_activation_requirement: Option<ReadinessActivationRequirement>,
}

#[derive(Clone)]
struct ReadinessActivationRequirement {
    profile: String,
    context_fingerprint: String,
    max_age: chrono::Duration,
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

/// trust read の有無から独立した distance opt-out の判定依存。
pub struct RelationVisibilityState {
    pub relation: Arc<dyn RelationStore>,
    pub min_proximity: f64,
}

impl RelationVisibilityState {
    pub fn new(relation: Arc<dyn RelationStore>, min_proximity: f64) -> Result<Self> {
        if !min_proximity.is_finite() || min_proximity <= 0.0 || min_proximity > 1.0 {
            anyhow::bail!("relation distance opt-out min proximity must be within (0, 1]");
        }
        Ok(Self {
            relation,
            min_proximity,
        })
    }
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

    /// distance opt-out 判定依存を差し替える（テスト用、または明示的な有効化）。
    pub fn with_relation_visibility(
        mut self,
        relation_visibility: Arc<RelationVisibilityState>,
    ) -> Self {
        self.relation_visibility = Some(relation_visibility);
        self
    }

    /// 起動後もactivationの期限・失効を各read requestで再確認する。
    pub(crate) async fn readiness_activation_is_valid(&self) -> bool {
        let Some(requirement) = self.readiness_activation_requirement.as_ref() else {
            return true;
        };
        match activation_is_valid(&self.pool, requirement).await {
            Ok(valid) => valid,
            Err(error) => {
                tracing::warn!(error = %format!("{error:#}"), "readiness activationの再確認に失敗しました");
                false
            }
        }
    }
}

/// public manifest endpoint 用の最小 state。DB を必要としないため、
/// manifest 単独でテスト・配信できる。
#[derive(Clone)]
pub(crate) struct ManifestState {
    pub(crate) manifest: Option<Arc<CommunityNodeManifest>>,
    pub(crate) public_disclosures: Arc<BTreeMap<String, String>>,
}

struct LoadedManifest {
    manifest: Option<Arc<CommunityNodeManifest>>,
    public_disclosures: Arc<BTreeMap<String, String>>,
    operator_config_yaml: Vec<u8>,
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
    let LoadedManifest {
        manifest,
        public_disclosures,
        operator_config_yaml,
    } = load_manifest(config.operator_config_path.as_deref())?;
    if let (Some(manifest), Some(expected)) = (
        manifest.as_deref(),
        config.expected_issuer_node_id.as_deref(),
    ) {
        ensure_manifest_node_id_matches_issuer(manifest.node_id.as_str(), expected)?;
    }
    let channel_secret_cipher = config
        .channel_secret_key
        .as_deref()
        .map(ChannelSecretCipher::from_key_material)
        .transpose()
        .context("invalid COMMUNITY_NODE_CHANNEL_SECRET_KEY")?
        .map(Arc::new);
    // 有効化の関門（#616）。activation は起動後にtimerが作成・更新・失効するため、
    // backendの構成有無とは分離し、各requestで現在値を検査する。ここでは起動時点の状態を
    // 運用logへ残すだけにする。
    let readiness_activation_requirement =
        if config.index_query_enabled || config.trust_read_enabled {
            Some(ReadinessActivationRequirement {
                profile: "public-node".to_string(),
                context_fingerprint: readiness_context_fingerprint(
                    "public-node",
                    &config.deployment_revision,
                    &operator_config_yaml,
                ),
                max_age: chrono::Duration::seconds(
                    i64::try_from(config.readiness_activation_max_age_secs).unwrap_or(i64::MAX),
                ),
            })
        } else {
            None
        };
    if let Some(requirement) = readiness_activation_requirement.as_ref() {
        match latest_readiness_activation(&pool).await? {
            Some(activation)
                if activation.is_valid(
                    &requirement.profile,
                    &READINESS_CHECK_IDS,
                    &requirement.context_fingerprint,
                    chrono::Utc::now(),
                    requirement.max_age,
                ) =>
            {
                tracing::info!(
                    activated_at = %activation.activated_at.to_rfc3339(),
                    "readiness の有効化記録を確認しました"
                );
            }
            Some(activation) => {
                tracing::warn!(
                    activated_at = %activation.activated_at.to_rfc3339(),
                    "readiness の有効化記録が現在のprofile/config/deploy/期限と一致しないため、                     index / trust の読み取り面はrequest時に拒否されます（`cn-cli readiness` を再実行してください）"
                );
            }
            None => {
                tracing::warn!(
                    "readiness の有効化記録が無いため、index / trust の読み取り面はrequest時に拒否されます                     （`cn-cli readiness` の全項目合格が必要です）"
                );
            }
        }
    }

    // ユーザー向け index query(#404)。有効時のみ ArcadeDB(投影)+ Postgres(真実源)を
    // fail-closed gate(`FailClosedIndexQuery`)で束ねる。読み口はこの gate 以外に作らない。
    let index_query: Option<Arc<dyn IndexQuery>> = if config.index_query_enabled {
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
    // index / relation の表示制御で共有する relation graph。trust read が無効でも
    // index surface の distance opt-out 判定には必要となる。
    let relation_visibility: Option<Arc<RelationVisibilityState>> =
        if config.index_query_enabled || config.trust_read_enabled {
            let min_proximity = config
                .relation_distance_optout_min_proximity
                .context("relation distance opt-out policy is required for index/trust surfaces")?;
            let relation = ArcadeDbRelationGraph::new(ArcadeDbConfig::from_env())
                .context("failed to build ArcadeDB client for relation visibility")?;
            Some(Arc::new(RelationVisibilityState::new(
                Arc::new(relation),
                min_proximity,
            )?))
        } else {
            None
        };
    // trust / relation read surface(#415)。有効時のみ trust パラメータ(operator 可変)を
    // 検証つきで読み、relation graph(ArcadeDB。`cn-cli relation analyze` が構築する)へ接続する。
    let trust_read: Option<Arc<TrustReadState>> = if config.trust_read_enabled {
        let params = TrustParams::from_env().context("invalid COMMUNITY_NODE_TRUST_* params")?;
        let relation = relation_visibility
            .as_ref()
            .context("relation visibility must be configured for trust reads")?
            .relation
            .clone();
        Some(Arc::new(TrustReadState { params, relation }))
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
        public_disclosures,
        channel_secret_cipher,
        index_query,
        trust_read,
        relation_visibility,
        readiness_activation_requirement,
    })
}

async fn activation_is_valid(
    pool: &PgPool,
    requirement: &ReadinessActivationRequirement,
) -> Result<bool> {
    Ok(latest_readiness_activation(pool)
        .await?
        .is_some_and(|activation| {
            activation.is_valid(
                &requirement.profile,
                &READINESS_CHECK_IDS,
                &requirement.context_fingerprint,
                chrono::Utc::now(),
                requirement.max_age,
            )
        }))
}

/// 公開ノード情報の `node_id` が、この配備がリスク判定に載せる `issuer_node_id`
/// (署名鍵の公開鍵 hex、または明示指定)と一致することを起動時に強制する(#706)。
///
/// 異議申し立てはサーバ(`/v1/report`)もクライアントも `manifest.node_id == issuer_node_id`
/// を前提にするため、不一致のまま起動すると本番で異議申し立てが端から端まで成立しない。
/// 運用者設定の誤りを黙って通さず、理由を明示して起動を止める。
pub(crate) fn ensure_manifest_node_id_matches_issuer(
    manifest_node_id: &str,
    expected_issuer_node_id: &str,
) -> Result<()> {
    let manifest_node_id = manifest_node_id.trim();
    let expected = expected_issuer_node_id.trim();
    if manifest_node_id == expected {
        return Ok(());
    }
    anyhow::bail!(
        "operator config の server.node_id (`{manifest_node_id}`) が、この配備のモデレーション事象の \
         発行元識別子 (`{expected}`; COMMUNITY_NODE_SAFETY_SIGNING_KEY の公開鍵 hex、または \
         COMMUNITY_NODE_SAFETY_ISSUER_NODE_ID) と一致しません。異議申し立てが受理されなくなるため \
         起動を拒否します。`cn-cli moderation issuer-node-id` で導出した値を server.node_id に \
         記入してください"
    );
}

/// operator config から公開 manifest を構築する。
///
/// config が指定されているのに読込・検証に失敗した場合は起動を失敗させる
/// (運営者の設定ミスを黙って無視せず、明示的に止める)。
fn load_manifest(path: Option<&std::path::Path>) -> Result<LoadedManifest> {
    let Some(path) = path else {
        return Ok(LoadedManifest {
            manifest: None,
            public_disclosures: Arc::new(BTreeMap::new()),
            operator_config_yaml: Vec::new(),
        });
    };
    let yaml = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read operator config at {}", path.display()))?;
    let resolved = load_and_validate(&yaml)
        .with_context(|| format!("invalid operator config at {}", path.display()))?;
    let bytes = yaml.as_bytes().to_vec();
    let public_disclosures = generate_all(&resolved)
        .into_iter()
        .filter(|file| {
            matches!(
                file.filename.as_str(),
                "terms.md"
                    | "privacy-policy.md"
                    | "external-transmission-notice.md"
                    | "moderation-policy.md"
                    | "abuse-policy.md"
                    | "data-retention-policy.md"
            )
        })
        .map(|file| (file.filename, file.content))
        .collect();
    Ok(LoadedManifest {
        manifest: Some(Arc::new(build_manifest(&resolved))),
        public_disclosures: Arc::new(public_disclosures),
        operator_config_yaml: bytes,
    })
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use anyhow::Result;
    use tempfile::NamedTempFile;

    use super::load_manifest;

    #[test]
    fn manifest_node_id_must_match_expected_issuer() {
        assert!(super::ensure_manifest_node_id_matches_issuer("node-a", "node-a").is_ok());
        assert!(super::ensure_manifest_node_id_matches_issuer(" node-a ", "node-a").is_ok());
        let err = super::ensure_manifest_node_id_matches_issuer("", "79be66").unwrap_err();
        assert!(err.to_string().contains("server.node_id"), "{err}");
        assert!(err.to_string().contains("79be66"), "{err}");
        assert!(super::ensure_manifest_node_id_matches_issuer("node-a", "node-b").is_err());
    }

    #[test]
    fn load_manifest_includes_every_http_disclosure() -> Result<()> {
        let mut config = NamedTempFile::new()?;
        write!(
            config,
            "server:\n  domain: example-kukuri.net\n  operator_name: Example Operator\n  country: JP\nprofile: relay-enabled\nacknowledge_planned_capabilities: true\n"
        )?;

        let loaded = load_manifest(Some(config.path()))?;
        for filename in [
            "terms.md",
            "privacy-policy.md",
            "external-transmission-notice.md",
            "moderation-policy.md",
            "abuse-policy.md",
            "data-retention-policy.md",
        ] {
            assert!(
                loaded.public_disclosures.contains_key(filename),
                "missing public disclosure: {filename}"
            );
        }
        Ok(())
    }
}
