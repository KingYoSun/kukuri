use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use futures_util::StreamExt;
use iroh::EndpointAddr;
use iroh::EndpointId;
use iroh_docs::api::Doc;
use iroh_docs::store::Query;
use iroh_docs::{Capability, DocTicket, NamespaceSecret};
use kukuri_core::ReplicaId;
use kukuri_transport::{SeedPeer, parse_endpoint_ticket};
use tokio::sync::{Mutex, broadcast};
use tokio::task::JoinHandle;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{info, warn};

use crate::access::parse_namespace_secret_hex;
use crate::node::IrohDocsNode;
use crate::replicas::public_replica_secret;
use crate::types::{DocEvent, DocEventStream, DocOp, DocQuery, DocRecord, DocsSync};

struct ReplicaHandle {
    doc: Doc,
    events: broadcast::Sender<DocEvent>,
    sync_peer_ids: BTreeSet<String>,
    live_task: JoinHandle<()>,
}

#[derive(Clone, Debug, Default)]
pub struct DocsPeerState {
    pub learned_peers: Vec<EndpointAddr>,
    pub imported_peers: Vec<EndpointAddr>,
}

#[derive(Clone)]
pub struct IrohDocsSync {
    node: Arc<IrohDocsNode>,
    replicas: Arc<Mutex<HashMap<String, ReplicaHandle>>>,
    learned_peers: Arc<Mutex<BTreeMap<String, EndpointAddr>>>,
    seed_peers: Arc<Mutex<BTreeMap<String, EndpointAddr>>>,
    imported_peers: Arc<Mutex<BTreeMap<String, EndpointAddr>>>,
    private_replica_secrets: Arc<Mutex<HashMap<String, NamespaceSecret>>>,
}

impl IrohDocsSync {
    pub fn new(node: Arc<IrohDocsNode>) -> Self {
        Self {
            node,
            replicas: Arc::new(Mutex::new(HashMap::new())),
            learned_peers: Arc::new(Mutex::new(BTreeMap::new())),
            seed_peers: Arc::new(Mutex::new(BTreeMap::new())),
            imported_peers: Arc::new(Mutex::new(BTreeMap::new())),
            private_replica_secrets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn shutdown(&self) {
        let handles = {
            let mut replicas = self.replicas.lock().await;
            replicas
                .drain()
                .map(|(_, handle)| handle)
                .collect::<Vec<_>>()
        };
        for handle in handles {
            handle.live_task.abort();
        }
    }

    async fn sync_peers(&self) -> Vec<EndpointAddr> {
        let mut peers = self
            .learned_peers
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for peer in self.seed_peers.lock().await.values() {
            if !peers.iter().any(|existing| existing.id == peer.id) {
                peers.push(peer.clone());
            }
        }
        for peer in self.imported_peers.lock().await.values() {
            if !peers.iter().any(|existing| existing.id == peer.id) {
                peers.push(peer.clone());
            }
        }
        peers
    }

    async fn insert_learned_peer_addr(&self, endpoint_addr: EndpointAddr) {
        if !endpoint_addr.is_empty() {
            self.node
                .discovery()
                .add_endpoint_info(endpoint_addr.clone());
        }
        self.learned_peers
            .lock()
            .await
            .insert(endpoint_addr.id.to_string(), endpoint_addr);
    }

    async fn insert_imported_peer_addr(&self, endpoint_addr: EndpointAddr) {
        self.node
            .discovery()
            .add_endpoint_info(endpoint_addr.clone());
        self.imported_peers
            .lock()
            .await
            .insert(endpoint_addr.id.to_string(), endpoint_addr);
    }

    pub async fn peer_state(&self) -> DocsPeerState {
        DocsPeerState {
            learned_peers: self.learned_peers.lock().await.values().cloned().collect(),
            imported_peers: self.imported_peers.lock().await.values().cloned().collect(),
        }
    }

    pub async fn restore_peer_state(&self, state: DocsPeerState) -> Result<()> {
        for endpoint_addr in state.learned_peers {
            self.insert_learned_peer_addr(endpoint_addr).await;
        }
        for endpoint_addr in state.imported_peers {
            self.insert_imported_peer_addr(endpoint_addr).await;
        }
        self.reapply_sync_peers().await
    }

    async fn record_learned_peer(&self, endpoint_id: &str) -> Result<()> {
        let endpoint_id = EndpointId::from_str(endpoint_id.trim())?;
        let relay_urls = self.node.relay_urls().await;
        let mut endpoint_addr = self
            .node
            .endpoint()
            .remote_info(endpoint_id)
            .await
            .map(|remote_info| {
                EndpointAddr::from_parts(
                    remote_info.id(),
                    remote_info.into_addrs().map(|addr| addr.into_addr()),
                )
            })
            .unwrap_or_else(|| EndpointAddr::new(endpoint_id));
        for relay_url in relay_urls {
            endpoint_addr = endpoint_addr.with_relay_url(relay_url);
        }
        self.insert_learned_peer_addr(endpoint_addr).await;
        Ok(())
    }

    async fn connect_candidates(&self, imported_peer: &EndpointAddr) -> Vec<EndpointAddr> {
        let mut candidates = Vec::new();
        if imported_peer.relay_urls().next().is_some() {
            candidates.push(imported_peer.clone());
        }
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

    pub(crate) async fn available_sync_peer_ids(&self) -> Vec<String> {
        let peers = self.sync_peers().await;
        let mut available = BTreeSet::new();
        for peer in peers {
            if self
                .node
                .endpoint()
                .remote_info(peer.id)
                .await
                .is_some_and(|info| {
                    info.addrs().any(|addr| {
                        matches!(addr.usage(), iroh::endpoint::TransportAddrUsage::Active)
                    })
                })
            {
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

    async fn replica_secret(&self, replica_id: &ReplicaId) -> Result<NamespaceSecret> {
        if let Some(secret) = self
            .private_replica_secrets
            .lock()
            .await
            .get(replica_id.as_str())
            .cloned()
        {
            return Ok(secret);
        }
        public_replica_secret(replica_id)
            .ok_or_else(|| anyhow!("private replica capability is not registered"))
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

        let secret = self.replica_secret(replica_id).await?;
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
                live_task: task,
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
impl DocsSync for IrohDocsSync {
    async fn open_replica(&self, replica_id: &ReplicaId) -> Result<()> {
        let _ = self.ensure_replica(replica_id).await?;
        Ok(())
    }

    async fn register_private_replica_secret(
        &self,
        replica_id: &ReplicaId,
        namespace_secret_hex: &str,
    ) -> Result<()> {
        let secret = parse_namespace_secret_hex(namespace_secret_hex)?;
        self.private_replica_secrets
            .lock()
            .await
            .insert(replica_id.as_str().to_string(), secret);
        Ok(())
    }

    async fn remove_private_replica_secret(&self, replica_id: &ReplicaId) -> Result<()> {
        self.private_replica_secrets
            .lock()
            .await
            .remove(replica_id.as_str());
        if let Some(handle) = self.replicas.lock().await.remove(replica_id.as_str()) {
            handle.live_task.abort();
        }
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
        let stream = futures_util::StreamExt::filter_map(
            BroadcastStream::new(sender.subscribe()),
            |item| async move { item.ok().map(Ok) },
        );
        Ok(Box::pin(stream))
    }

    async fn import_peer_ticket(&self, ticket: &str) -> Result<()> {
        let endpoint_addr = parse_endpoint_ticket(ticket)?;
        self.insert_imported_peer_addr(endpoint_addr).await;
        self.reapply_sync_peers().await?;
        Ok(())
    }

    async fn learn_peer(&self, endpoint_id: &str) -> Result<()> {
        self.record_learned_peer(endpoint_id).await
    }

    async fn restart_replica_sync(&self, replica_id: &ReplicaId) -> Result<()> {
        let peers = self.sync_peers().await;
        let peer_ids = peers
            .iter()
            .map(|peer| peer.id.to_string())
            .collect::<BTreeSet<_>>();
        let existing_doc = self
            .replicas
            .lock()
            .await
            .get(replica_id.as_str())
            .map(|handle| handle.doc.clone());
        if let Some(doc) = existing_doc
            && doc_start_sync(&doc, peers.clone()).await.is_ok()
        {
            if let Some(handle) = self.replicas.lock().await.get_mut(replica_id.as_str()) {
                handle.sync_peer_ids = peer_ids;
            }
            return Ok(());
        }
        if let Some(handle) = self.replicas.lock().await.remove(replica_id.as_str()) {
            handle.live_task.abort();
        }
        let _ = self.ensure_replica(replica_id).await?;
        Ok(())
    }

    async fn set_seed_peers(&self, peers: Vec<SeedPeer>) -> Result<()> {
        let relay_urls = self.node.relay_urls().await;
        let mut parsed = BTreeMap::new();
        for peer in peers {
            let endpoint_addr = peer.to_endpoint_addr_with_relays(&relay_urls)?;
            self.node
                .discovery()
                .add_endpoint_info(endpoint_addr.clone());
            parsed.insert(endpoint_addr.id.to_string(), endpoint_addr);
        }
        *self.seed_peers.lock().await = parsed;
        self.reapply_sync_peers().await
    }

    async fn assist_peer_ids(&self) -> Result<Vec<String>> {
        Ok(self.available_sync_peer_ids().await)
    }
}

async fn doc_start_sync(doc: &Doc, peers: Vec<EndpointAddr>) -> Result<()> {
    doc.start_sync(peers).await
}
