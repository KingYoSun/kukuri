use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use iroh::address_lookup::MemoryLookup;
use iroh::protocol::Router;
use iroh::{Endpoint, EndpointAddr, RelayUrl, Watcher};
use iroh_blobs::api::Store as BlobStore;
use iroh_blobs::store::{fs::options::Options as BlobStoreOptions, mem::MemStore};
use iroh_docs::api::{Doc, DocsApi};
use iroh_docs::store::Query;
use iroh_docs::{Capability, DocTicket, NamespaceSecret};
use iroh_gossip::net::Gossip;
use kukuri_core::{ReplicaId, blob_hash};
use kukuri_transport::{
    DhtDiscoveryOptions, SeedPeer, TransportNetworkConfig, TransportRelayConfig,
    build_endpoint_builder, parse_endpoint_ticket, prepare_endpoint_for_discovery,
    sync_endpoint_relay_config,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, broadcast};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{info, warn};

pub type DocEventStream = Pin<Box<dyn Stream<Item = Result<DocEvent>> + Send>>;
type ReplicaRecords = HashMap<String, Vec<u8>>;
type MemoryReplicaMap = HashMap<String, ReplicaRecords>;
const ENDPOINT_SECRET_FILE_NAME: &str = "endpoint-secret.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocOp {
    SetJson {
        key: String,
        value: serde_json::Value,
    },
    SetBytes {
        key: String,
        value: Vec<u8>,
    },
    DeletePrefix {
        prefix: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocQuery {
    Exact(String),
    Prefix(String),
    All,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocRecord {
    pub key: String,
    pub value: Vec<u8>,
    pub content_hash: String,
    pub content_len: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocEvent {
    pub replica_id: ReplicaId,
    pub key: String,
    pub content_hash: String,
    pub source_peer: Option<String>,
}

#[async_trait]
pub trait DocsSync: Send + Sync {
    async fn open_replica(&self, replica_id: &ReplicaId) -> Result<()>;
    async fn apply_doc_op(&self, replica_id: &ReplicaId, op: DocOp) -> Result<()>;
    async fn query_replica(
        &self,
        replica_id: &ReplicaId,
        query: DocQuery,
    ) -> Result<Vec<DocRecord>>;
    async fn subscribe_replica(&self, replica_id: &ReplicaId) -> Result<DocEventStream>;
    async fn import_peer_ticket(&self, ticket: &str) -> Result<()>;
    async fn set_seed_peers(&self, _peers: Vec<SeedPeer>) -> Result<()> {
        Ok(())
    }
    async fn assist_peer_ids(&self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
}

pub struct IrohDocsNode {
    endpoint: Endpoint,
    gossip: Gossip,
    discovery: Arc<MemoryLookup>,
    relay_urls: Mutex<Vec<RelayUrl>>,
    router: Arc<Router>,
    docs: DocsApi,
    blobs: BlobStore,
    endpoint_publish_task: Option<JoinHandle<()>>,
}

#[derive(Serialize, Deserialize)]
struct StoredEndpointSecret {
    secret_key: iroh::SecretKey,
}

impl IrohDocsNode {
    pub async fn memory() -> Result<Arc<Self>> {
        let store = MemStore::new();
        Self::spawn(
            (*store).clone(),
            None,
            TransportNetworkConfig::loopback(),
            DhtDiscoveryOptions::disabled(),
            TransportRelayConfig::default(),
        )
        .await
    }

    pub async fn persistent(root: impl AsRef<Path>) -> Result<Arc<Self>> {
        Self::persistent_with_config(root, TransportNetworkConfig::loopback()).await
    }

    pub async fn persistent_with_config(
        root: impl AsRef<Path>,
        network_config: TransportNetworkConfig,
    ) -> Result<Arc<Self>> {
        Self::persistent_with_discovery_config(
            root,
            network_config,
            DhtDiscoveryOptions::disabled(),
            TransportRelayConfig::default(),
        )
        .await
    }

    pub async fn persistent_with_discovery_config(
        root: impl AsRef<Path>,
        network_config: TransportNetworkConfig,
        dht_options: DhtDiscoveryOptions,
        relay_config: TransportRelayConfig,
    ) -> Result<Arc<Self>> {
        let root = root.as_ref();
        std::fs::create_dir_all(root)
            .with_context(|| format!("failed to create docs root {}", root.display()))?;
        let options = BlobStoreOptions::new(root);
        let store = iroh_blobs::store::fs::FsStore::load_with_opts(root.join("blobs.db"), options)
            .await
            .with_context(|| format!("failed to load blob store at {}", root.display()))?;
        Self::spawn(
            (*store).clone(),
            Some(root.to_path_buf()),
            network_config,
            dht_options,
            relay_config,
        )
        .await
    }

    async fn spawn(
        store: impl Into<BlobStore>,
        root: Option<PathBuf>,
        network_config: TransportNetworkConfig,
        dht_options: DhtDiscoveryOptions,
        relay_config: TransportRelayConfig,
    ) -> Result<Arc<Self>> {
        let blobs = store.into();
        let discovery = Arc::new(MemoryLookup::new());
        let endpoint_secret = root
            .as_deref()
            .map(load_endpoint_secret)
            .transpose()?
            .flatten();
        let relay_config = relay_config.normalized();
        let relay_urls = relay_config.parsed_relay_urls()?;
        let mut endpoint_builder = build_endpoint_builder(
            Endpoint::empty_builder(relay_config.relay_mode()?),
            &discovery,
            Some(&dht_options),
            &relay_config,
        )?;
        #[cfg(test)]
        {
            endpoint_builder = endpoint_builder.insecure_skip_relay_cert_verify(true);
        }
        if let Some(secret_key) = endpoint_secret {
            endpoint_builder = endpoint_builder.secret_key(secret_key);
        }
        endpoint_builder = match network_config.bind_addr {
            std::net::SocketAddr::V4(addr) => endpoint_builder.bind_addr(addr)?,
            std::net::SocketAddr::V6(addr) => endpoint_builder.bind_addr(addr)?,
        };
        let endpoint = endpoint_builder
            .bind()
            .await
            .context("failed to bind iroh endpoint for docs node")?;
        if let Some(root) = root.as_deref() {
            save_endpoint_secret(root, endpoint.secret_key())?;
        }
        let endpoint_publish_task =
            prepare_endpoint_for_discovery(&endpoint, &discovery, &dht_options, &relay_config)
                .await?;
        let gossip = Gossip::builder().spawn(endpoint.clone());
        let docs_builder = match root {
            Some(path) => iroh_docs::protocol::Docs::persistent(path),
            None => iroh_docs::protocol::Docs::memory(),
        };
        let docs = docs_builder
            .spawn(endpoint.clone(), blobs.clone(), gossip.clone())
            .await
            .context("failed to spawn iroh docs")?;
        let router = Router::builder(endpoint.clone())
            .accept(
                iroh_blobs::ALPN,
                iroh_blobs::BlobsProtocol::new(&blobs, None),
            )
            .accept(iroh_docs::ALPN, docs.clone())
            .accept(iroh_gossip::ALPN, gossip.clone())
            .spawn();

        Ok(Arc::new(Self {
            endpoint,
            gossip,
            discovery,
            relay_urls: Mutex::new(relay_urls),
            router: Arc::new(router),
            docs: docs.api().clone(),
            blobs,
            endpoint_publish_task,
        }))
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn gossip(&self) -> &Gossip {
        &self.gossip
    }

    pub fn discovery(&self) -> Arc<MemoryLookup> {
        self.discovery.clone()
    }

    pub async fn relay_urls(&self) -> Vec<RelayUrl> {
        self.relay_urls.lock().await.clone()
    }

    pub async fn apply_relay_config(&self, relay_config: TransportRelayConfig) -> Result<()> {
        let relay_config = relay_config.normalized();
        let next_relay_urls = relay_config.parsed_relay_urls()?;
        let current_relay_urls = self.relay_urls.lock().await.clone();
        sync_endpoint_relay_config(&self.endpoint, &current_relay_urls, &next_relay_urls).await?;
        if !next_relay_urls.is_empty() {
            let mut addr_watcher = self.endpoint.watch_addr();
            let expected_relays = next_relay_urls.iter().cloned().collect::<BTreeSet<_>>();
            let relay_ready = |addr: &EndpointAddr| {
                addr.relay_urls()
                    .any(|relay_url| expected_relays.contains(relay_url))
            };
            if !relay_ready(&addr_watcher.get()) {
                timeout(Duration::from_secs(10), async move {
                    let mut stream = addr_watcher.stream();
                    while let Some(addr) = stream.next().await {
                        if relay_ready(&addr) {
                            return Ok::<(), anyhow::Error>(());
                        }
                    }
                    bail!("relay address watcher ended before any configured relay became active")
                })
                .await
                .context("timed out waiting for live relay connectivity")??;
            }
        }
        *self.relay_urls.lock().await = next_relay_urls;
        self.discovery.add_endpoint_info(self.endpoint.addr());
        Ok(())
    }

    pub fn docs(&self) -> &DocsApi {
        &self.docs
    }

    pub fn blobs(&self) -> &BlobStore {
        &self.blobs
    }

    pub async fn shutdown(self: Arc<Self>) -> Result<()> {
        if let Some(task) = &self.endpoint_publish_task {
            task.abort();
        }
        self.router.shutdown().await?;
        self.endpoint.close().await;
        let _ = self.blobs.shutdown().await;
        Ok(())
    }
}

impl Drop for IrohDocsNode {
    fn drop(&mut self) {
        if let Some(task) = self.endpoint_publish_task.take() {
            task.abort();
        }
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let router = Arc::clone(&self.router);
            let blobs = self.blobs.clone();
            let endpoint = self.endpoint.clone();
            handle.spawn(async move {
                let _ = router.shutdown().await;
                endpoint.close().await;
                let _ = blobs.shutdown().await;
            });
        }
    }
}

fn endpoint_secret_path(root: &Path) -> PathBuf {
    root.join(ENDPOINT_SECRET_FILE_NAME)
}

fn load_endpoint_secret(root: &Path) -> Result<Option<iroh::SecretKey>> {
    let path = endpoint_secret_path(root);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path)
        .with_context(|| format!("failed to read endpoint secret at {}", path.display()))?;
    let stored: StoredEndpointSecret = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse endpoint secret at {}", path.display()))?;
    Ok(Some(stored.secret_key))
}

fn save_endpoint_secret(root: &Path, secret_key: &iroh::SecretKey) -> Result<()> {
    let path = endpoint_secret_path(root);
    let bytes = serde_json::to_vec(&StoredEndpointSecret {
        secret_key: secret_key.clone(),
    })
    .with_context(|| format!("failed to serialize endpoint secret at {}", path.display()))?;
    std::fs::write(&path, bytes)
        .with_context(|| format!("failed to write endpoint secret at {}", path.display()))?;
    Ok(())
}

struct ReplicaHandle {
    doc: Doc,
    events: broadcast::Sender<DocEvent>,
    sync_peer_ids: BTreeSet<String>,
    _live_task: JoinHandle<()>,
}

#[derive(Clone)]
pub struct IrohDocsSync {
    node: Arc<IrohDocsNode>,
    replicas: Arc<Mutex<HashMap<String, ReplicaHandle>>>,
    seed_peers: Arc<Mutex<BTreeMap<String, EndpointAddr>>>,
    imported_peers: Arc<Mutex<BTreeMap<String, EndpointAddr>>>,
}

#[derive(Clone, Default)]
pub struct MemoryDocsSync {
    records: Arc<Mutex<MemoryReplicaMap>>,
    events: Arc<Mutex<HashMap<String, broadcast::Sender<DocEvent>>>>,
}

impl IrohDocsSync {
    pub fn new(node: Arc<IrohDocsNode>) -> Self {
        Self {
            node,
            replicas: Arc::new(Mutex::new(HashMap::new())),
            seed_peers: Arc::new(Mutex::new(BTreeMap::new())),
            imported_peers: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    async fn sync_peers(&self) -> Vec<EndpointAddr> {
        let mut peers = self
            .seed_peers
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for peer in self.imported_peers.lock().await.values() {
            if !peers.iter().any(|existing| existing.id == peer.id) {
                peers.push(peer.clone());
            }
        }
        peers
    }

    async fn available_sync_peer_ids(&self) -> Vec<String> {
        let peers = self.sync_peers().await;
        let mut available = BTreeSet::new();
        for peer in peers {
            if !peer.is_empty() || self.node.endpoint().remote_info(peer.id).await.is_some() {
                available.insert(peer.id.to_string());
            }
        }
        available.into_iter().collect()
    }

    async fn reapply_sync_peers(&self) -> Result<()> {
        let peers = self.sync_peers().await;
        let peer_ids = peers
            .iter()
            .map(|peer| peer.id.to_string())
            .collect::<BTreeSet<_>>();
        let mut replicas = self.replicas.lock().await;
        for handle in replicas.values_mut() {
            doc_start_sync(&handle.doc, peers.clone()).await?;
            handle.sync_peer_ids = peer_ids.clone();
        }
        Ok(())
    }

    async fn ensure_replica(&self, replica_id: &ReplicaId) -> Result<Doc> {
        let imported = self.sync_peers().await;
        let imported_ids = imported
            .iter()
            .map(|peer| peer.id.to_string())
            .collect::<BTreeSet<_>>();

        if let Some(handle) = self.replicas.lock().await.get_mut(replica_id.as_str()) {
            if handle.sync_peer_ids != imported_ids && !imported.is_empty() {
                doc_start_sync(&handle.doc, imported.clone()).await?;
                handle.sync_peer_ids = imported_ids;
            }
            return Ok(handle.doc.clone());
        }

        let secret = replica_secret(replica_id);
        let doc = if imported.is_empty() {
            self.node
                .docs()
                .import_namespace(Capability::Write(secret))
                .await?
        } else {
            self.node
                .docs()
                .import(DocTicket {
                    capability: Capability::Write(secret),
                    nodes: imported.clone(),
                })
                .await?
        };
        doc_start_sync(&doc, imported.clone()).await?;
        let (tx, _) = broadcast::channel(256);
        let mut live = doc.subscribe().await?;
        let live_replica = replica_id.clone();
        let live_events = tx.clone();
        let task = tokio::spawn(async move {
            while let Some(item) = live.next().await {
                if let Ok(event) = item {
                    match event {
                        iroh_docs::engine::LiveEvent::InsertLocal { entry } => {
                            let _ = live_events.send(DocEvent {
                                replica_id: live_replica.clone(),
                                key: String::from_utf8_lossy(entry.key()).to_string(),
                                content_hash: entry.content_hash().to_string(),
                                source_peer: None,
                            });
                        }
                        iroh_docs::engine::LiveEvent::InsertRemote { from, entry, .. } => {
                            let _ = live_events.send(DocEvent {
                                replica_id: live_replica.clone(),
                                key: String::from_utf8_lossy(entry.key()).to_string(),
                                content_hash: entry.content_hash().to_string(),
                                source_peer: Some(from.to_string()),
                            });
                        }
                        _ => {}
                    }
                }
            }
        });
        self.replicas.lock().await.insert(
            replica_id.as_str().to_string(),
            ReplicaHandle {
                doc: doc.clone(),
                events: tx,
                sync_peer_ids: imported_ids,
                _live_task: task,
            },
        );
        Ok(doc)
    }

    async fn sender(&self, replica_id: &ReplicaId) -> Result<broadcast::Sender<DocEvent>> {
        self.ensure_replica(replica_id).await?;
        let guard = self.replicas.lock().await;
        let sender = guard
            .get(replica_id.as_str())
            .map(|handle| handle.events.clone())
            .context("missing replica sender")?;
        Ok(sender)
    }

    async fn connect_candidates(&self, imported_peer: &EndpointAddr) -> Vec<EndpointAddr> {
        let mut candidates = Vec::new();
        if let Some(remote_info) = self.node.endpoint().remote_info(imported_peer.id).await {
            let learned_peer = EndpointAddr::from_parts(
                remote_info.id(),
                remote_info.into_addrs().map(|addr| addr.into_addr()),
            );
            if !learned_peer.is_empty() {
                candidates.push(learned_peer);
            }
        }
        if !candidates
            .iter()
            .any(|candidate| candidate == imported_peer)
        {
            candidates.push(imported_peer.clone());
        }
        candidates
    }

    async fn fetch_entry_bytes(&self, content_hash: &str) -> Result<Option<Vec<u8>>> {
        let hash = iroh_blobs::Hash::from_str(content_hash)?;
        match self.node.blobs().blobs().get_bytes(hash).await {
            Ok(bytes) => Ok(Some(bytes.to_vec())),
            Err(error) => {
                let peers = self.sync_peers().await;
                info!(
                    hash = %content_hash,
                    error = %error,
                    configured_peer_count = peers.len(),
                    "docs entry fetch local miss, trying remote peers"
                );
                for imported_peer in peers {
                    let candidates = self.connect_candidates(&imported_peer).await;
                    for peer in candidates {
                        match self
                            .node
                            .endpoint()
                            .connect(peer.clone(), iroh_blobs::ALPN)
                            .await
                        {
                            Ok(conn) => match self.node.blobs().remote().fetch(conn, hash).await {
                                Ok(_) => match self.node.blobs().blobs().get_bytes(hash).await {
                                    Ok(bytes) => return Ok(Some(bytes.to_vec())),
                                    Err(error) => {
                                        warn!(
                                            hash = %content_hash,
                                            peer_id = %peer.id,
                                            error = %error,
                                            "docs entry transfer completed but content is still missing locally"
                                        );
                                    }
                                },
                                Err(error) => {
                                    warn!(
                                        hash = %content_hash,
                                        peer_id = %peer.id,
                                        addrs = ?peer.addrs,
                                        error = %error,
                                        "docs entry remote transfer failed"
                                    );
                                }
                            },
                            Err(error) => {
                                warn!(
                                    hash = %content_hash,
                                    peer_id = %peer.id,
                                    addrs = ?peer.addrs,
                                    error = %error,
                                    "docs entry fetch connect failed"
                                );
                            }
                        }
                    }
                }
                warn!(
                    hash = %content_hash,
                    "docs entry fetch exhausted remote peers without success"
                );
                Ok(None)
            }
        }
    }
}

#[async_trait]
impl DocsSync for MemoryDocsSync {
    async fn open_replica(&self, replica_id: &ReplicaId) -> Result<()> {
        self.records
            .lock()
            .await
            .entry(replica_id.as_str().to_string())
            .or_default();
        self.events
            .lock()
            .await
            .entry(replica_id.as_str().to_string())
            .or_insert_with(|| broadcast::channel(256).0);
        Ok(())
    }

    async fn apply_doc_op(&self, replica_id: &ReplicaId, op: DocOp) -> Result<()> {
        self.open_replica(replica_id).await?;
        let mut records = self.records.lock().await;
        let replica = records.entry(replica_id.as_str().to_string()).or_default();
        match op {
            DocOp::SetJson { key, value } => {
                let bytes = serde_json::to_vec(&value)?;
                replica.insert(key.clone(), bytes.clone());
                let _ = self
                    .events
                    .lock()
                    .await
                    .get(replica_id.as_str())
                    .cloned()
                    .context("missing events sender")?
                    .send(DocEvent {
                        replica_id: replica_id.clone(),
                        key,
                        content_hash: value_hash(bytes),
                        source_peer: None,
                    });
            }
            DocOp::SetBytes { key, value } => {
                let hash = value_hash(&value);
                replica.insert(key.clone(), value);
                let _ = self
                    .events
                    .lock()
                    .await
                    .get(replica_id.as_str())
                    .cloned()
                    .context("missing events sender")?
                    .send(DocEvent {
                        replica_id: replica_id.clone(),
                        key,
                        content_hash: hash,
                        source_peer: None,
                    });
            }
            DocOp::DeletePrefix { prefix } => {
                replica.retain(|key, _| !key.starts_with(prefix.as_str()));
            }
        }
        Ok(())
    }

    async fn query_replica(
        &self,
        replica_id: &ReplicaId,
        query: DocQuery,
    ) -> Result<Vec<DocRecord>> {
        self.open_replica(replica_id).await?;
        let records = self.records.lock().await;
        let items = records
            .get(replica_id.as_str())
            .cloned()
            .unwrap_or_default();
        let mut rows = items
            .into_iter()
            .filter(|(key, _)| match &query {
                DocQuery::Exact(exact) => key == exact,
                DocQuery::Prefix(prefix) => key.starts_with(prefix.as_str()),
                DocQuery::All => true,
            })
            .map(|(key, value)| DocRecord {
                content_hash: value_hash(&value),
                content_len: value.len() as u64,
                key,
                value,
            })
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| left.key.cmp(&right.key));
        Ok(rows)
    }

    async fn subscribe_replica(&self, replica_id: &ReplicaId) -> Result<DocEventStream> {
        self.open_replica(replica_id).await?;
        let sender = self
            .events
            .lock()
            .await
            .get(replica_id.as_str())
            .cloned()
            .context("missing replica events")?;
        let stream = BroadcastStream::new(sender.subscribe())
            .filter_map(|item| async move { item.ok().map(Ok) });
        Ok(Box::pin(stream))
    }

    async fn import_peer_ticket(&self, _ticket: &str) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl DocsSync for IrohDocsSync {
    async fn open_replica(&self, replica_id: &ReplicaId) -> Result<()> {
        let _ = self.ensure_replica(replica_id).await?;
        Ok(())
    }

    async fn apply_doc_op(&self, replica_id: &ReplicaId, op: DocOp) -> Result<()> {
        let doc = self.ensure_replica(replica_id).await?;
        let author = self.node.docs().author_default().await?;
        let sender = self.sender(replica_id).await?;

        match op {
            DocOp::SetJson { key, value } => {
                let payload = serde_json::to_vec(&value)?;
                let content_hash = doc
                    .set_bytes(author, key.as_bytes().to_vec(), payload.clone())
                    .await?;
                let _ = sender.send(DocEvent {
                    replica_id: replica_id.clone(),
                    key,
                    content_hash: content_hash.to_string(),
                    source_peer: None,
                });
            }
            DocOp::SetBytes { key, value } => {
                let content_hash = doc
                    .set_bytes(author, key.as_bytes().to_vec(), value)
                    .await?;
                let _ = sender.send(DocEvent {
                    replica_id: replica_id.clone(),
                    key,
                    content_hash: content_hash.to_string(),
                    source_peer: None,
                });
            }
            DocOp::DeletePrefix { prefix } => {
                let _ = doc.del(author, prefix.as_bytes().to_vec()).await?;
            }
        }
        Ok(())
    }

    async fn query_replica(
        &self,
        replica_id: &ReplicaId,
        query: DocQuery,
    ) -> Result<Vec<DocRecord>> {
        let doc = self.ensure_replica(replica_id).await?;
        let query = match query {
            DocQuery::Exact(key) => Query::key_exact(key).build(),
            DocQuery::Prefix(prefix) => Query::key_prefix(prefix).build(),
            DocQuery::All => Query::all().build(),
        };
        let stream = doc.get_many(query).await?;
        tokio::pin!(stream);
        let mut records = Vec::new();
        while let Some(entry) = stream.next().await {
            let entry = entry?;
            let key = String::from_utf8(entry.key().to_vec()).context("docs key is not utf8")?;
            let content_hash = entry.content_hash().to_string();
            let Some(value) = self.fetch_entry_bytes(content_hash.as_str()).await? else {
                continue;
            };
            records.push(DocRecord {
                key,
                value,
                content_hash,
                content_len: entry.content_len(),
            });
        }
        Ok(records)
    }

    async fn subscribe_replica(&self, replica_id: &ReplicaId) -> Result<DocEventStream> {
        let sender = self.sender(replica_id).await?;
        let stream = BroadcastStream::new(sender.subscribe())
            .filter_map(|item| async move { item.ok().map(Ok) });
        Ok(Box::pin(stream))
    }

    async fn import_peer_ticket(&self, ticket: &str) -> Result<()> {
        let endpoint_addr = parse_endpoint_ticket(ticket)?;
        self.node
            .discovery()
            .add_endpoint_info(endpoint_addr.clone());
        self.imported_peers
            .lock()
            .await
            .insert(endpoint_addr.id.to_string(), endpoint_addr.clone());
        self.reapply_sync_peers().await?;
        Ok(())
    }

    async fn set_seed_peers(&self, peers: Vec<SeedPeer>) -> Result<()> {
        let relay_urls = self.node.relay_urls().await;
        let mut parsed = BTreeMap::new();
        for peer in peers {
            let endpoint_addr = peer.to_endpoint_addr_with_relays(&relay_urls)?;
            if !endpoint_addr.is_empty() {
                self.node
                    .discovery()
                    .add_endpoint_info(endpoint_addr.clone());
            }
            parsed.insert(endpoint_addr.id.to_string(), endpoint_addr);
        }
        *self.seed_peers.lock().await = parsed;
        self.reapply_sync_peers().await
    }

    async fn assist_peer_ids(&self) -> Result<Vec<String>> {
        Ok(self.available_sync_peer_ids().await)
    }
}

fn replica_secret(replica_id: &ReplicaId) -> NamespaceSecret {
    let digest = blake3::hash(format!("kukuri-docs:{}", replica_id.as_str()).as_bytes());
    NamespaceSecret::from_bytes(digest.as_bytes())
}

pub fn topic_replica_id(topic_id: &str) -> ReplicaId {
    ReplicaId::new(format!("topic::{topic_id}"))
}

pub fn author_replica_id(author_pubkey: &str) -> ReplicaId {
    ReplicaId::new(format!("author::{author_pubkey}"))
}

pub fn device_replica_id(author_pubkey: &str, device_id: &str) -> ReplicaId {
    ReplicaId::new(format!("device::{author_pubkey}::{device_id}"))
}

pub fn stable_key(prefix: &str, key: &str) -> String {
    format!("{prefix}/{key}")
}

pub fn value_hash(value: impl AsRef<[u8]>) -> String {
    blob_hash(value).0
}

async fn doc_start_sync(doc: &Doc, peers: Vec<EndpointAddr>) -> Result<()> {
    doc.start_sync(peers).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn docs_topic_index_roundtrip() {
        let node = IrohDocsNode::memory().await.expect("memory docs node");
        let docs = IrohDocsSync::new(node);
        let replica = topic_replica_id("kukuri:topic:docs");

        docs.open_replica(&replica).await.expect("open replica");
        docs.apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("timeline", "0001-event"),
                value: serde_json::json!({
                    "object_id": "event-1",
                    "topic_id": "kukuri:topic:docs"
                }),
            },
        )
        .await
        .expect("apply op");

        let rows = docs
            .query_replica(&replica, DocQuery::Prefix("timeline/".into()))
            .await
            .expect("query replica");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, "timeline/0001-event");
        assert!(
            String::from_utf8(rows[0].value.clone())
                .expect("utf8")
                .contains("event-1")
        );
    }

    #[tokio::test]
    async fn private_cursor_not_in_public_replica() {
        let node = IrohDocsNode::memory().await.expect("memory docs node");
        let docs = IrohDocsSync::new(node);
        let topic_replica = topic_replica_id("kukuri:topic:docs");
        let author_replica = author_replica_id("f".repeat(64).as_str());
        let device_replica = device_replica_id("f".repeat(64).as_str(), "device-a");

        docs.open_replica(&topic_replica)
            .await
            .expect("open topic replica");
        docs.open_replica(&author_replica)
            .await
            .expect("open author replica");
        docs.open_replica(&device_replica)
            .await
            .expect("open device replica");

        docs.apply_doc_op(
            &device_replica,
            DocOp::SetJson {
                key: "cursor/topic/kukuri:topic:docs".into(),
                value: serde_json::json!({ "created_at": 1 }),
            },
        )
        .await
        .expect("write device cursor");

        let topic_rows = docs
            .query_replica(&topic_replica, DocQuery::Prefix("cursor/".into()))
            .await
            .expect("query topic cursor");
        let author_rows = docs
            .query_replica(&author_replica, DocQuery::Prefix("cursor/".into()))
            .await
            .expect("query author cursor");
        let device_rows = docs
            .query_replica(&device_replica, DocQuery::Prefix("cursor/".into()))
            .await
            .expect("query device cursor");

        assert!(topic_rows.is_empty());
        assert!(author_rows.is_empty());
        assert_eq!(device_rows.len(), 1);
        assert_eq!(device_rows[0].key, "cursor/topic/kukuri:topic:docs");
    }
}
