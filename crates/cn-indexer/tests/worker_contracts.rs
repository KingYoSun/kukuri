//! #613 T2/T3 常駐ワーカーのループ契約テスト。
//!
//! `KUKURI_CN_RUN_INTEGRATION_TESTS=1` のときだけ実 Postgres（scope 管理 state）に接続して実行する。
//! docs 同期・投影・真実源はメモリ内実装 + mock 安全性プロバイダで、ワーカーのループ契約を固定する:
//! - 起動時にサポート対象を取り込み、レプリカの変更通知で追加の投稿を取り込む。
//! - サポート対象から外れた scope / 秘密鍵が失効した private channel は、次の見直しで索引解除される。
//! - 1 つの scope の失敗が他の scope の取り込みを妨げない（再試行間隔つき）。
//! - 再起動後にサポート対象が復元される。停止後はワーカーが動いていないと観測できる。

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use kukuri_cn_core::TestDatabase;
use kukuri_cn_core::{
    ChannelSecretCipher, IndexScopeKind, MemoryIndexEntryStore, add_supported_topic,
    connect_postgres, initialize_database, register_channel_secret, remove_channel_secret,
    remove_supported_topic,
};
use kukuri_cn_indexer::ingest::IngestPipeline;
use kukuri_cn_indexer::participant::IndexerParticipant;
use kukuri_cn_indexer::projection::{IndexProjection, MemoryIndexProjection};
use kukuri_cn_indexer::state::IndexerRuntimeState;
use kukuri_cn_indexer::worker::{IndexerWorker, WorkerConfig};
use kukuri_cn_safety::{MockSafetyProvider, ModerationEventSigner};
use kukuri_cn_safety_runtime::clock::SystemScanClock;
use kukuri_cn_safety_runtime::id::UuidEventIdGenerator;
use kukuri_cn_safety_runtime::{
    MemorySafetyArtifactStore, SafetyOrchestrator, SafetyScanService,
    Secp256k1ModerationEventSigner,
};
use kukuri_core::{KukuriKeys, ReplicaId, TopicId, build_post_envelope};
use kukuri_docs_sync::{
    DocFetchPolicy, DocOp, DocQuery, DocRecord, DocsSync, MemoryDocsSync, stable_key,
};
use sqlx::postgres::PgPool;

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";
const TEST_SIGNER_SECRET: &str = "0000000000000000000000000000000000000000000000000000000000000001";
const TEST_CIPHER_KEY: &str = "worker-contract-test-channel-secret-key-0123456789";
const TEST_NAMESPACE_SECRET: &str =
    "0303030303030303030303030303030303030303030303030303030303030303";

fn integration_test_admin_database_url() -> Option<String> {
    kukuri_test_support::gated_env_url(
        "KUKURI_CN_RUN_INTEGRATION_TESTS",
        "COMMUNITY_NODE_DATABASE_URL",
        DEFAULT_ADMIN_DATABASE_URL,
    )
}

fn cipher() -> ChannelSecretCipher {
    ChannelSecretCipher::from_key_material(TEST_CIPHER_KEY).expect("cipher")
}

/// participant を組む（docs 同期は差し替え可能）。真実源（メモリ内実装）は scan service と
/// 同じ artifact store を参照し、verdict への外部キー相当を成立させる。
fn participant_with_docs(
    pool: &PgPool,
    docs: Arc<dyn DocsSync>,
    projection: &Arc<MemoryIndexProjection>,
    state: &Arc<IndexerRuntimeState>,
) -> (Arc<IndexerParticipant>, Arc<MemoryIndexEntryStore>) {
    let signer = Secp256k1ModerationEventSigner::from_secret(TEST_SIGNER_SECRET).expect("signer");
    let issuer = signer.issuer_node_id().to_string();
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let orchestrator = SafetyOrchestrator::builder(
        &issuer,
        Arc::new(SystemScanClock),
        Arc::new(UuidEventIdGenerator),
    )
    .provider(Arc::new(MockSafetyProvider::known_csam("mock-known-csam")))
    .build()
    .expect("orchestrator");
    let service = Arc::new(
        SafetyScanService::builder(Arc::new(orchestrator), store.clone())
            .signer(Arc::new(signer))
            .build()
            .expect("service"),
    );
    let entries = Arc::new(MemoryIndexEntryStore::new(store));
    let pipeline = IngestPipeline::new(docs.clone(), service, entries.clone(), projection.clone())
        .with_metrics(Arc::clone(state));
    let participant = Arc::new(IndexerParticipant::new(
        pool.clone(),
        docs,
        entries.clone(),
        projection.clone(),
        pipeline,
        cipher(),
    ));
    (participant, entries)
}

/// テスト用に間隔を短縮したワーカー設定。
fn fast_config(poll_interval: Duration) -> WorkerConfig {
    WorkerConfig {
        poll_interval,
        event_debounce: Duration::from_millis(50),
        backoff_base: Duration::from_millis(100),
        backoff_max: Duration::from_millis(500),
    }
}

/// 本文 text の post envelope を共有 replica に実在させ、object_id を返す。
async fn persist_post(
    docs: &dyn DocsSync,
    replica: &ReplicaId,
    topic: &TopicId,
    body: &str,
) -> String {
    let keys = KukuriKeys::generate();
    let envelope = build_post_envelope(&keys, topic, body, None).expect("envelope");
    let object = envelope
        .to_post_object()
        .expect("post object")
        .expect("post object present");
    let object_id = object.object_id.as_str().to_string();
    docs.open_replica(replica).await.expect("open");
    docs.apply_doc_op(
        replica,
        DocOp::SetJson {
            key: stable_key("objects", &format!("{object_id}/state")),
            value: serde_json::to_value(&object).expect("state json"),
        },
    )
    .await
    .expect("state op");
    docs.apply_doc_op(
        replica,
        DocOp::SetJson {
            key: stable_key("objects", &format!("{object_id}/envelope")),
            value: serde_json::to_value(&envelope).expect("envelope json"),
        },
    )
    .await
    .expect("envelope op");
    object_id
}

/// 条件が成立するまで待つ（最長 30 秒。CI の並行負荷を考慮して余裕を持たせる）。
async fn wait_until<F, Fut>(what: &str, mut condition: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..600 {
        if condition().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("timed out waiting for {what}");
}

#[tokio::test]
async fn worker_ingests_on_startup_and_reacts_to_replica_events() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping worker contract test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_worker_events").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    initialize_database(&pool).await?;

    add_supported_topic(&pool, IndexScopeKind::PublicTopic, "rust").await?;
    let topic = TopicId::new("rust".to_string());
    let replica = kukuri_docs_sync::topic_replica_id("rust");
    let docs = Arc::new(MemoryDocsSync::default());
    persist_post(docs.as_ref(), &replica, &topic, "hello worker").await;

    let state = Arc::new(IndexerRuntimeState::default());
    let projection = Arc::new(MemoryIndexProjection::default());
    let (participant, entries) = participant_with_docs(&pool, docs.clone(), &projection, &state);

    // 定期見直しを長くして、2 件目が「変更通知」経由で取り込まれることを確かめる。
    let worker = IndexerWorker::new(
        participant,
        docs.clone(),
        Arc::clone(&state),
        fast_config(Duration::from_secs(120)),
    );
    let handle = worker.spawn();

    // 起動直後の 1 巡でサポート対象が取り込まれる。
    wait_until("initial ingest", || {
        let projection = projection.clone();
        async move {
            projection
                .count_scope(IndexScopeKind::PublicTopic, "rust")
                .await
                .unwrap_or(0)
                == 1
        }
    })
    .await;
    assert!(state.snapshot().worker_running);
    assert!(state.snapshot().last_sync_at.is_some());
    assert_eq!(state.snapshot().opened_scopes, 1);

    // レプリカの変更通知で 2 件目が取り込まれる（定期見直しはまだ先）。
    let second = persist_post(docs.as_ref(), &replica, &topic, "event driven post").await;
    wait_until("event driven ingest", || {
        let projection = projection.clone();
        let second = second.clone();
        async move {
            projection
                .contains_object(IndexScopeKind::PublicTopic, "rust", second.as_str())
                .await
                .unwrap_or(false)
        }
    })
    .await;
    assert!(entries.contains(IndexScopeKind::PublicTopic, "rust", second.as_str()));

    handle.shutdown().await;
    assert!(!state.snapshot().worker_running);
    Ok(())
}

#[tokio::test]
async fn removed_scope_is_deindexed_on_next_pass() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping worker contract test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_worker_deindex").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    initialize_database(&pool).await?;

    add_supported_topic(&pool, IndexScopeKind::PublicTopic, "rust").await?;
    let topic = TopicId::new("rust".to_string());
    let replica = kukuri_docs_sync::topic_replica_id("rust");
    let docs = Arc::new(MemoryDocsSync::default());
    let object_id = persist_post(docs.as_ref(), &replica, &topic, "to be removed").await;

    let state = Arc::new(IndexerRuntimeState::default());
    let projection = Arc::new(MemoryIndexProjection::default());
    let (participant, entries) = participant_with_docs(&pool, docs.clone(), &projection, &state);
    let worker = IndexerWorker::new(
        participant,
        docs.clone(),
        Arc::clone(&state),
        fast_config(Duration::from_millis(200)),
    );
    let handle = worker.spawn();

    wait_until("initial ingest", || {
        let projection = projection.clone();
        let object_id = object_id.clone();
        async move {
            projection
                .contains_object(IndexScopeKind::PublicTopic, "rust", object_id.as_str())
                .await
                .unwrap_or(false)
        }
    })
    .await;

    // サポート対象から外すと、次の見直しで真実源 → 投影の順に索引解除される。
    remove_supported_topic(&pool, IndexScopeKind::PublicTopic, "rust").await?;
    wait_until("scope de-index", || {
        let projection = projection.clone();
        async move {
            projection
                .count_scope(IndexScopeKind::PublicTopic, "rust")
                .await
                .unwrap_or(usize::MAX)
                == 0
        }
    })
    .await;
    assert!(!entries.contains(IndexScopeKind::PublicTopic, "rust", object_id.as_str()));

    handle.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn revoked_channel_secret_deindexes_private_channel() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping worker contract test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_worker_secret").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    initialize_database(&pool).await?;

    // 購読リクエスト経路の結果（サポート対象入り + 秘密鍵登録済み）を直接作る。
    add_supported_topic(&pool, IndexScopeKind::PrivateChannel, "secret-room").await?;
    register_channel_secret(&pool, &cipher(), "secret-room", TEST_NAMESPACE_SECRET).await?;

    let topic = TopicId::new("secret-room".to_string());
    let replica = kukuri_docs_sync::private_channel_replica_id("secret-room");
    let docs = Arc::new(MemoryDocsSync::default());
    // 投稿の下準備として、docs 側にも capability を登録してから replica を開く
    // （本番ではワーカーの restore_scopes が登録する。ここでは投稿を先に置くため）。
    docs.register_private_replica_secret(&replica, TEST_NAMESPACE_SECRET)
        .await?;
    let object_id = persist_post(docs.as_ref(), &replica, &topic, "private post").await;

    let state = Arc::new(IndexerRuntimeState::default());
    let projection = Arc::new(MemoryIndexProjection::default());
    let (participant, entries) = participant_with_docs(&pool, docs.clone(), &projection, &state);
    let worker = IndexerWorker::new(
        participant,
        docs.clone(),
        Arc::clone(&state),
        fast_config(Duration::from_millis(200)),
    );
    let handle = worker.spawn();

    // 秘密鍵が登録済みの private channel は公開トピックと同じループで取り込まれる。
    wait_until("private channel ingest", || {
        let projection = projection.clone();
        let object_id = object_id.clone();
        async move {
            projection
                .contains_object(
                    IndexScopeKind::PrivateChannel,
                    "secret-room",
                    object_id.as_str(),
                )
                .await
                .unwrap_or(false)
        }
    })
    .await;

    // 秘密鍵の失効で、次の見直しで索引解除される（鍵が無ければ索引しない）。
    remove_channel_secret(&pool, "secret-room").await?;
    wait_until("private channel de-index", || {
        let projection = projection.clone();
        async move {
            projection
                .count_scope(IndexScopeKind::PrivateChannel, "secret-room")
                .await
                .unwrap_or(usize::MAX)
                == 0
        }
    })
    .await;
    assert!(!entries.contains(
        IndexScopeKind::PrivateChannel,
        "secret-room",
        object_id.as_str()
    ));

    handle.shutdown().await;
    Ok(())
}

/// 特定 replica の走査だけ失敗する docs 同期（他 scope の取り込みを妨げない検証用）。
struct FailingScopeDocsSync {
    inner: Arc<MemoryDocsSync>,
    failing_replica: String,
}

#[async_trait]
impl DocsSync for FailingScopeDocsSync {
    async fn open_replica(&self, replica_id: &ReplicaId) -> anyhow::Result<()> {
        self.inner.open_replica(replica_id).await
    }

    async fn apply_doc_op(&self, replica_id: &ReplicaId, op: DocOp) -> anyhow::Result<()> {
        self.inner.apply_doc_op(replica_id, op).await
    }

    async fn query_replica_with_policy(
        &self,
        replica_id: &ReplicaId,
        query: DocQuery,
        policy: DocFetchPolicy,
    ) -> anyhow::Result<Vec<DocRecord>> {
        if replica_id.as_str() == self.failing_replica {
            anyhow::bail!("simulated replica query failure");
        }
        self.inner
            .query_replica_with_policy(replica_id, query, policy)
            .await
    }

    async fn subscribe_replica(
        &self,
        replica_id: &ReplicaId,
    ) -> anyhow::Result<kukuri_docs_sync::DocEventStream> {
        self.inner.subscribe_replica(replica_id).await
    }

    async fn import_peer_ticket(&self, ticket: &str) -> anyhow::Result<()> {
        self.inner.import_peer_ticket(ticket).await
    }
}

#[tokio::test]
async fn failing_scope_backs_off_without_blocking_others() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping worker contract test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_worker_backoff").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    initialize_database(&pool).await?;

    add_supported_topic(&pool, IndexScopeKind::PublicTopic, "healthy").await?;
    add_supported_topic(&pool, IndexScopeKind::PublicTopic, "broken").await?;

    let memory = Arc::new(MemoryDocsSync::default());
    let healthy_topic = TopicId::new("healthy".to_string());
    let healthy_replica = kukuri_docs_sync::topic_replica_id("healthy");
    let object_id = persist_post(
        memory.as_ref(),
        &healthy_replica,
        &healthy_topic,
        "still works",
    )
    .await;
    let docs: Arc<dyn DocsSync> = Arc::new(FailingScopeDocsSync {
        inner: memory.clone(),
        failing_replica: kukuri_docs_sync::topic_replica_id("broken")
            .as_str()
            .to_string(),
    });

    let state = Arc::new(IndexerRuntimeState::default());
    let projection = Arc::new(MemoryIndexProjection::default());
    let (participant, _entries) = participant_with_docs(&pool, docs.clone(), &projection, &state);
    let worker = IndexerWorker::new(
        participant,
        docs,
        Arc::clone(&state),
        fast_config(Duration::from_millis(200)),
    );
    let handle = worker.spawn();

    // 壊れた scope があっても健全な scope は取り込まれ、ワーカーは動き続ける。
    wait_until("healthy scope ingest", || {
        let projection = projection.clone();
        let object_id = object_id.clone();
        async move {
            projection
                .contains_object(IndexScopeKind::PublicTopic, "healthy", object_id.as_str())
                .await
                .unwrap_or(false)
        }
    })
    .await;
    wait_until("failure is observable", || {
        let state = Arc::clone(&state);
        async move {
            let snapshot = state.snapshot();
            snapshot.worker_running
                && snapshot.last_error.is_some()
                && snapshot.last_error_scope.as_deref() == Some("topic::broken")
        }
    })
    .await;

    handle.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn worker_restart_restores_supported_scopes() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping worker contract test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_worker_restart").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    initialize_database(&pool).await?;

    add_supported_topic(&pool, IndexScopeKind::PublicTopic, "rust").await?;
    let topic = TopicId::new("rust".to_string());
    let replica = kukuri_docs_sync::topic_replica_id("rust");
    let docs = Arc::new(MemoryDocsSync::default());
    let object_id = persist_post(docs.as_ref(), &replica, &topic, "survives restart").await;

    let projection = Arc::new(MemoryIndexProjection::default());

    // 1 回目のワーカー: 取り込んで停止する。
    let first_state = Arc::new(IndexerRuntimeState::default());
    let (participant, _entries) =
        participant_with_docs(&pool, docs.clone(), &projection, &first_state);
    let first = IndexerWorker::new(
        participant,
        docs.clone(),
        Arc::clone(&first_state),
        fast_config(Duration::from_millis(200)),
    );
    let first_handle = first.spawn();
    wait_until("first ingest", || {
        let projection = projection.clone();
        let object_id = object_id.clone();
        async move {
            projection
                .contains_object(IndexScopeKind::PublicTopic, "rust", object_id.as_str())
                .await
                .unwrap_or(false)
        }
    })
    .await;
    first_handle.shutdown().await;
    assert!(!first_state.snapshot().worker_running);

    // 2 回目のワーカー: scope 管理 state からサポート対象が復元され、取り込みが再開する。
    let second_state = Arc::new(IndexerRuntimeState::default());
    let (participant, _entries) =
        participant_with_docs(&pool, docs.clone(), &projection, &second_state);
    let second = IndexerWorker::new(
        participant,
        docs.clone(),
        Arc::clone(&second_state),
        fast_config(Duration::from_millis(200)),
    );
    let second_handle = second.spawn();
    wait_until("restart restore", || {
        let state = Arc::clone(&second_state);
        async move {
            let snapshot = state.snapshot();
            snapshot.worker_running && snapshot.opened_scopes == 1 && snapshot.indexed >= 1
        }
    })
    .await;

    second_handle.shutdown().await;
    Ok(())
}
