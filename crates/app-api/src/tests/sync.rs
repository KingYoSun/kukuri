use super::*;
use kukuri_core::{BlobHash, ChannelAudienceKind, CreatePrivateChannelInput, KukuriKeys, TopicId};
use std::collections::HashMap;

#[derive(Clone, Default)]
struct CountingDocsSync {
    inner: kukuri_docs_sync::MemoryDocsSync,
    queries: Arc<TokioMutex<Vec<(String, DocQuery)>>>,
    assist_peer_ids: Vec<String>,
}

impl CountingDocsSync {
    fn with_assist_peer_ids(peer_ids: Vec<&str>) -> Self {
        Self {
            assist_peer_ids: peer_ids.into_iter().map(str::to_string).collect(),
            ..Self::default()
        }
    }

    async fn clear_queries(&self) {
        self.queries.lock().await.clear();
    }

    async fn queries(&self) -> Vec<(String, DocQuery)> {
        self.queries.lock().await.clone()
    }
}

#[async_trait]
impl DocsSync for CountingDocsSync {
    async fn open_replica(&self, replica_id: &ReplicaId) -> Result<()> {
        self.inner.open_replica(replica_id).await
    }

    async fn register_private_replica_secret(
        &self,
        replica_id: &ReplicaId,
        namespace_secret_hex: &str,
    ) -> Result<()> {
        self.inner
            .register_private_replica_secret(replica_id, namespace_secret_hex)
            .await
    }

    async fn remove_private_replica_secret(&self, replica_id: &ReplicaId) -> Result<()> {
        self.inner.remove_private_replica_secret(replica_id).await
    }

    async fn apply_doc_op(&self, replica_id: &ReplicaId, op: DocOp) -> Result<()> {
        self.inner.apply_doc_op(replica_id, op).await
    }

    async fn query_replica_with_policy(
        &self,
        replica_id: &ReplicaId,
        query: DocQuery,
        _policy: kukuri_docs_sync::DocFetchPolicy,
    ) -> Result<Vec<kukuri_docs_sync::DocRecord>> {
        self.queries
            .lock()
            .await
            .push((replica_id.as_str().to_string(), query.clone()));
        self.inner.query_replica(replica_id, query).await
    }

    async fn subscribe_replica(
        &self,
        replica_id: &ReplicaId,
    ) -> Result<kukuri_docs_sync::DocEventStream> {
        self.inner.subscribe_replica(replica_id).await
    }

    async fn import_peer_ticket(&self, ticket: &str) -> Result<()> {
        self.inner.import_peer_ticket(ticket).await
    }

    async fn assist_peer_ids(&self) -> Result<Vec<String>> {
        Ok(self.assist_peer_ids.clone())
    }
}

#[derive(Clone, Default)]
struct HangingRemoteOnMissDocsSync {
    inner: kukuri_docs_sync::MemoryDocsSync,
}

#[async_trait]
impl DocsSync for HangingRemoteOnMissDocsSync {
    async fn open_replica(&self, replica_id: &ReplicaId) -> Result<()> {
        self.inner.open_replica(replica_id).await
    }

    async fn register_private_replica_secret(
        &self,
        replica_id: &ReplicaId,
        namespace_secret_hex: &str,
    ) -> Result<()> {
        self.inner
            .register_private_replica_secret(replica_id, namespace_secret_hex)
            .await
    }

    async fn remove_private_replica_secret(&self, replica_id: &ReplicaId) -> Result<()> {
        self.inner.remove_private_replica_secret(replica_id).await
    }

    async fn apply_doc_op(&self, replica_id: &ReplicaId, op: DocOp) -> Result<()> {
        self.inner.apply_doc_op(replica_id, op).await
    }

    async fn query_replica_with_policy(
        &self,
        replica_id: &ReplicaId,
        query: DocQuery,
        policy: kukuri_docs_sync::DocFetchPolicy,
    ) -> Result<Vec<kukuri_docs_sync::DocRecord>> {
        let records = self.inner.query_replica(replica_id, query).await?;
        if records.is_empty() && policy == kukuri_docs_sync::DocFetchPolicy::LocalThenRemote {
            sleep(Duration::from_secs(30)).await;
        }
        Ok(records)
    }

    async fn subscribe_replica(
        &self,
        replica_id: &ReplicaId,
    ) -> Result<kukuri_docs_sync::DocEventStream> {
        self.inner.subscribe_replica(replica_id).await
    }

    async fn import_peer_ticket(&self, ticket: &str) -> Result<()> {
        self.inner.import_peer_ticket(ticket).await
    }
}

#[derive(Clone, Default)]
struct DelayedBlobService {
    inner: MemoryBlobService,
    remaining_misses: Arc<TokioMutex<HashMap<String, usize>>>,
}

impl DelayedBlobService {
    async fn delay_hash(&self, hash: &BlobHash, misses: usize) {
        self.remaining_misses
            .lock()
            .await
            .insert(hash.as_str().to_string(), misses);
    }
}

#[async_trait]
impl BlobService for DelayedBlobService {
    async fn put_blob(&self, data: Vec<u8>, mime: &str) -> Result<StoredBlob> {
        self.inner.put_blob(data, mime).await
    }

    async fn fetch_blob(&self, hash: &BlobHash) -> Result<Option<Vec<u8>>> {
        let mut guard = self.remaining_misses.lock().await;
        if let Some(remaining) = guard.get_mut(hash.as_str())
            && *remaining > 0
        {
            *remaining -= 1;
            return Ok(None);
        }
        drop(guard);
        self.inner.fetch_blob(hash).await
    }

    async fn pin_blob(&self, hash: &BlobHash) -> Result<()> {
        self.inner.pin_blob(hash).await
    }

    async fn blob_status(&self, hash: &BlobHash) -> Result<BlobStatus> {
        if self
            .remaining_misses
            .lock()
            .await
            .get(hash.as_str())
            .copied()
            .unwrap_or_default()
            > 0
        {
            return Ok(BlobStatus::Missing);
        }
        self.inner.blob_status(hash).await
    }

    async fn import_peer_ticket(&self, ticket: &str) -> Result<()> {
        self.inner.import_peer_ticket(ticket).await
    }
}

async fn iroh_sync_diagnostics(
    app_a: &AppService,
    app_b: &AppService,
    stack_a: &TestIrohStack,
    stack_b: &TestIrohStack,
    topic: &str,
) -> String {
    let snapshot_a = app_a
        .get_sync_status()
        .await
        .map(|status| format_sync_snapshot(&status, topic))
        .unwrap_or_else(|error| format!("failed to read sync status a: {error}"));
    let snapshot_b = app_b
        .get_sync_status()
        .await
        .map(|status| format_sync_snapshot(&status, topic))
        .unwrap_or_else(|error| format!("failed to read sync status b: {error}"));
    let timeline_a = app_a
        .list_timeline(topic, None, 20)
        .await
        .map(|timeline| {
            timeline
                .items
                .into_iter()
                .map(|post| post.object_id)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|error| vec![format!("timeline a error: {error}")]);
    let timeline_b = app_b
        .list_timeline(topic, None, 20)
        .await
        .map(|timeline| {
            timeline
                .items
                .into_iter()
                .map(|post| post.object_id)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|error| vec![format!("timeline b error: {error}")]);
    let notifications_a = app_a
        .list_notifications()
        .await
        .map(|items| {
            items
                .into_iter()
                .map(|item| {
                    format!(
                        "{}:{:?}:{}",
                        item.notification_id,
                        item.kind,
                        item.object_id.unwrap_or_else(|| "-".into())
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|error| vec![format!("notifications a error: {error}")]);
    let remote_info_a = stack_a
        ._node
        .endpoint()
        .remote_info(stack_b._node.endpoint().id())
        .await
        .is_some();
    let remote_info_b = stack_b
        ._node
        .endpoint()
        .remote_info(stack_a._node.endpoint().id())
        .await
        .is_some();
    format!(
        "snapshot_a={snapshot_a}; snapshot_b={snapshot_b}; remote_info_a={remote_info_a}; remote_info_b={remote_info_b}; timeline_a={timeline_a:?}; timeline_b={timeline_b:?}; notifications_a={notifications_a:?}"
    )
}

fn app_with_hanging_remote_docs(
    store: Arc<MemoryStore>,
    docs_sync: Arc<HangingRemoteOnMissDocsSync>,
    blob_service: Arc<MemoryBlobService>,
    keys: KukuriKeys,
) -> AppService {
    AppService::new_with_services(
        store.clone(),
        store,
        Arc::new(StaticTransport::new(PeerSnapshot::default())),
        Arc::new(NoopHintTransport),
        docs_sync,
        blob_service,
        keys,
    )
}

#[tokio::test]
async fn tracking_multiple_topics_updates_sync_status() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
    let app = AppService::new(store, transport);

    let _ = app
        .list_timeline("kukuri:topic:one", None, 10)
        .await
        .expect("timeline one");
    let _ = app
        .list_timeline("kukuri:topic:two", None, 10)
        .await
        .expect("timeline two");
    let status = app.get_sync_status().await.expect("sync status");

    assert!(
        status
            .subscribed_topics
            .iter()
            .any(|topic| topic == "kukuri:topic:one")
    );
    assert!(
        status
            .subscribed_topics
            .iter()
            .any(|topic| topic == "kukuri:topic:two")
    );
    assert!(
        status
            .topic_diagnostics
            .iter()
            .any(|topic| topic.topic == "kukuri:topic:one")
    );
    assert!(
        status
            .topic_diagnostics
            .iter()
            .any(|topic| topic.topic == "kukuri:topic:two")
    );
    assert_eq!(status.status_detail, "No peers configured");
    assert!(
        status
            .topic_diagnostics
            .iter()
            .all(|topic| !topic.status_detail.is_empty())
    );
    assert!(
        status
            .topic_diagnostics
            .iter()
            .all(|topic| topic.last_error.is_none())
    );
}

#[tokio::test]
async fn local_only_bootstrap_reads_return_empty_without_remote_docs() {
    let docs_sync = Arc::new(HangingRemoteOnMissDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let store = Arc::new(MemoryStore::default());
    let app = app_with_hanging_remote_docs(store, docs_sync, blob_service, generate_keys());
    let topic = "kukuri:topic:local-only-empty";

    let timeline = timeout(
        Duration::from_secs(2),
        app.list_timeline_scoped(topic, TimelineScope::Public, None, 20),
    )
    .await
    .expect("timeline should not wait for remote docs")
    .expect("timeline");
    assert!(timeline.items.is_empty());

    let thread = timeout(
        Duration::from_secs(2),
        app.list_thread(topic, "missing-root", None, 20),
    )
    .await
    .expect("thread should not wait for remote docs")
    .expect("thread");
    assert!(thread.items.is_empty());

    let joined = timeout(
        Duration::from_secs(2),
        app.list_joined_private_channels(topic),
    )
    .await
    .expect("joined channels should not wait for remote docs")
    .expect("joined channels");
    assert!(joined.is_empty());

    timeout(Duration::from_secs(2), app.warm_social_graph())
        .await
        .expect("warm social graph should not wait for remote docs")
        .expect("warm social graph");

    app.shutdown().await;
}

#[tokio::test]
async fn local_only_bootstrap_reads_return_cached_content_without_remote_docs() {
    let docs_sync = Arc::new(HangingRemoteOnMissDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let keys = generate_keys();
    let writer = app_with_hanging_remote_docs(
        Arc::new(MemoryStore::default()),
        docs_sync.clone(),
        blob_service.clone(),
        keys.clone(),
    );
    let topic = "kukuri:topic:local-only-cached";
    let followed_pubkey = generate_keys().public_key_hex();

    let root_id = writer
        .create_post(topic, "cached root", None)
        .await
        .expect("create cached root");
    let reply_id = writer
        .create_post(topic, "cached reply", Some(root_id.as_str()))
        .await
        .expect("create cached reply");
    let channel = writer
        .create_private_channel(CreatePrivateChannelInput {
            topic_id: TopicId::new(topic),
            label: "cached".into(),
            audience_kind: ChannelAudienceKind::InviteOnly,
        })
        .await
        .expect("create private channel");
    let capability = writer
        .get_private_channel_capability(topic, channel.channel_id.as_str())
        .await
        .expect("get capability")
        .expect("capability");
    writer
        .follow_author(followed_pubkey.as_str())
        .await
        .expect("follow author");

    let reader = app_with_hanging_remote_docs(
        Arc::new(MemoryStore::default()),
        docs_sync,
        blob_service,
        keys,
    );
    reader
        .restore_private_channel_capability(capability)
        .await
        .expect("restore capability");

    let timeline = timeout(
        Duration::from_secs(2),
        reader.list_timeline_scoped(topic, TimelineScope::Public, None, 20),
    )
    .await
    .expect("timeline should use cached local docs")
    .expect("timeline");
    assert!(timeline.items.iter().any(|post| post.object_id == root_id));

    let thread = timeout(
        Duration::from_secs(2),
        reader.list_thread(topic, root_id.as_str(), None, 20),
    )
    .await
    .expect("thread should use cached local docs")
    .expect("thread");
    assert!(thread.items.iter().any(|post| post.object_id == root_id));
    assert!(thread.items.iter().any(|post| post.object_id == reply_id));

    let joined = timeout(
        Duration::from_secs(2),
        reader.list_joined_private_channels(topic),
    )
    .await
    .expect("joined channels should use cached local docs")
    .expect("joined channels");
    assert_eq!(joined.len(), 1);
    assert_eq!(joined[0].channel_id, channel.channel_id);

    timeout(Duration::from_secs(2), reader.warm_social_graph())
        .await
        .expect("warm social graph should use cached local docs")
        .expect("warm social graph");
    timeout(Duration::from_secs(2), async {
        loop {
            let view = reader
                .get_author_social_view(followed_pubkey.as_str())
                .await
                .expect("author social view");
            if view.following {
                return;
            }
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("follow relationship should hydrate from cached local docs");

    writer.shutdown().await;
    reader.shutdown().await;
}

#[tokio::test]
async fn discovery_status_separates_bootstrap_seed_peers_from_manual_tickets() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
    transport
        .configure_discovery(
            DiscoveryMode::StaticPeer,
            false,
            vec![SeedPeer {
                endpoint_id: "configured-peer".into(),
                addr_hint: None,
            }],
            vec![SeedPeer {
                endpoint_id: "bootstrap-peer".into(),
                addr_hint: None,
            }],
        )
        .await
        .expect("configure discovery");
    transport
        .import_ticket("manual-ticket-peer")
        .await
        .expect("import ticket");
    let app = AppService::new(store, transport);

    let discovery = app.get_discovery_status().await.expect("discovery status");

    assert_eq!(
        discovery.configured_seed_peer_ids,
        vec!["configured-peer".to_string()]
    );
    assert_eq!(
        discovery.bootstrap_seed_peer_ids,
        vec!["bootstrap-peer".to_string()]
    );
    assert_eq!(
        discovery.manual_ticket_peer_ids,
        vec!["manual-ticket-peer".to_string()]
    );
    assert!(discovery.docs_assist_peer_ids.is_empty());
    assert!(discovery.blob_assist_peer_ids.is_empty());
}

#[tokio::test]
async fn docs_assisted_peers_do_not_mark_live_sync_connected() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot {
        connected: false,
        peer_count: 0,
        connected_peers: Vec::new(),
        configured_peers: vec!["peer-a".into(), "peer-b".into()],
        subscribed_topics: vec!["kukuri:topic:relay-assisted".into()],
        pending_events: 0,
        status_detail: "No peers configured".into(),
        last_error: None,
        topic_diagnostics: vec![TopicPeerSnapshot {
            topic: "kukuri:topic:relay-assisted".into(),
            joined: false,
            peer_count: 0,
            connected_peers: Vec::new(),
            configured_peer_ids: vec!["peer-a".into(), "peer-b".into()],
            missing_peer_ids: vec!["peer-a".into(), "peer-b".into()],
            last_received_at: None,
            status_detail: "No peers configured".into(),
            last_error: None,
        }],
    }));
    let docs_sync = Arc::new(AssistedDocsSync::new(vec!["peer-a", "peer-b"]));
    let blob_service = Arc::new(AssistedBlobService::new(vec!["peer-b", "peer-c"]));
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport.clone(),
        transport,
        docs_sync,
        blob_service,
        generate_keys(),
    );

    let status = app.get_sync_status().await.expect("sync status");

    assert!(!status.connected);
    assert_eq!(status.delivery_state, DeliveryState::DurableRecovering);
    assert_eq!(status.peer_count, 0);
    assert_eq!(
        status.status_detail,
        "docs-assisted recovery is in progress via 2 peer(s); live topic delivery is unavailable"
    );
    assert_eq!(
        status.discovery.docs_assist_peer_ids,
        vec!["peer-a".to_string(), "peer-b".to_string()]
    );
    assert_eq!(
        status.discovery.blob_assist_peer_ids,
        vec!["peer-b".to_string(), "peer-c".to_string()]
    );
    assert_eq!(status.topic_diagnostics.len(), 1);
    assert!(!status.topic_diagnostics[0].joined);
    assert_eq!(
        status.topic_diagnostics[0].delivery_state,
        DeliveryState::DurableRecovering
    );
    assert_eq!(status.topic_diagnostics[0].peer_count, 0);
    assert_eq!(
        status.topic_diagnostics[0].docs_assist_peer_ids,
        vec!["peer-a".to_string(), "peer-b".to_string()]
    );
    assert_eq!(
        status.topic_diagnostics[0].status_detail,
        "docs-assisted recovery is in progress via 2 peer(s); live topic delivery is unavailable"
    );
}

#[tokio::test]
async fn blob_only_assist_peers_do_not_mark_sync_healthy() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot {
        connected: false,
        peer_count: 0,
        connected_peers: Vec::new(),
        configured_peers: vec!["peer-a".into()],
        subscribed_topics: vec!["kukuri:topic:relay-assisted".into()],
        pending_events: 0,
        status_detail: "No peers configured".into(),
        last_error: None,
        topic_diagnostics: vec![TopicPeerSnapshot {
            topic: "kukuri:topic:relay-assisted".into(),
            joined: false,
            peer_count: 0,
            connected_peers: Vec::new(),
            configured_peer_ids: vec!["peer-a".into()],
            missing_peer_ids: vec!["peer-a".into()],
            last_received_at: None,
            status_detail: "No peers configured".into(),
            last_error: None,
        }],
    }));
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport.clone(),
        transport,
        Arc::new(AssistedDocsSync::default()),
        Arc::new(AssistedBlobService::new(vec!["peer-b"])),
        generate_keys(),
    );

    let status = app.get_sync_status().await.expect("sync status");

    assert!(!status.connected);
    assert_eq!(status.delivery_state, DeliveryState::Offline);
    assert_eq!(status.peer_count, 0);
    assert_eq!(status.status_detail, "No peers configured");
    assert!(status.discovery.docs_assist_peer_ids.is_empty());
    assert_eq!(
        status.discovery.blob_assist_peer_ids,
        vec!["peer-b".to_string()]
    );
    assert_eq!(status.topic_diagnostics.len(), 1);
    assert!(!status.topic_diagnostics[0].joined);
    assert_eq!(
        status.topic_diagnostics[0].delivery_state,
        DeliveryState::Offline
    );
    assert_eq!(status.topic_diagnostics[0].peer_count, 0);
    assert!(status.topic_diagnostics[0].docs_assist_peer_ids.is_empty());
    assert_eq!(
        status.topic_diagnostics[0].status_detail,
        "No peers configured"
    );
}

#[tokio::test]
async fn list_profile_timeline_restarts_author_subscription_with_cooldown_when_profile_is_empty() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(TrackingDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport.clone(),
        transport,
        docs_sync.clone(),
        blob_service,
        generate_keys(),
    );
    let author_pubkey = "b".repeat(64);

    let timeline = app
        .list_profile_timeline(author_pubkey.as_str(), None, 20)
        .await
        .expect("timeline");
    assert!(timeline.items.is_empty());

    let second_timeline = app
        .list_profile_timeline(author_pubkey.as_str(), None, 20)
        .await
        .expect("second timeline");
    assert!(second_timeline.items.is_empty());

    let subscribed = docs_sync.subscribe_replicas.lock().await.clone();
    assert_eq!(
        subscribed,
        vec![
            author_replica_id(author_pubkey.as_str())
                .as_str()
                .to_string(),
            author_replica_id(author_pubkey.as_str())
                .as_str()
                .to_string()
        ]
    );
}

#[tokio::test]
async fn topic_doc_events_do_not_rehydrate_whole_replica() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(CountingDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let keys = generate_keys();
    let app = AppService::new_with_services(
        store.clone(),
        store.clone(),
        transport.clone(),
        transport,
        docs_sync.clone(),
        blob_service,
        keys.clone(),
    );
    let topic = TopicId::new("kukuri:topic:incremental-doc-event");

    let _ = app
        .list_timeline(topic.as_str(), None, 20)
        .await
        .expect("initial timeline");
    sleep(Duration::from_millis(100)).await;
    docs_sync.clear_queries().await;

    let envelope = persist_test_post(
        docs_sync.as_ref(),
        None,
        &keys,
        &topic,
        PayloadRef::InlineText {
            text: "remote incremental doc".into(),
        },
        Vec::new(),
        None,
    )
    .await;

    timeout(Duration::from_secs(5), async {
        loop {
            if ProjectionStore::get_object_projection(store.as_ref(), &envelope.id)
                .await
                .expect("get projection")
                .is_some()
            {
                break;
            }
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("doc event projection timeout");

    let queries = docs_sync.queries().await;
    assert!(
        queries.iter().any(|(_, query)| {
            *query
                == DocQuery::Exact(stable_key(
                    "objects",
                    &format!("{}/state", envelope.id.as_str()),
                ))
        }),
        "expected exact object query after doc event, got {queries:?}"
    );
    assert!(
        queries.iter().all(|(_, query)| {
            !matches!(
                query,
                DocQuery::Prefix(prefix)
                    if prefix == "objects/"
                        || prefix == "reactions/"
                        || prefix == "sessions/live/"
                        || prefix == "sessions/game/"
            )
        }),
        "doc event should not trigger whole-replica rehydrate, got {queries:?}"
    );
}

#[tokio::test]
async fn topic_object_hints_do_not_rehydrate_whole_replica() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(CountingDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let keys = generate_keys();
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport.clone(),
        transport.clone(),
        docs_sync.clone(),
        blob_service,
        keys.clone(),
    );
    let topic = TopicId::new("kukuri:topic:incremental-hint-event");

    let envelope = persist_test_post(
        docs_sync.as_ref(),
        None,
        &keys,
        &topic,
        PayloadRef::InlineText {
            text: "remote incremental hint".into(),
        },
        Vec::new(),
        None,
    )
    .await;

    let _ = app
        .list_timeline(topic.as_str(), None, 20)
        .await
        .expect("initial timeline");
    sleep(Duration::from_millis(100)).await;
    docs_sync.clear_queries().await;

    transport
        .publish_hint(
            &channel_hint_topic_for(topic.as_str(), None),
            GossipHint::TopicObjectsChanged {
                topic_id: topic.clone(),
                objects: vec![HintObjectRef {
                    object_id: envelope.id.as_str().to_string(),
                    object_kind: "post".into(),
                }],
            },
        )
        .await
        .expect("publish hint");

    timeout(Duration::from_secs(5), async {
        loop {
            let queries = docs_sync.queries().await;
            if !queries.is_empty() {
                break;
            }
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("hint handling timeout");

    let queries = docs_sync.queries().await;
    assert!(
        queries.iter().any(|(_, query)| {
            *query
                == DocQuery::Exact(stable_key(
                    "objects",
                    &format!("{}/state", envelope.id.as_str()),
                ))
        }),
        "expected exact object query after hint, got {queries:?}"
    );
    assert!(
        queries.iter().all(|(_, query)| {
            !matches!(
                query,
                DocQuery::Prefix(prefix)
                    if prefix == "objects/"
                        || prefix == "reactions/"
                        || prefix == "sessions/live/"
                        || prefix == "sessions/game/"
            )
        }),
        "hint should not trigger whole-replica rehydrate, got {queries:?}"
    );
}

#[tokio::test]
async fn topic_reaction_hints_rehydrate_only_target_reactions() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(CountingDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let keys = generate_keys();
    let app = AppService::new_with_services(
        store.clone(),
        store.clone(),
        transport.clone(),
        transport.clone(),
        docs_sync.clone(),
        blob_service,
        keys.clone(),
    );
    let topic = TopicId::new("kukuri:topic:incremental-reaction-hint");
    let replica = topic_replica_id(topic.as_str());

    let envelope = persist_test_post(
        docs_sync.as_ref(),
        None,
        &keys,
        &topic,
        PayloadRef::InlineText {
            text: "remote reaction target".into(),
        },
        Vec::new(),
        None,
    )
    .await;
    let reaction_key = ReactionKeyV1::Emoji {
        emoji: "👍".into()
    };
    let reaction_id = deterministic_reaction_id(
        &replica,
        &envelope.id,
        &keys.public_key(),
        reaction_key
            .normalized_key()
            .expect("normalized reaction key")
            .as_str(),
    );
    let reaction_envelope = build_reaction_envelope(
        &keys,
        &topic,
        None,
        &envelope.id,
        reaction_key,
        &reaction_id,
        ObjectStatus::Active,
    )
    .expect("build reaction envelope");
    let reaction = parse_reaction(&reaction_envelope)
        .expect("parse reaction envelope")
        .expect("reaction doc");
    persist_reaction_doc(docs_sync.as_ref(), &replica, &reaction, &reaction_envelope)
        .await
        .expect("persist reaction doc");

    let _ = app
        .list_timeline(topic.as_str(), None, 20)
        .await
        .expect("initial timeline");
    sleep(Duration::from_millis(100)).await;
    docs_sync.clear_queries().await;

    transport
        .publish_hint(
            &channel_hint_topic_for(topic.as_str(), None),
            GossipHint::TopicObjectsChanged {
                topic_id: topic.clone(),
                objects: vec![HintObjectRef {
                    object_id: envelope.id.as_str().to_string(),
                    object_kind: "reaction".into(),
                }],
            },
        )
        .await
        .expect("publish reaction hint");

    timeout(Duration::from_secs(5), async {
        loop {
            if !docs_sync.queries().await.is_empty() {
                break;
            }
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("reaction hint handling timeout");

    let queries = docs_sync.queries().await;
    assert!(
        queries.iter().any(|(_, query)| {
            *query
                == DocQuery::Prefix(stable_key(
                    "reactions",
                    &format!("{}/", envelope.id.as_str()),
                ))
        }),
        "expected targeted reaction prefix query after hint, got {queries:?}"
    );
    assert!(
        queries.iter().all(|(_, query)| {
            !matches!(
                query,
                DocQuery::Prefix(prefix)
                    if prefix == "objects/"
                        || prefix == "reactions/"
                        || prefix == "sessions/live/"
                        || prefix == "sessions/game/"
            )
        }),
        "reaction hint should not trigger whole-replica rehydrate, got {queries:?}"
    );
}

#[tokio::test]
async fn public_topic_recovery_keeps_docs_probe_when_live_peer_has_not_delivered_content() {
    let store = Arc::new(MemoryStore::default());
    let topic = TopicId::new("kukuri:topic:live-peer-docs-probe");
    let transport = Arc::new(StaticTransport::new(PeerSnapshot {
        connected: true,
        peer_count: 1,
        connected_peers: vec!["peer-a".into()],
        configured_peers: vec!["peer-a".into()],
        subscribed_topics: vec![topic.as_str().to_string()],
        pending_events: 0,
        status_detail: "live peer connected".into(),
        last_error: None,
        topic_diagnostics: vec![TopicPeerSnapshot {
            topic: topic.as_str().to_string(),
            joined: true,
            peer_count: 1,
            connected_peers: vec!["peer-a".into()],
            configured_peer_ids: vec!["peer-a".into()],
            missing_peer_ids: Vec::new(),
            last_received_at: None,
            status_detail: "live peer connected".into(),
            last_error: None,
        }],
    }));
    let docs_sync = Arc::new(CountingDocsSync::with_assist_peer_ids(vec!["peer-a"]));
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport.clone(),
        transport.clone(),
        docs_sync.clone(),
        Arc::new(MemoryBlobService::default()),
        generate_keys(),
    );

    let _ = app
        .list_timeline(topic.as_str(), None, 20)
        .await
        .expect("initial timeline");
    sleep(Duration::from_millis(100)).await;
    docs_sync.clear_queries().await;

    transport
        .publish_hint(
            &channel_hint_topic_for(topic.as_str(), None),
            GossipHint::TopicObjectsChanged {
                topic_id: topic.clone(),
                objects: vec![HintObjectRef {
                    object_id: "missing-post".into(),
                    object_kind: "post".into(),
                }],
            },
        )
        .await
        .expect("publish hint miss");

    timeout(Duration::from_secs(5), async {
        loop {
            let queries = docs_sync.queries().await;
            if queries
                .iter()
                .any(|(_, query)| *query == DocQuery::Prefix("objects/".into()))
            {
                break;
            }
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("initial recovery probe timeout");
    docs_sync.clear_queries().await;

    timeout(
        Duration::from_millis(PUBLIC_TOPIC_RECOVERY_GRACE_MS as u64 + 2_000),
        async {
            loop {
                let queries = docs_sync.queries().await;
                if queries
                    .iter()
                    .any(|(_, query)| *query == DocQuery::Prefix("objects/".into()))
                {
                    break;
                }
                sleep(Duration::from_millis(50)).await;
            }
        },
    )
    .await
    .expect("periodic docs-assisted recovery probe timeout");
}

#[tokio::test]
async fn topic_session_hints_retry_until_manifest_blob_is_available() {
    let docs_sync = Arc::new(kukuri_docs_sync::MemoryDocsSync::default());
    let blob_service = Arc::new(DelayedBlobService::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let owner_store = Arc::new(MemoryStore::default());
    let remote_store = Arc::new(MemoryStore::default());
    let owner_app = AppService::new_with_services(
        owner_store.clone(),
        owner_store,
        transport.clone(),
        Arc::new(NoopHintTransport),
        docs_sync.clone(),
        blob_service.clone(),
        generate_keys(),
    );
    let topic = TopicId::new("kukuri:topic:incremental-live-session-retry");
    let replica = topic_replica_id(topic.as_str());

    let session_id = owner_app
        .create_live_session(
            topic.as_str(),
            CreateLiveSessionInput {
                title: "retry live".into(),
                description: "delayed manifest".into(),
            },
        )
        .await
        .expect("create live session");
    let state = fetch_live_session_state_from_replica(docs_sync.as_ref(), &replica, &session_id)
        .await
        .expect("fetch live state")
        .expect("live state");
    blob_service
        .delay_hash(&state.current_manifest.hash, 2)
        .await;

    let hydrated = hydrate_subscription_hint_with_services(
        docs_sync.as_ref(),
        blob_service.as_ref(),
        remote_store.as_ref(),
        topic.as_str(),
        &replica,
        &GossipHint::SessionChanged {
            topic_id: topic.clone(),
            session_id: session_id.clone(),
            object_kind: "live-session".into(),
        },
    )
    .await
    .expect("hydrate live hint");

    assert_eq!(hydrated, 1);
    assert!(
        ProjectionStore::list_topic_live_sessions(remote_store.as_ref(), topic.as_str())
            .await
            .expect("list remote live sessions")
            .iter()
            .any(|session| session.session_id == session_id),
        "expected live session projection after retry"
    );
}

#[tokio::test]
async fn list_timeline_restarts_topic_replica_sync_with_cooldown_when_projection_is_empty() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(TrackingDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport.clone(),
        transport,
        docs_sync.clone(),
        blob_service,
        generate_keys(),
    );

    let timeline = app
        .list_timeline("kukuri:topic:replica-restart", None, 20)
        .await
        .expect("timeline");
    assert!(timeline.items.is_empty());

    let second_timeline = app
        .list_timeline("kukuri:topic:replica-restart", None, 20)
        .await
        .expect("second timeline");
    assert!(second_timeline.items.is_empty());
    let third_timeline = app
        .list_timeline("kukuri:topic:replica-restart", None, 20)
        .await
        .expect("third timeline");
    assert!(third_timeline.items.is_empty());

    let restarted = docs_sync.restarted_replicas.lock().await.clone();
    assert_eq!(
        restarted,
        vec![
            topic_replica_id("kukuri:topic:replica-restart")
                .as_str()
                .to_string()
        ]
    );
}

#[tokio::test]
async fn list_timeline_restarts_topic_subscription_with_cooldown_when_projection_is_empty() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let hint_transport = Arc::new(TrackingHintTransport::default());
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport,
        hint_transport.clone(),
        Arc::new(TrackingDocsSync::default()),
        Arc::new(MemoryBlobService::default()),
        generate_keys(),
    );
    let topic = "kukuri:topic:subscription-restart";

    let timeline = app.list_timeline(topic, None, 20).await.expect("timeline");
    assert!(timeline.items.is_empty());

    let second_timeline = app
        .list_timeline(topic, None, 20)
        .await
        .expect("second timeline");
    assert!(second_timeline.items.is_empty());
    let third_timeline = app
        .list_timeline(topic, None, 20)
        .await
        .expect("third timeline");
    assert!(third_timeline.items.is_empty());

    assert_eq!(*hint_transport.subscribe_count.lock().await, 2);
    assert_eq!(
        hint_transport.unsubscribed_topics.lock().await.clone(),
        vec![topic.to_string()]
    );
}

#[tokio::test]
async fn ensure_topic_subscription_recreates_finished_handle() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let hint_transport = Arc::new(TrackingHintTransport::default());
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport,
        hint_transport.clone(),
        Arc::new(TrackingDocsSync::default()),
        Arc::new(MemoryBlobService::default()),
        generate_keys(),
    );
    let topic = "kukuri:topic:stale-subscription";

    let timeline = app.list_timeline(topic, None, 20).await.expect("timeline");
    assert!(timeline.items.is_empty());
    assert_eq!(*hint_transport.subscribe_count.lock().await, 1);

    {
        let subscriptions = app.subscriptions.lock().await;
        subscriptions
            .get(topic)
            .expect("topic subscription handle")
            .abort();
    }
    timeout(Duration::from_secs(5), async {
        loop {
            if !app.has_topic_subscription(topic).await {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("subscription should finish after abort");

    let second_timeline = app
        .list_timeline(topic, None, 20)
        .await
        .expect("second timeline");
    assert!(second_timeline.items.is_empty());
    assert_eq!(*hint_transport.subscribe_count.lock().await, 2);
}

#[tokio::test]
async fn set_discovery_seeds_restarts_existing_topic_hint_subscription() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let hint_transport = Arc::new(TrackingHintTransport::default());
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport.clone(),
        hint_transport.clone(),
        Arc::new(MemoryDocsSync::default()),
        Arc::new(MemoryBlobService::default()),
        generate_keys(),
    );
    let topic = "kukuri:topic:hint-restart";

    let _ = app
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe timeline");

    app.set_discovery_seeds(
        DiscoveryMode::StaticPeer,
        false,
        vec![SeedPeer {
            endpoint_id: "peer-a".into(),
            addr_hint: None,
        }],
        Vec::new(),
    )
    .await
    .expect("set discovery seeds");

    assert_eq!(*hint_transport.subscribe_count.lock().await, 2);
    assert_eq!(
        hint_transport.unsubscribed_topics.lock().await.clone(),
        vec![topic.to_string()]
    );
}

#[tokio::test]
async fn import_peer_ticket_restarts_existing_topic_hint_subscription() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let hint_transport = Arc::new(TrackingHintTransport::default());
    let docs_sync = Arc::new(TrackingDocsSync::default());
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport,
        hint_transport.clone(),
        docs_sync.clone(),
        Arc::new(MemoryBlobService::default()),
        generate_keys(),
    );
    let topic = "kukuri:topic:hint-import";

    let _ = app
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe timeline");

    app.import_peer_ticket("peer-ticket")
        .await
        .expect("import peer ticket");

    assert_eq!(*hint_transport.subscribe_count.lock().await, 2);
    assert_eq!(
        hint_transport.unsubscribed_topics.lock().await.clone(),
        vec![topic.to_string()]
    );
    assert_eq!(docs_sync.subscribe_replicas.lock().await.len(), 2);
}

#[tokio::test]
async fn local_public_post_coalesces_replica_sync_restarts() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(TrackingDocsSync::default());
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport.clone(),
        transport,
        docs_sync.clone(),
        Arc::new(MemoryBlobService::default()),
        generate_keys(),
    );
    let topic = "kukuri:topic:post-restart-cooldown";

    let _ = app
        .create_post(topic, "first post", None)
        .await
        .expect("first post");
    let _ = app
        .create_post(topic, "second post", None)
        .await
        .expect("second post");

    assert_eq!(
        docs_sync.restarted_replicas.lock().await.clone(),
        vec![topic_replica_id(topic).as_str().to_string()]
    );
}

#[tokio::test]
async fn hint_miss_coalesces_replica_sync_restarts() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let hint_transport = Arc::new(TrackingHintTransport::default());
    let docs_sync = Arc::new(TrackingDocsSync::default());
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport,
        hint_transport.clone(),
        docs_sync.clone(),
        Arc::new(MemoryBlobService::default()),
        generate_keys(),
    );
    let topic = "kukuri:topic:hint-miss-cooldown";

    let _ = app
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe timeline");

    let hint_topic = TopicId::new(topic);
    for suffix in ["one", "two"] {
        hint_transport
            .publish_hint(
                &hint_topic,
                GossipHint::TopicObjectsChanged {
                    topic_id: hint_topic.clone(),
                    objects: vec![HintObjectRef {
                        object_id: format!("missing-{suffix}"),
                        object_kind: "post".into(),
                    }],
                },
            )
            .await
            .expect("publish hint miss");
    }

    timeout(Duration::from_secs(5), async {
        loop {
            if !docs_sync.restarted_replicas.lock().await.is_empty() {
                return;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("restart should be requested");

    assert_eq!(
        docs_sync.restarted_replicas.lock().await.clone(),
        vec![topic_replica_id(topic).as_str().to_string()]
    );
}

#[tokio::test]
async fn shutdown_unsubscribes_active_hint_topics() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let hint_transport = Arc::new(TrackingHintTransport::default());
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport,
        hint_transport.clone(),
        Arc::new(MemoryDocsSync::default()),
        Arc::new(MemoryBlobService::default()),
        generate_keys(),
    );
    let topic = "kukuri:topic:shutdown";

    let _ = app
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe timeline");

    app.shutdown().await;

    assert_eq!(
        hint_transport.unsubscribed_topics.lock().await.clone(),
        vec![topic.to_string()]
    );
}

#[tokio::test]
async fn sync_status_normalizes_hint_topic_names() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot {
        connected: true,
        peer_count: 1,
        connected_peers: vec!["peer-a".into()],
        configured_peers: vec!["peer-a".into()],
        subscribed_topics: vec!["hint/kukuri:topic:demo".into()],
        pending_events: 0,
        status_detail: "Connected".into(),
        last_error: None,
        topic_diagnostics: vec![TopicPeerSnapshot {
            topic: "hint/kukuri:topic:demo".into(),
            joined: true,
            peer_count: 1,
            connected_peers: vec!["peer-a".into()],
            configured_peer_ids: vec!["peer-a".into()],
            missing_peer_ids: Vec::new(),
            last_received_at: Some(1),
            status_detail: "Connected".into(),
            last_error: None,
        }],
    }));
    let app = AppService::new(store, transport);

    let status = app.get_sync_status().await.expect("sync status");

    assert_eq!(status.subscribed_topics, vec!["kukuri:topic:demo"]);
    assert_eq!(status.topic_diagnostics.len(), 1);
    assert_eq!(status.topic_diagnostics[0].topic, "kukuri:topic:demo");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_ticket_updates_sync_status_error_reason() {
    let _guard = iroh_integration_test_lock().lock_owned().await;
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(
        IrohGossipTransport::bind_local()
            .await
            .expect("transport should bind"),
    );
    let app = AppService::new(store, transport);

    let error = app
        .import_peer_ticket("not-a-ticket")
        .await
        .expect_err("invalid ticket should fail");
    let status = app.get_sync_status().await.expect("sync status");

    assert!(error.to_string().contains("failed to import peer ticket"));
    assert!(
        status
            .last_error
            .as_deref()
            .is_some_and(|message| message.contains("failed to import peer ticket"))
    );
}

#[tokio::test]
async fn unsubscribe_topic_removes_subscription_from_sync_status() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
    let app = AppService::new(store, transport);

    let _ = app
        .list_timeline("kukuri:topic:one", None, 10)
        .await
        .expect("timeline one");
    let _ = app
        .list_timeline("kukuri:topic:two", None, 10)
        .await
        .expect("timeline two");
    app.unsubscribe_topic("kukuri:topic:two")
        .await
        .expect("unsubscribe topic");
    let status = app.get_sync_status().await.expect("sync status");

    assert!(
        status
            .subscribed_topics
            .iter()
            .any(|topic| topic == "kukuri:topic:one")
    );
    assert!(
        !status
            .subscribed_topics
            .iter()
            .any(|topic| topic == "kukuri:topic:two")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_transport_syncs_post_between_apps() {
    let _guard = iroh_integration_test_lock().lock_owned().await;
    let dir = tempdir().expect("tempdir");
    let stack_a = TestIrohStack::new(&dir.path().join("post-a")).await;
    let stack_b = TestIrohStack::new(&dir.path().join("post-b")).await;
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let app_a = app_with_iroh_services(store_a, &stack_a);
    let app_b = app_with_iroh_services(store_b.clone(), &stack_b);

    let ticket_a = app_a
        .peer_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = app_b
        .peer_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");
    app_a
        .import_peer_ticket(&ticket_b)
        .await
        .expect("import b into a");
    app_b
        .import_peer_ticket(&ticket_a)
        .await
        .expect("import a into b");

    let topic = "kukuri:topic:app-api-iroh";
    let _ = app_b
        .list_timeline(topic, None, 20)
        .await
        .expect("app b should subscribe to topic");

    let object_id = app_a
        .create_post(topic, "hello over iroh transport", None)
        .await
        .expect("app a should create post");

    let received = timeout(Duration::from_secs(30), async {
        loop {
            let timeline = app_b
                .list_timeline(topic, None, 20)
                .await
                .expect("timeline should load");
            if let Some(post) = timeline
                .items
                .iter()
                .find(|post| post.object_id == object_id)
            {
                return post.clone();
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("timeline sync timeout");

    assert_eq!(received.content, "hello over iroh transport");
    let status_b = app_b.get_sync_status().await.expect("sync status b");
    assert!(status_b.last_sync_ts.is_some());
    assert!(
        status_b
            .subscribed_topics
            .iter()
            .any(|value| value == topic)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn import_peer_ticket_restarts_existing_topic_subscription_and_resumes_delivery() {
    let _guard = iroh_integration_test_lock().lock_owned().await;
    let dir = tempdir().expect("tempdir");
    let stack_a = TestIrohStack::new(&dir.path().join("rebind-a")).await;
    let stack_b = TestIrohStack::new(&dir.path().join("rebind-b")).await;
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let app_a = app_with_iroh_services(store_a, &stack_a);
    let app_b = app_with_iroh_services(store_b, &stack_b);
    let topic = "kukuri:topic:rebind-after-import";

    let _ = app_a
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe a before import");
    let _ = app_b
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe b before import");

    let ticket_a = app_a
        .peer_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = app_b
        .peer_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");
    app_a
        .import_peer_ticket(&ticket_b)
        .await
        .expect("import b into a");
    app_b
        .import_peer_ticket(&ticket_a)
        .await
        .expect("import a into b");

    wait_for_topic_delivery(&app_a, topic, 1).await;
    wait_for_topic_delivery(&app_b, topic, 1).await;

    let object_id = app_a
        .create_post(topic, "hello after import", None)
        .await
        .expect("create post");
    let received = timeout(Duration::from_secs(30), async {
        loop {
            let timeline = app_b
                .list_timeline(topic, None, 20)
                .await
                .expect("timeline should load");
            if let Some(post) = timeline
                .items
                .iter()
                .find(|post| post.object_id == object_id)
            {
                return post.clone();
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("timeline sync timeout");

    assert_eq!(received.content, "hello after import");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seeded_dht_syncs_post_between_apps_without_ticket_import() {
    let _guard = iroh_integration_test_lock().lock_owned().await;
    let dir = tempdir().expect("tempdir");
    let testnet = Testnet::new(5).expect("testnet");
    let stack_a = TestIrohStack::new_with_dht(&dir.path().join("seeded-dht-a"), &testnet).await;
    let stack_b = TestIrohStack::new_with_dht(&dir.path().join("seeded-dht-b"), &testnet).await;
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let app_a = app_with_iroh_services(store_a, &stack_a);
    let app_b = app_with_iroh_services(store_b, &stack_b);
    let endpoint_a = app_a
        .get_sync_status()
        .await
        .expect("status a")
        .discovery
        .local_endpoint_id;
    let endpoint_b = app_b
        .get_sync_status()
        .await
        .expect("status b")
        .discovery
        .local_endpoint_id;

    configure_seeded_dht(&app_a, endpoint_b.clone()).await;
    configure_seeded_dht(&app_b, endpoint_a.clone()).await;
    let topic = "kukuri:topic:seeded-dht-app";
    let _ = app_a
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe a timeline");
    let _ = app_b
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe b timeline");
    timeout(Duration::from_secs(90), async {
        loop {
            let status_a = app_a.get_sync_status().await.expect("status a");
            let status_b = app_b.get_sync_status().await.expect("status b");
            let ready_a = status_a
                .topic_diagnostics
                .iter()
                .any(|topic_status| topic_status.topic == topic && topic_status.peer_count > 0);
            let ready_b = status_b
                .topic_diagnostics
                .iter()
                .any(|topic_status| topic_status.topic == topic && topic_status.peer_count > 0);
            if ready_a && ready_b {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("seeded dht ready timeout");

    let object_id = app_a
        .create_post(topic, "seeded dht app sync", None)
        .await
        .expect("create post");

    let received = timeout(Duration::from_secs(20), async {
        loop {
            let timeline = app_b
                .list_timeline(topic, None, 20)
                .await
                .expect("timeline");
            if let Some(post) = timeline
                .items
                .iter()
                .find(|post| post.object_id == object_id)
            {
                return post.clone();
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("seeded dht sync timeout");

    assert_eq!(received.content, "seeded dht app sync");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn relay_seeded_syncs_post_between_apps_without_ticket_import() {
    let _guard = iroh_integration_test_lock().lock_owned().await;
    let (_relay_map, relay_url, _relay_guard) = iroh::test_utils::run_relay_server()
        .await
        .expect("relay server");
    let relay_config = TransportRelayConfig {
        iroh_relay_urls: vec![relay_url.to_string()],
    };
    let dir = tempdir().expect("tempdir");
    let stack_a = TestIrohStack::new_with_options(
        &dir.path().join("relay-seeded-a"),
        DhtDiscoveryOptions::disabled(),
        relay_config.clone(),
    )
    .await;
    let stack_b = TestIrohStack::new_with_options(
        &dir.path().join("relay-seeded-b"),
        DhtDiscoveryOptions::disabled(),
        relay_config,
    )
    .await;
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let app_a = app_with_iroh_services(store_a, &stack_a);
    let app_b = app_with_iroh_services(store_b, &stack_b);
    let ticket_a = app_a
        .peer_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = app_b
        .peer_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");
    let seed_a = {
        let (endpoint_id, addr_hint) = ticket_b.split_once('@').expect("ticket b host");
        SeedPeer {
            endpoint_id: endpoint_id.to_string(),
            addr_hint: Some(addr_hint.to_string()),
        }
    };
    let seed_b = {
        let (endpoint_id, addr_hint) = ticket_a.split_once('@').expect("ticket a host");
        SeedPeer {
            endpoint_id: endpoint_id.to_string(),
            addr_hint: Some(addr_hint.to_string()),
        }
    };
    app_a
        .set_discovery_seeds(DiscoveryMode::StaticPeer, false, Vec::new(), vec![seed_a])
        .await
        .expect("set relay seeds a");
    app_b
        .set_discovery_seeds(DiscoveryMode::StaticPeer, false, Vec::new(), vec![seed_b])
        .await
        .expect("set relay seeds b");

    let topic = "kukuri:topic:relay-seeded";
    let _ = app_a
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe a");
    let _ = app_b
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe b");

    let object_id = app_a
        .create_post(topic, "relay seeded app sync", None)
        .await
        .expect("create post");
    let received = timeout(Duration::from_secs(60), async {
        loop {
            let timeline = app_b
                .list_timeline(topic, None, 20)
                .await
                .expect("timeline");
            if let Some(post) = timeline
                .items
                .iter()
                .find(|post| post.object_id == object_id)
            {
                return post.clone();
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await;
    let received = match received {
        Ok(post) => post,
        Err(error) => {
            let diagnostics =
                iroh_sync_diagnostics(&app_a, &app_b, &stack_a, &stack_b, topic).await;
            panic!("relay seeded sync timeout: {error:?}; {diagnostics}");
        }
    };

    assert_eq!(received.content, "relay seeded app sync");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn external_relay_endpoint_only_seeds_sync_post_between_apps() {
    let Some(relay_url) = std::env::var("KUKURI_TEST_EXTERNAL_IROH_RELAY_URL")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
    else {
        return;
    };

    let _guard = iroh_integration_test_lock().lock_owned().await;
    let relay_config = TransportRelayConfig {
        iroh_relay_urls: vec![relay_url],
    };
    let dir = tempdir().expect("tempdir");
    let stack_a = TestIrohStack::new_with_network_options(
        &dir.path().join("external-relay-a"),
        kukuri_transport::TransportNetworkConfig::default(),
        DhtDiscoveryOptions::disabled(),
        relay_config.clone(),
    )
    .await;
    let stack_b = TestIrohStack::new_with_network_options(
        &dir.path().join("external-relay-b"),
        kukuri_transport::TransportNetworkConfig::default(),
        DhtDiscoveryOptions::disabled(),
        relay_config,
    )
    .await;
    let app_a = app_with_iroh_services(Arc::new(MemoryStore::default()), &stack_a);
    let app_b = app_with_iroh_services(Arc::new(MemoryStore::default()), &stack_b);
    let endpoint_a = app_a
        .get_sync_status()
        .await
        .expect("status a")
        .discovery
        .local_endpoint_id;
    let endpoint_b = app_b
        .get_sync_status()
        .await
        .expect("status b")
        .discovery
        .local_endpoint_id;

    app_a
        .set_discovery_seeds(
            DiscoveryMode::StaticPeer,
            false,
            Vec::new(),
            vec![SeedPeer {
                endpoint_id: endpoint_b,
                addr_hint: None,
            }],
        )
        .await
        .expect("set seed a");
    app_b
        .set_discovery_seeds(
            DiscoveryMode::StaticPeer,
            false,
            Vec::new(),
            vec![SeedPeer {
                endpoint_id: endpoint_a,
                addr_hint: None,
            }],
        )
        .await
        .expect("set seed b");

    let topic = "kukuri:topic:external-relay-endpoint-only";
    let _ = app_a
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe a");
    let _ = app_b
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe b");
    wait_for_topic_delivery(&app_a, topic, 1).await;
    wait_for_topic_delivery(&app_b, topic, 1).await;

    let object_id = app_a
        .create_post(topic, "external relay endpoint-only app sync", None)
        .await
        .expect("create post");
    let received = timeout(Duration::from_secs(120), async {
        loop {
            let timeline = app_b
                .list_timeline(topic, None, 20)
                .await
                .expect("timeline");
            if let Some(post) = timeline
                .items
                .iter()
                .find(|post| post.object_id == object_id)
            {
                return post.clone();
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await;
    let received = match received {
        Ok(post) => post,
        Err(error) => {
            let diagnostics =
                iroh_sync_diagnostics(&app_a, &app_b, &stack_a, &stack_b, topic).await;
            panic!("external relay endpoint-only sync timeout: {error:?}; {diagnostics}");
        }
    };

    assert_eq!(received.content, "external relay endpoint-only app sync");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seeded_dht_updates_existing_topic_subscription_after_seed_update() {
    let _guard = iroh_integration_test_lock().lock_owned().await;
    let dir = tempdir().expect("tempdir");
    let testnet = Testnet::new(5).expect("testnet");
    let stack_a = TestIrohStack::new_with_dht(&dir.path().join("seeded-rebind-a"), &testnet).await;
    let stack_b = TestIrohStack::new_with_dht(&dir.path().join("seeded-rebind-b"), &testnet).await;
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let app_a = app_with_iroh_services(store_a, &stack_a);
    let app_b = app_with_iroh_services(store_b, &stack_b);
    let topic = "kukuri:topic:seeded-rebind";

    let _ = app_a
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe a before seed update");
    let _ = app_b
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe b before seed update");

    let endpoint_a = app_a
        .get_sync_status()
        .await
        .expect("status a")
        .discovery
        .local_endpoint_id;
    let endpoint_b = app_b
        .get_sync_status()
        .await
        .expect("status b")
        .discovery
        .local_endpoint_id;
    configure_seeded_dht(&app_a, endpoint_b.clone()).await;
    configure_seeded_dht(&app_b, endpoint_a.clone()).await;

    timeout(Duration::from_secs(20), async {
        let mut stable_ready_polls = 0usize;
        loop {
            let status_a = app_a.get_sync_status().await.expect("status a");
            let status_b = app_b.get_sync_status().await.expect("status b");
            let ready_a = status_a.topic_diagnostics.iter().any(|topic_status| {
                topic_status.topic == topic
                    && topic_status.joined
                    && !topic_status.connected_peers.is_empty()
            });
            let ready_b = status_b.topic_diagnostics.iter().any(|topic_status| {
                topic_status.topic == topic
                    && topic_status.joined
                    && !topic_status.connected_peers.is_empty()
            });
            if ready_a && ready_b {
                stable_ready_polls += 1;
                if stable_ready_polls >= 3 {
                    return;
                }
            } else {
                stable_ready_polls = 0;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("seeded dht topic update timeout");

    let object_id = app_a
        .create_post(topic, "seeded dht rebind", None)
        .await
        .expect("create post");

    timeout(Duration::from_secs(90), async {
        loop {
            let timeline = app_b
                .list_timeline(topic, None, 20)
                .await
                .expect("timeline b");
            if timeline
                .items
                .iter()
                .any(|post| post.object_id == object_id)
            {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("seeded dht propagation timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seeded_dht_backfills_docs_and_blobs_with_id_only_seed() {
    let _guard = iroh_integration_test_lock().lock_owned().await;
    let dir = tempdir().expect("tempdir");
    let testnet = Testnet::new(5).expect("testnet");
    let stack_a = TestIrohStack::new_with_dht(&dir.path().join("seeded-image-a"), &testnet).await;
    let stack_b = TestIrohStack::new_with_dht(&dir.path().join("seeded-image-b"), &testnet).await;
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let app_a = app_with_iroh_services(store_a, &stack_a);
    let app_b = app_with_iroh_services(store_b, &stack_b);
    let endpoint_a = app_a
        .get_sync_status()
        .await
        .expect("status a")
        .discovery
        .local_endpoint_id;
    let endpoint_b = app_b
        .get_sync_status()
        .await
        .expect("status b")
        .discovery
        .local_endpoint_id;
    configure_seeded_dht(&app_a, endpoint_b.clone()).await;
    configure_seeded_dht(&app_b, endpoint_a.clone()).await;
    let topic = "kukuri:topic:seeded-image";
    let _ = app_a
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe a timeline");
    let _ = app_b
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe b timeline");
    timeout(Duration::from_secs(20), async {
        loop {
            let status_a = app_a.get_sync_status().await.expect("status a");
            let status_b = app_b.get_sync_status().await.expect("status b");
            let ready_a = status_a
                .topic_diagnostics
                .iter()
                .any(|topic_status| topic_status.topic == topic && topic_status.peer_count > 0);
            let ready_b = status_b
                .topic_diagnostics
                .iter()
                .any(|topic_status| topic_status.topic == topic && topic_status.peer_count > 0);
            if ready_a && ready_b {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("seeded dht image ready timeout");

    let object_id = app_a
        .create_post_with_attachments(
            topic,
            "seeded image",
            None,
            vec![pending_image_attachment("image/png", b"seeded-image-bytes")],
        )
        .await
        .expect("create image post");

    let received = timeout(Duration::from_secs(20), async {
        loop {
            let timeline = app_b
                .list_timeline(topic, None, 20)
                .await
                .expect("timeline b");
            if let Some(post) = timeline
                .items
                .iter()
                .find(|post| post.object_id == object_id)
            {
                return post.clone();
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("seeded dht image backfill timeout");

    assert_eq!(received.attachments.len(), 1);
    assert_eq!(received.attachments[0].status, BlobViewStatus::Available);
    assert!(
        app_b
            .blob_preview_data_url(received.attachments[0].hash.as_str(), "image/png")
            .await
            .expect("preview")
            .is_some()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_transport_syncs_repost_and_notification() {
    let _guard = iroh_integration_test_lock().lock_owned().await;
    let dir = tempdir().expect("tempdir");
    let stack_a = TestIrohStack::new(&dir.path().join("repost-a")).await;
    let stack_b = TestIrohStack::new(&dir.path().join("repost-b")).await;
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let app_a = app_with_iroh_services(store_a, &stack_a);
    let app_b = app_with_iroh_services(store_b, &stack_b);
    let topic = "kukuri:topic:repost-notification";

    let ticket_a = app_a
        .peer_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = app_b
        .peer_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");
    app_a
        .import_peer_ticket(&ticket_b)
        .await
        .expect("import b into a");
    app_b
        .import_peer_ticket(&ticket_a)
        .await
        .expect("import a into b");

    let _ = app_a
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe a timeline");
    let _ = app_b
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe b timeline");
    wait_for_topic_delivery(&app_a, topic, 1).await;
    wait_for_topic_delivery(&app_b, topic, 1).await;

    let source_id = app_a
        .create_post(topic, "relay source post", None)
        .await
        .expect("create source post");
    if let Err(error) = timeout(p2p_replication_timeout(), async {
        loop {
            let timeline = app_b
                .list_timeline(topic, None, 20)
                .await
                .expect("timeline b");
            if timeline
                .items
                .iter()
                .any(|post| post.object_id == source_id)
            {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    {
        panic!(
            "source propagation timeout: {error:?}; {}",
            iroh_sync_diagnostics(&app_a, &app_b, &stack_a, &stack_b, topic).await
        );
    }

    let repost_id = app_b
        .create_repost(topic, topic, source_id.as_str(), None)
        .await
        .expect("create repost");
    if let Err(error) = timeout(p2p_replication_timeout(), async {
        loop {
            let timeline = app_a
                .list_timeline(topic, None, 20)
                .await
                .expect("timeline a");
            let notifications = app_a.list_notifications().await.expect("notifications a");
            let has_repost = timeline
                .items
                .iter()
                .any(|post| post.object_id == repost_id);
            let has_notification = notifications.iter().any(|item| {
                item.kind == NotificationKind::Repost
                    && item.object_id.as_deref() == Some(repost_id.as_str())
            });
            if has_repost && has_notification {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    {
        panic!(
            "repost propagation timeout: {error:?}; {}",
            iroh_sync_diagnostics(&app_a, &app_b, &stack_a, &stack_b, topic).await
        );
    }

    assert_eq!(
        app_a
            .get_notification_status()
            .await
            .expect("notification status")
            .unread_count,
        1
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_transport_syncs_reply_into_thread() {
    let _guard = iroh_integration_test_lock().lock_owned().await;
    let dir = tempdir().expect("tempdir");
    let stack_a = TestIrohStack::new(&dir.path().join("reply-a")).await;
    let stack_b = TestIrohStack::new(&dir.path().join("reply-b")).await;
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let app_a = app_with_iroh_services(store_a.clone(), &stack_a);
    let app_b = app_with_iroh_services(store_b, &stack_b);
    let topic = "kukuri:topic:reply-thread";

    let ticket_a = app_a
        .peer_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = app_b
        .peer_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");
    app_a
        .import_peer_ticket(&ticket_b)
        .await
        .expect("import b into a");
    app_b
        .import_peer_ticket(&ticket_a)
        .await
        .expect("import a into b");
    let _ = app_a
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe a timeline");
    let _ = app_b
        .list_timeline(topic, None, 20)
        .await
        .expect("subscribe b timeline");
    wait_for_topic_delivery(&app_a, topic, 1).await;
    wait_for_topic_delivery(&app_b, topic, 1).await;

    let root_id = app_a
        .create_post(topic, "root over iroh", None)
        .await
        .expect("create root");

    timeout(Duration::from_secs(10), async {
        loop {
            let timeline = app_b
                .list_timeline(topic, None, 20)
                .await
                .expect("timeline b");
            if timeline.items.iter().any(|post| post.object_id == root_id) {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("root propagation timeout");

    let reply_id = app_b
        .create_post(topic, "reply over iroh", Some(root_id.as_str()))
        .await
        .expect("create reply");
    let thread = timeout(p2p_replication_timeout(), async {
        loop {
            let thread = app_b
                .list_thread(topic, root_id.as_str(), None, 20)
                .await
                .expect("thread b");
            if thread.items.iter().any(|post| post.object_id == reply_id) {
                return thread;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("local reply propagation timeout");

    let thread_ids = thread
        .items
        .iter()
        .map(|post| post.object_id.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        thread_ids.len(),
        2,
        "thread items: {:?}",
        thread
            .items
            .iter()
            .map(|post| format!(
                "{}|reply={:?}|root={:?}",
                post.object_id, post.reply_to, post.root_id
            ))
            .collect::<Vec<_>>()
    );
    assert!(thread_ids.contains(root_id.as_str()));
    assert!(thread_ids.contains(reply_id.as_str()));
    let reply = thread
        .items
        .iter()
        .find(|post| post.object_id == reply_id)
        .expect("reply in thread");
    assert_eq!(reply.reply_to.as_deref(), Some(root_id.as_str()));
    assert_eq!(reply.root_id.as_deref(), Some(root_id.as_str()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_transport_syncs_multiple_topics_bidirectionally() {
    let _guard = iroh_integration_test_lock().lock_owned().await;
    let dir = tempdir().expect("tempdir");
    let stack_a = TestIrohStack::new(&dir.path().join("multi-a")).await;
    let stack_b = TestIrohStack::new(&dir.path().join("multi-b")).await;
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let app_a = app_with_iroh_services(store_a, &stack_a);
    let app_b = app_with_iroh_services(store_b, &stack_b);
    let topic_one = "kukuri:topic:one";
    let topic_two = "kukuri:topic:two";

    let ticket_a = app_a
        .peer_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = app_b
        .peer_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");
    app_a
        .import_peer_ticket(&ticket_b)
        .await
        .expect("import b into a");
    app_b
        .import_peer_ticket(&ticket_a)
        .await
        .expect("import a into b");

    let _ = app_a
        .list_timeline(topic_one, None, 20)
        .await
        .expect("subscribe a topic one");
    let _ = app_a
        .list_timeline(topic_two, None, 20)
        .await
        .expect("subscribe a topic two");
    let _ = app_b
        .list_timeline(topic_one, None, 20)
        .await
        .expect("subscribe b topic one");
    let _ = app_b
        .list_timeline(topic_two, None, 20)
        .await
        .expect("subscribe b topic two");

    let id_one = app_a
        .create_post(topic_one, "topic one from a", None)
        .await
        .expect("post one");
    let id_two = app_b
        .create_post(topic_two, "topic two from b", None)
        .await
        .expect("post two");

    timeout(Duration::from_secs(10), async {
        loop {
            let timeline_b = app_b
                .list_timeline(topic_one, None, 20)
                .await
                .expect("timeline b");
            let timeline_a = app_a
                .list_timeline(topic_two, None, 20)
                .await
                .expect("timeline a");
            let has_one = timeline_b.items.iter().any(|post| post.object_id == id_one);
            let has_two = timeline_a.items.iter().any(|post| post.object_id == id_two);
            if has_one && has_two {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("multi topic propagation timeout");
}
