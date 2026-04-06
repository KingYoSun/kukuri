use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use iroh::address_lookup::EndpointInfo;
use kukuri_blob_service::IrohBlobService;
use kukuri_core::build_post_envelope_with_payload;
use kukuri_docs_sync::IrohDocsNode;
use kukuri_docs_sync::IrohDocsSync;
use kukuri_store::{BookmarkedCustomReactionRow, MemoryStore, SqliteStore};
use kukuri_transport::{
    DhtDiscoveryOptions, DiscoveryMode, FakeNetwork, FakeTransport, HintEnvelope, HintStream,
    IrohGossipTransport, SeedPeer,
};
use pkarr::errors::{ConcurrencyError, PublishError, QueryError};
use pkarr::{Client as PkarrClient, SignedPacket, Timestamp, mainline::Testnet};
use std::sync::OnceLock;
use tempfile::tempdir;
use tokio::sync::{Mutex as TokioMutex, broadcast};
use tokio::time::{Duration, sleep, timeout};
use tokio_stream::wrappers::BroadcastStream;

mod direct_messages;
mod game;
mod live;
mod media;
mod notifications;
mod private_channels;
mod reactions;
mod social;
mod sync;
mod timeline;

fn social_graph_propagation_timeout() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(300)
    } else {
        Duration::from_secs(10)
    }
}

fn p2p_replication_timeout() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(60)
    } else {
        Duration::from_secs(10)
    }
}

fn seeded_dht_publish_attempts() -> usize {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        60
    } else {
        20
    }
}

fn seeded_dht_publish_resolve_timeout() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(15)
    } else {
        Duration::from_secs(5)
    }
}

fn iroh_integration_test_lock() -> Arc<TokioMutex<()>> {
    static LOCK: OnceLock<Arc<TokioMutex<()>>> = OnceLock::new();
    LOCK.get_or_init(|| Arc::new(TokioMutex::new(()))).clone()
}

fn format_sync_snapshot(status: &SyncStatus, topic: &str) -> String {
    let topic_status = status
        .topic_diagnostics
        .iter()
        .find(|entry| entry.topic == topic)
        .map(|entry| {
            format!(
                "topic_peers={}, connected_peers={:?}, assist_peer_ids={:?}, configured_peer_ids={:?}, status_detail={}",
                entry.peer_count,
                entry.connected_peers,
                entry.assist_peer_ids,
                entry.configured_peer_ids,
                entry.status_detail
            )
        })
        .unwrap_or_else(|| "topic_status=missing".to_string());
    format!(
        "connected={}, peer_count={}, status_detail={}, last_error={:?}, discovery_connected_peers={:?}, {}",
        status.connected,
        status.peer_count,
        status.status_detail,
        status.last_error,
        status.discovery.connected_peer_ids,
        topic_status
    )
}

async fn wait_for_connected_peer_count(app: &AppService, expected: usize) {
    match timeout(social_graph_propagation_timeout(), async {
        let mut stable_ready_polls = 0usize;
        loop {
            let status = app.get_sync_status().await.expect("sync status");
            if status.connected && status.peer_count >= expected {
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
    {
        Ok(()) => {}
        Err(_) => {
            let status = app.get_sync_status().await.expect("sync status");
            panic!(
                "peer connection timeout; connected={}, peer_count={}, status_detail={}, last_error={:?}, discovery_connected_peers={:?}",
                status.connected,
                status.peer_count,
                status.status_detail,
                status.last_error,
                status.discovery.connected_peer_ids
            );
        }
    }
}

async fn wait_for_topic_peer_count(app: &AppService, topic: &str, expected: usize) {
    match timeout(social_graph_propagation_timeout(), async {
        let mut stable_ready_polls = 0usize;
        loop {
            let status = app.get_sync_status().await.expect("sync status");
            let ready = status.topic_diagnostics.iter().any(|entry| {
                let relay_assisted_ready = entry.assist_peer_ids.len() >= expected;
                entry.topic == topic
                    && entry.joined
                    && entry.peer_count >= expected
                    && (entry.connected_peers.len() >= expected || relay_assisted_ready)
            });
            if ready {
                stable_ready_polls += 1;
                if stable_ready_polls >= 3 {
                    return;
                }
            } else {
                stable_ready_polls = 0;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    {
        Ok(()) => {}
        Err(_) => {
            let snapshot = app
                .get_sync_status()
                .await
                .map(|status| format_sync_snapshot(&status, topic))
                .unwrap_or_else(|_| "failed to read sync status".to_string());
            panic!("topic connected-peer timeout for {topic}; {snapshot}");
        }
    }
}

async fn warm_author_social_view(app: &AppService, author_pubkey: &str, topic: &str) {
    match timeout(social_graph_propagation_timeout(), async {
        loop {
            if app.get_author_social_view(author_pubkey).await.is_ok() {
                return;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(()) => {}
        Err(_) => {
            let snapshot = app
                .get_sync_status()
                .await
                .map(|status| format_sync_snapshot(&status, topic))
                .unwrap_or_else(|_| "failed to read sync status".to_string());
            panic!("author social view warmup timeout for {author_pubkey}; {snapshot}");
        }
    }
}

async fn wait_for_mutual_author_view(app: &AppService, author_pubkey: &str, topic: &str) {
    match timeout(social_graph_propagation_timeout(), async {
        loop {
            let view = app
                .get_author_social_view(author_pubkey)
                .await
                .expect("author social view");
            if view.mutual {
                return;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(()) => {}
        Err(_) => {
            let social_view = app
                .get_author_social_view(author_pubkey)
                .await
                .map(|value| {
                    format!(
                        "following={}, followed_by={}, mutual={}, friend_of_friend={}, fof_via={:?}",
                        value.following,
                        value.followed_by,
                        value.mutual,
                        value.friend_of_friend,
                        value.friend_of_friend_via_pubkeys
                    )
                })
                .unwrap_or_else(|_| "social_view=unavailable".to_string());
            let snapshot = app
                .get_sync_status()
                .await
                .map(|status| format_sync_snapshot(&status, topic))
                .unwrap_or_else(|_| "failed to read sync status".to_string());
            panic!("mutual relationship timeout for {author_pubkey}; {social_view}, {snapshot}");
        }
    }
}

fn is_retryable_friend_only_grant_import_error(message: &str) -> bool {
    message.contains("mutual relationship")
        || message.contains("friend-only grant epoch does not match the current policy")
        || message.contains("friend-only grant owner is not an active participant")
        || message.contains("timed out waiting for friend-only channel replica sync")
}

async fn wait_for_friend_only_grant_import(
    app: &AppService,
    token: &str,
    step_timeout: Duration,
) -> kukuri_core::FriendOnlyGrantPreview {
    match timeout(step_timeout, async {
        loop {
            match app.import_friend_only_grant(token).await {
                Ok(preview) => return preview,
                Err(error)
                    if is_retryable_friend_only_grant_import_error(error.to_string().as_str()) =>
                {
                    sleep(Duration::from_millis(100)).await;
                }
                Err(error) => panic!("friend-only grant import failed: {error:#}"),
            }
        }
    })
    .await
    {
        Ok(preview) => preview,
        Err(_) => {
            let preview =
                kukuri_core::parse_friend_only_grant_token(token).expect("parse grant token");
            let social_view = app
                .get_author_social_view(preview.owner_pubkey.as_str())
                .await
                .map(|value| {
                    format!(
                        "following={}, followed_by={}, mutual={}, friend_of_friend={}, fof_via={:?}",
                        value.following,
                        value.followed_by,
                        value.mutual,
                        value.friend_of_friend,
                        value.friend_of_friend_via_pubkeys
                    )
                })
                .unwrap_or_else(|_| "social_view=unavailable".to_string());
            let snapshot = app
                .get_sync_status()
                .await
                .map(|status| format_sync_snapshot(&status, preview.topic_id.as_str()))
                .unwrap_or_else(|_| "failed to read sync status".to_string());
            panic!(
                "friend-only grant import timeout for {}; {social_view}, {snapshot}",
                preview.owner_pubkey.as_str()
            );
        }
    }
}

fn is_retryable_friend_plus_share_import_error(message: &str) -> bool {
    message.contains("mutual relationship")
        || message.contains("sponsor is not an active participant")
        || message.contains("timed out waiting for friend-plus sponsor participant sync")
        || message.contains("timed out waiting for friend-plus channel replica sync")
}

async fn wait_for_friend_plus_share_import(
    app: &AppService,
    token: &str,
    step_timeout: Duration,
) -> kukuri_core::FriendPlusSharePreview {
    let preview = kukuri_core::parse_friend_plus_share_token(token).expect("parse share token");
    match timeout(step_timeout, async {
        loop {
            match app.import_friend_plus_share(token).await {
                Ok(preview) => return preview,
                Err(error)
                    if is_retryable_friend_plus_share_import_error(error.to_string().as_str()) =>
                {
                    sleep(Duration::from_millis(100)).await;
                }
                Err(error) => panic!("friend-plus share import failed: {error:#}"),
            }
        }
    })
    .await
    {
        Ok(preview) => preview,
        Err(_) => {
            let social_view = app
                .get_author_social_view(preview.sponsor_pubkey.as_str())
                .await
                .map(|value| {
                    format!(
                        "following={}, followed_by={}, mutual={}, friend_of_friend={}, fof_via={:?}",
                        value.following,
                        value.followed_by,
                        value.mutual,
                        value.friend_of_friend,
                        value.friend_of_friend_via_pubkeys
                    )
                })
                .unwrap_or_else(|_| "social_view=unavailable".to_string());
            let snapshot = app
                .get_sync_status()
                .await
                .map(|status| format_sync_snapshot(&status, preview.topic_id.as_str()))
                .unwrap_or_else(|_| "failed to read sync status".to_string());
            panic!(
                "friend-plus share import timeout; sponsor_pubkey={}, {social_view}, {snapshot}",
                preview.sponsor_pubkey.as_str()
            );
        }
    }
}

#[derive(Clone)]
struct StaticTransport {
    peers: Arc<TokioMutex<PeerSnapshot>>,
    hints: Arc<TokioMutex<HashMap<String, broadcast::Sender<HintEnvelope>>>>,
    local_ticket: String,
}

impl StaticTransport {
    fn new(peers: PeerSnapshot) -> Self {
        Self {
            peers: Arc::new(TokioMutex::new(peers)),
            hints: Arc::new(TokioMutex::new(HashMap::new())),
            local_ticket: "static-peer".into(),
        }
    }

    async fn hint_sender(&self, topic: &TopicId) -> broadcast::Sender<HintEnvelope> {
        let mut guard = self.hints.lock().await;
        guard
            .entry(topic.as_str().to_string())
            .or_insert_with(|| broadcast::channel(64).0)
            .clone()
    }
}

#[derive(Clone, Default)]
struct AssistedDocsSync {
    peer_ids: Vec<String>,
}

impl AssistedDocsSync {
    fn new(peer_ids: Vec<&str>) -> Self {
        Self {
            peer_ids: peer_ids.into_iter().map(str::to_string).collect(),
        }
    }
}

#[async_trait]
impl DocsSync for AssistedDocsSync {
    async fn open_replica(&self, _replica_id: &ReplicaId) -> Result<()> {
        Ok(())
    }

    async fn apply_doc_op(&self, _replica_id: &ReplicaId, _op: DocOp) -> Result<()> {
        Ok(())
    }

    async fn query_replica(
        &self,
        _replica_id: &ReplicaId,
        _query: DocQuery,
    ) -> Result<Vec<kukuri_docs_sync::DocRecord>> {
        Ok(Vec::new())
    }

    async fn subscribe_replica(
        &self,
        _replica_id: &ReplicaId,
    ) -> Result<kukuri_docs_sync::DocEventStream> {
        let (sender, _) = broadcast::channel::<kukuri_docs_sync::DocEvent>(1);
        let stream = BroadcastStream::new(sender.subscribe())
            .filter_map(|item| async move { item.ok().map(Ok) });
        Ok(Box::pin(stream))
    }

    async fn import_peer_ticket(&self, _ticket: &str) -> Result<()> {
        Ok(())
    }

    async fn assist_peer_ids(&self) -> Result<Vec<String>> {
        Ok(self.peer_ids.clone())
    }
}

#[derive(Clone, Default)]
struct TrackingDocsSync {
    restarted_replicas: Arc<TokioMutex<Vec<String>>>,
}

#[async_trait]
impl DocsSync for TrackingDocsSync {
    async fn open_replica(&self, _replica_id: &ReplicaId) -> Result<()> {
        Ok(())
    }

    async fn apply_doc_op(&self, _replica_id: &ReplicaId, _op: DocOp) -> Result<()> {
        Ok(())
    }

    async fn query_replica(
        &self,
        _replica_id: &ReplicaId,
        _query: DocQuery,
    ) -> Result<Vec<kukuri_docs_sync::DocRecord>> {
        Ok(Vec::new())
    }

    async fn subscribe_replica(
        &self,
        _replica_id: &ReplicaId,
    ) -> Result<kukuri_docs_sync::DocEventStream> {
        let (sender, _) = broadcast::channel::<kukuri_docs_sync::DocEvent>(1);
        let stream = BroadcastStream::new(sender.subscribe())
            .filter_map(|item| async move { item.ok().map(Ok) });
        Ok(Box::pin(stream))
    }

    async fn import_peer_ticket(&self, _ticket: &str) -> Result<()> {
        Ok(())
    }

    async fn restart_replica_sync(&self, replica_id: &ReplicaId) -> Result<()> {
        self.restarted_replicas
            .lock()
            .await
            .push(replica_id.as_str().to_string());
        Ok(())
    }
}

#[derive(Clone, Default)]
struct AssistedBlobService {
    peer_ids: Vec<String>,
}

impl AssistedBlobService {
    fn new(peer_ids: Vec<&str>) -> Self {
        Self {
            peer_ids: peer_ids.into_iter().map(str::to_string).collect(),
        }
    }
}

#[async_trait]
impl BlobService for AssistedBlobService {
    async fn put_blob(&self, _data: Vec<u8>, mime: &str) -> Result<StoredBlob> {
        Ok(StoredBlob {
            hash: kukuri_core::BlobHash::new("test-hash"),
            mime: mime.to_string(),
            bytes: 0,
        })
    }

    async fn fetch_blob(&self, _hash: &kukuri_core::BlobHash) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }

    async fn pin_blob(&self, _hash: &kukuri_core::BlobHash) -> Result<()> {
        Ok(())
    }

    async fn blob_status(&self, _hash: &kukuri_core::BlobHash) -> Result<BlobStatus> {
        Ok(BlobStatus::Missing)
    }

    async fn import_peer_ticket(&self, _ticket: &str) -> Result<()> {
        Ok(())
    }

    async fn assist_peer_ids(&self) -> Result<Vec<String>> {
        Ok(self.peer_ids.clone())
    }
}

async fn persist_test_post(
    docs_sync: &dyn DocsSync,
    projection_store: Option<&dyn ProjectionStore>,
    keys: &KukuriKeys,
    topic: &TopicId,
    payload_ref: PayloadRef,
    attachments: Vec<kukuri_core::AssetRef>,
    reply_to: Option<&KukuriEnvelope>,
) -> KukuriEnvelope {
    let envelope = build_post_envelope_with_payload(
        keys,
        topic,
        payload_ref,
        attachments,
        Vec::new(),
        reply_to,
        ObjectVisibility::Public,
    )
    .expect("event");
    let object = envelope
        .to_post_object()
        .expect("post object")
        .expect("post object");
    let replica = topic_replica_id(topic.as_str());
    persist_post_object(docs_sync, &replica, object.clone(), envelope.clone())
        .await
        .expect("persist post object");
    if let Some(projection_store) = projection_store {
        ProjectionStore::put_object_projection(
            projection_store,
            projection_row_from_header(&object, None, &replica),
        )
        .await
        .expect("put placeholder projection");
    }
    envelope
}

#[async_trait]
impl Transport for StaticTransport {
    async fn peers(&self) -> Result<PeerSnapshot> {
        Ok(self.peers.lock().await.clone())
    }

    async fn export_ticket(&self) -> Result<Option<String>> {
        Ok(Some(self.local_ticket.clone()))
    }

    async fn import_ticket(&self, _ticket: &str) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl HintTransport for StaticTransport {
    async fn subscribe_hints(&self, topic: &TopicId) -> Result<HintStream> {
        let sender = self.hint_sender(topic).await;
        let stream =
            BroadcastStream::new(sender.subscribe()).filter_map(|item| async move { item.ok() });
        Ok(Box::pin(stream))
    }

    async fn unsubscribe_hints(&self, _topic: &TopicId) -> Result<()> {
        Ok(())
    }

    async fn publish_hint(&self, topic: &TopicId, hint: GossipHint) -> Result<()> {
        let sender = self.hint_sender(topic).await;
        let _ = sender.send(HintEnvelope {
            hint,
            received_at: Utc::now().timestamp_millis(),
            source_peer: "static".into(),
        });
        Ok(())
    }
}

struct TestIrohStack {
    _node: Arc<IrohDocsNode>,
    transport: Arc<IrohGossipTransport>,
    docs_sync: Arc<IrohDocsSync>,
    blob_service: Arc<IrohBlobService>,
}

impl TestIrohStack {
    async fn new(root: &std::path::Path) -> Self {
        Self::new_with_discovery(root, DhtDiscoveryOptions::disabled()).await
    }

    async fn new_with_dht(root: &std::path::Path, testnet: &Testnet) -> Self {
        let stack = Self::new_with_discovery(
            root,
            DhtDiscoveryOptions::with_client(dht_test_client(testnet)),
        )
        .await;
        publish_endpoint_to_testnet(stack._node.endpoint(), testnet).await;
        stack
    }

    async fn new_with_discovery(root: &std::path::Path, dht_options: DhtDiscoveryOptions) -> Self {
        let node = IrohDocsNode::persistent_with_discovery_config(
            root,
            kukuri_transport::TransportNetworkConfig::loopback(),
            dht_options,
            kukuri_transport::TransportRelayConfig::default(),
        )
        .await
        .expect("iroh docs node");
        let transport = Arc::new(
            IrohGossipTransport::from_shared_parts(
                node.endpoint().clone(),
                node.gossip().clone(),
                node.discovery(),
                kukuri_transport::TransportNetworkConfig::loopback(),
                kukuri_transport::TransportRelayConfig::default(),
            )
            .expect("transport"),
        );
        let docs_sync = Arc::new(IrohDocsSync::new(node.clone()));
        let blob_service = Arc::new(IrohBlobService::new(node.clone()));
        Self {
            _node: node,
            transport,
            docs_sync,
            blob_service,
        }
    }
}

fn dht_test_client(testnet: &Testnet) -> PkarrClient {
    let mut builder = PkarrClient::builder();
    builder.no_default_network().bootstrap(&testnet.bootstrap);
    builder.build().expect("pkarr client")
}

fn build_endpoint_signed_packet_with_timestamp(
    endpoint_info: &EndpointInfo,
    secret_key: &iroh::SecretKey,
    ttl: u32,
    timestamp: Timestamp,
) -> SignedPacket {
    use pkarr::dns::{self, rdata};

    let keypair = pkarr::Keypair::from_secret_key(&secret_key.to_bytes());
    let mut builder = SignedPacket::builder().timestamp(timestamp);
    let name = dns::Name::new("_iroh").expect("iroh txt name");
    for entry in endpoint_info.to_txt_strings() {
        let mut txt = rdata::TXT::new();
        txt.add_string(&entry)
            .expect("valid endpoint info txt entry");
        builder = builder.txt(name.clone(), txt.into_owned(), ttl);
    }
    builder.sign(&keypair).expect("sign endpoint info packet")
}

async fn publish_endpoint_to_testnet(endpoint: &iroh::Endpoint, testnet: &Testnet) {
    let client = dht_test_client(testnet);
    let public_key =
        pkarr::PublicKey::try_from(endpoint.id().as_bytes()).expect("pkarr public key");
    let expected_info = EndpointInfo::from(endpoint.addr());
    let mut last_error = None;
    for _ in 0..seeded_dht_publish_attempts() {
        let previous_timestamp = client
            .resolve_most_recent(&public_key)
            .await
            .map(|packet| packet.timestamp());
        let now = Timestamp::now();
        let timestamp = match previous_timestamp {
            Some(previous) if previous >= now => previous + 1,
            _ => now,
        };
        let signed_packet = build_endpoint_signed_packet_with_timestamp(
            &expected_info,
            endpoint.secret_key(),
            1,
            timestamp,
        );
        match client.publish(&signed_packet, previous_timestamp).await {
            Ok(()) => break,
            Err(PublishError::Concurrency(
                ConcurrencyError::ConflictRisk
                | ConcurrencyError::NotMostRecent
                | ConcurrencyError::CasFailed,
            )) => sleep(Duration::from_millis(50)).await,
            Err(error @ PublishError::Query(QueryError::Timeout | QueryError::NoClosestNodes)) => {
                last_error = Some(error);
                sleep(Duration::from_millis(100)).await;
            }
            Err(error) => panic!("publish endpoint info: {error}"),
        }
    }
    if let Some(error) = last_error.take()
        && client
            .resolve_most_recent(&public_key)
            .await
            .as_ref()
            .and_then(|packet| EndpointInfo::from_pkarr_signed_packet(packet).ok())
            .is_none_or(|packet_info| {
                packet_info.to_txt_strings() != expected_info.to_txt_strings()
            })
    {
        panic!("publish endpoint info: {error}");
    }
    timeout(seeded_dht_publish_resolve_timeout(), async {
        loop {
            if client
                .resolve_most_recent(&public_key)
                .await
                .as_ref()
                .and_then(|packet| EndpointInfo::from_pkarr_signed_packet(packet).ok())
                .is_some_and(|packet_info| {
                    packet_info.to_txt_strings() == expected_info.to_txt_strings()
                })
            {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("resolve published endpoint info");
}

async fn configure_seeded_dht(app: &AppService, remote_endpoint_id: String) {
    app.set_discovery_seeds(
        DiscoveryMode::SeededDht,
        false,
        vec![SeedPeer {
            endpoint_id: remote_endpoint_id,
            addr_hint: None,
        }],
        Vec::new(),
    )
    .await
    .expect("configure seeded dht");
}

fn app_with_iroh_services(store: Arc<MemoryStore>, stack: &TestIrohStack) -> AppService {
    AppService::new_with_services(
        store.clone(),
        store,
        stack.transport.clone(),
        stack.transport.clone(),
        stack.docs_sync.clone(),
        stack.blob_service.clone(),
        generate_keys(),
    )
}

fn pending_image_attachment(mime: &str, bytes: &[u8]) -> PendingAttachment {
    PendingAttachment {
        mime: mime.to_string(),
        bytes: bytes.to_vec(),
        role: AssetRole::ImageOriginal,
    }
}

fn pending_video_attachment(role: AssetRole, mime: &str, bytes: &[u8]) -> PendingAttachment {
    PendingAttachment {
        mime: mime.to_string(),
        bytes: bytes.to_vec(),
        role,
    }
}

fn tiny_png_bytes() -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO7ZPioAAAAASUVORK5CYII=")
        .expect("decode png")
}

fn reaction_snapshot_from_view(asset: &CustomReactionAssetView) -> CustomReactionAssetSnapshotV1 {
    CustomReactionAssetSnapshotV1 {
        asset_id: asset.asset_id.clone(),
        owner_pubkey: Pubkey::from(asset.owner_pubkey.as_str()),
        blob_hash: kukuri_core::BlobHash::new(asset.blob_hash.clone()),
        search_key: asset.search_key.clone(),
        mime: asset.mime.clone(),
        bytes: asset.bytes,
        width: asset.width,
        height: asset.height,
    }
}

fn local_app_with_memory_services() -> (
    AppService,
    Arc<MemoryStore>,
    Arc<MemoryDocsSync>,
    Arc<MemoryBlobService>,
) {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let app = AppService::new_with_services(
        store.clone(),
        store.clone(),
        transport,
        Arc::new(NoopHintTransport),
        docs_sync.clone(),
        blob_service.clone(),
        generate_keys(),
    );
    (app, store, docs_sync, blob_service)
}

fn shared_apps_with_memory_services() -> (
    AppService,
    KukuriKeys,
    AppService,
    KukuriKeys,
    Arc<MemoryStore>,
    Arc<MemoryDocsSync>,
    Arc<MemoryBlobService>,
) {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let local_keys = generate_keys();
    let remote_keys = generate_keys();
    let local_app = AppService::new_with_services(
        store.clone(),
        store.clone(),
        transport.clone(),
        Arc::new(NoopHintTransport),
        docs_sync.clone(),
        blob_service.clone(),
        local_keys.clone(),
    );
    let remote_app = AppService::new_with_services(
        store.clone(),
        store.clone(),
        transport,
        Arc::new(NoopHintTransport),
        docs_sync.clone(),
        blob_service.clone(),
        remote_keys.clone(),
    );
    (
        local_app,
        local_keys,
        remote_app,
        remote_keys,
        store,
        docs_sync,
        blob_service,
    )
}

async fn author_profile_post_docs(
    docs_sync: &dyn DocsSync,
    author_pubkey: &str,
) -> Vec<AuthorProfilePostDocV1> {
    docs_sync
        .query_replica(
            &author_replica_id(author_pubkey),
            DocQuery::Prefix("profile/posts/".into()),
        )
        .await
        .expect("profile post docs")
        .into_iter()
        .map(|record| {
            serde_json::from_slice::<AuthorProfilePostDocV1>(record.value.as_slice())
                .expect("decode profile post doc")
        })
        .collect()
}

async fn author_profile_repost_docs(
    docs_sync: &dyn DocsSync,
    author_pubkey: &str,
) -> Vec<AuthorProfileRepostDocV1> {
    docs_sync
        .query_replica(
            &author_replica_id(author_pubkey),
            DocQuery::Prefix("profile/reposts/".into()),
        )
        .await
        .expect("profile repost docs")
        .into_iter()
        .map(|record| {
            serde_json::from_slice::<AuthorProfileRepostDocV1>(record.value.as_slice())
                .expect("decode profile repost doc")
        })
        .collect()
}

async fn author_profile_doc(
    docs_sync: &dyn DocsSync,
    author_pubkey: &str,
) -> Option<AuthorProfileDocV1> {
    docs_sync
        .query_replica(
            &author_replica_id(author_pubkey),
            DocQuery::Exact(stable_key("profile", "latest")),
        )
        .await
        .expect("profile doc")
        .into_iter()
        .next()
        .map(|record| {
            serde_json::from_slice::<AuthorProfileDocV1>(record.value.as_slice())
                .expect("decode profile doc")
        })
}

fn remote_doc_event(replica_id: &ReplicaId, key: String) -> DocEvent {
    DocEvent {
        replica_id: replica_id.clone(),
        key,
        content_hash: "remote-content-hash".into(),
        source_peer: Some("remote-peer".into()),
    }
}

async fn create_remote_object_notification(
    app: &AppService,
    projection_store: &dyn ProjectionStore,
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    event: DocEvent,
) -> bool {
    AppService::maybe_create_notification_for_remote_object_event(
        projection_store,
        docs_sync,
        blob_service,
        app.current_author_pubkey().as_str(),
        &event,
    )
    .await
    .expect("create remote object notification")
}

async fn create_remote_follow_notification(
    app: &AppService,
    store: &dyn Store,
    projection_store: &dyn ProjectionStore,
    docs_sync: &dyn DocsSync,
    author_pubkey: &str,
    event: DocEvent,
) -> bool {
    AppService::maybe_create_notification_for_remote_follow_event(
        store,
        projection_store,
        docs_sync,
        app.current_author_pubkey().as_str(),
        author_pubkey,
        &event,
    )
    .await
    .expect("create remote follow notification")
}

#[derive(Clone)]
struct NoopHintTransport;

#[async_trait]
impl HintTransport for NoopHintTransport {
    async fn subscribe_hints(&self, _topic: &TopicId) -> Result<HintStream> {
        Ok(Box::pin(futures_util::stream::empty()))
    }

    async fn unsubscribe_hints(&self, _topic: &TopicId) -> Result<()> {
        Ok(())
    }

    async fn publish_hint(&self, _topic: &TopicId, _hint: GossipHint) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Default)]
struct CountingClosingHintTransport {
    subscribe_count: Arc<TokioMutex<usize>>,
}

#[async_trait]
impl HintTransport for CountingClosingHintTransport {
    async fn subscribe_hints(&self, _topic: &TopicId) -> Result<HintStream> {
        *self.subscribe_count.lock().await += 1;
        Ok(Box::pin(futures_util::stream::empty()))
    }

    async fn unsubscribe_hints(&self, _topic: &TopicId) -> Result<()> {
        Ok(())
    }

    async fn publish_hint(&self, _topic: &TopicId, _hint: GossipHint) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Default)]
struct TrackingHintTransport {
    hints: Arc<TokioMutex<HashMap<String, broadcast::Sender<HintEnvelope>>>>,
    subscribe_count: Arc<TokioMutex<usize>>,
    unsubscribed_topics: Arc<TokioMutex<Vec<String>>>,
}

impl TrackingHintTransport {
    async fn hint_sender(&self, topic: &TopicId) -> broadcast::Sender<HintEnvelope> {
        let mut guard = self.hints.lock().await;
        guard
            .entry(topic.as_str().to_string())
            .or_insert_with(|| broadcast::channel(64).0)
            .clone()
    }
}

#[async_trait]
impl HintTransport for TrackingHintTransport {
    async fn subscribe_hints(&self, topic: &TopicId) -> Result<HintStream> {
        *self.subscribe_count.lock().await += 1;
        let sender = self.hint_sender(topic).await;
        let stream =
            BroadcastStream::new(sender.subscribe()).filter_map(|item| async move { item.ok() });
        Ok(Box::pin(stream))
    }

    async fn unsubscribe_hints(&self, topic: &TopicId) -> Result<()> {
        self.unsubscribed_topics
            .lock()
            .await
            .push(topic.as_str().to_string());
        Ok(())
    }

    async fn publish_hint(&self, topic: &TopicId, hint: GossipHint) -> Result<()> {
        let sender = self.hint_sender(topic).await;
        let _ = sender.send(HintEnvelope {
            hint,
            received_at: Utc::now().timestamp_millis(),
            source_peer: "tracking".into(),
        });
        Ok(())
    }
}

async fn assert_docs_sync_recovers_post_without_hints(topic: &str, content: &str) {
    let dir = tempdir().expect("tempdir");
    let stack_a = TestIrohStack::new(&dir.path().join("a")).await;
    let stack_b = TestIrohStack::new(&dir.path().join("b")).await;
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let app_a = AppService::new_with_services(
        store_a.clone(),
        store_a,
        stack_a.transport.clone(),
        Arc::new(NoopHintTransport),
        stack_a.docs_sync.clone(),
        stack_a.blob_service.clone(),
        generate_keys(),
    );
    let app_b = AppService::new_with_services(
        store_b.clone(),
        store_b,
        stack_b.transport.clone(),
        Arc::new(NoopHintTransport),
        stack_b.docs_sync.clone(),
        stack_b.blob_service.clone(),
        generate_keys(),
    );

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
    app_a.import_peer_ticket(&ticket_b).await.expect("import b");
    app_b.import_peer_ticket(&ticket_a).await.expect("import a");

    let object_id = app_a
        .create_post(topic, content, None)
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
    .expect("missing gossip timeout");

    assert_eq!(received.content, content);
}
