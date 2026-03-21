use std::collections::{BTreeMap, HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use iroh::EndpointId;
use kukuri_core::BlobHash;
use kukuri_docs_sync::IrohDocsNode;
use kukuri_transport::{SeedPeer, parse_endpoint_ticket};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredBlob {
    pub hash: BlobHash,
    pub mime: String,
    pub bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlobStatus {
    Missing,
    Available,
    Pinned,
}

#[async_trait]
pub trait BlobService: Send + Sync {
    async fn put_blob(&self, data: Vec<u8>, mime: &str) -> Result<StoredBlob>;
    async fn fetch_blob(&self, hash: &BlobHash) -> Result<Option<Vec<u8>>>;
    async fn pin_blob(&self, hash: &BlobHash) -> Result<()>;
    async fn blob_status(&self, hash: &BlobHash) -> Result<BlobStatus>;
    async fn import_peer_ticket(&self, ticket: &str) -> Result<()>;
    async fn learn_peer(&self, _endpoint_id: &str) -> Result<()> {
        Ok(())
    }
    async fn set_seed_peers(&self, _peers: Vec<SeedPeer>) -> Result<()> {
        Ok(())
    }
    async fn assist_peer_ids(&self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
}

#[derive(Clone)]
pub struct IrohBlobService {
    node: Arc<IrohDocsNode>,
    pinned: Arc<RwLock<HashSet<String>>>,
    learned_peers: Arc<Mutex<BTreeMap<String, iroh::EndpointAddr>>>,
    seed_peers: Arc<Mutex<BTreeMap<String, iroh::EndpointAddr>>>,
    imported_peers: Arc<Mutex<BTreeMap<String, iroh::EndpointAddr>>>,
}

#[derive(Clone, Default)]
pub struct MemoryBlobService {
    blobs: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    pinned: Arc<RwLock<HashSet<String>>>,
}

impl IrohBlobService {
    pub fn new(node: Arc<IrohDocsNode>) -> Self {
        Self {
            node,
            pinned: Arc::new(RwLock::new(HashSet::new())),
            learned_peers: Arc::new(Mutex::new(BTreeMap::new())),
            seed_peers: Arc::new(Mutex::new(BTreeMap::new())),
            imported_peers: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    async fn connect_candidates(
        &self,
        imported_peer: &iroh::EndpointAddr,
    ) -> Vec<iroh::EndpointAddr> {
        let mut candidates = Vec::new();
        let imported_uses_relay = imported_peer.relay_urls().next().is_some();
        if imported_uses_relay {
            candidates.push(imported_peer.clone());
        }
        if let Some(remote_info) = self.node.endpoint().remote_info(imported_peer.id).await {
            let learned_peer = iroh::EndpointAddr::from_parts(
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

    async fn fetch_peers(&self) -> Vec<iroh::EndpointAddr> {
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

    async fn available_fetch_peer_ids(&self) -> Vec<String> {
        let peers = self.fetch_peers().await;
        let mut available = std::collections::BTreeSet::new();
        for peer in peers {
            if !peer.is_empty() || self.node.endpoint().remote_info(peer.id).await.is_some() {
                available.insert(peer.id.to_string());
            }
        }
        available.into_iter().collect()
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
                iroh::EndpointAddr::from_parts(
                    remote_info.id(),
                    remote_info.into_addrs().map(|addr| addr.into_addr()),
                )
            })
            .unwrap_or_else(|| iroh::EndpointAddr::new(endpoint_id));
        for relay_url in relay_urls {
            endpoint_addr = endpoint_addr.with_relay_url(relay_url.clone());
        }
        if !endpoint_addr.is_empty() {
            self.node
                .discovery()
                .add_endpoint_info(endpoint_addr.clone());
        }
        self.learned_peers
            .lock()
            .await
            .insert(endpoint_addr.id.to_string(), endpoint_addr);
        Ok(())
    }
}

#[async_trait]
impl BlobService for MemoryBlobService {
    async fn put_blob(&self, data: Vec<u8>, mime: &str) -> Result<StoredBlob> {
        let hash = BlobHash::new(blake3::hash(&data).to_hex().to_string());
        self.blobs
            .write()
            .await
            .insert(hash.as_str().to_string(), data.clone());
        Ok(StoredBlob {
            hash,
            mime: mime.to_string(),
            bytes: data.len() as u64,
        })
    }

    async fn fetch_blob(&self, hash: &BlobHash) -> Result<Option<Vec<u8>>> {
        Ok(self.blobs.read().await.get(hash.as_str()).cloned())
    }

    async fn pin_blob(&self, hash: &BlobHash) -> Result<()> {
        self.pinned.write().await.insert(hash.as_str().to_string());
        Ok(())
    }

    async fn blob_status(&self, hash: &BlobHash) -> Result<BlobStatus> {
        if self.pinned.read().await.contains(hash.as_str()) {
            return Ok(BlobStatus::Pinned);
        }
        Ok(match self.fetch_blob(hash).await? {
            Some(_) => BlobStatus::Available,
            None => BlobStatus::Missing,
        })
    }

    async fn import_peer_ticket(&self, _ticket: &str) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl BlobService for IrohBlobService {
    async fn put_blob(&self, data: Vec<u8>, mime: &str) -> Result<StoredBlob> {
        let byte_len = data.len() as u64;
        let temp_tag = self.node.blobs().blobs().add_bytes(data).await?;
        Ok(StoredBlob {
            hash: BlobHash::new(temp_tag.hash.to_string()),
            mime: mime.to_string(),
            bytes: byte_len,
        })
    }

    async fn fetch_blob(&self, hash: &BlobHash) -> Result<Option<Vec<u8>>> {
        let hash_text = hash.as_str().to_string();
        let hash = iroh_blobs::Hash::from_str(hash.as_str())?;
        match self.node.blobs().blobs().get_bytes(hash).await {
            Ok(bytes) => Ok(Some(bytes.to_vec())),
            Err(error) => {
                let peers = self.fetch_peers().await;
                info!(
                    hash = %hash_text,
                    error = %error,
                    configured_peer_count = peers.len(),
                    "blob fetch local miss, trying remote peers"
                );
                for imported_peer in peers {
                    let candidates = self.connect_candidates(&imported_peer).await;
                    info!(
                        hash = %hash_text,
                        peer_id = %imported_peer.id,
                        imported_addrs = ?imported_peer.addrs,
                        candidate_count = candidates.len(),
                        "blob fetch prepared remote peer candidates"
                    );
                    for peer in candidates {
                        match self
                            .node
                            .endpoint()
                            .connect(peer.clone(), iroh_blobs::ALPN)
                            .await
                        {
                            Ok(conn) => {
                                info!(
                                    hash = %hash_text,
                                    peer_id = %peer.id,
                                    addrs = ?peer.addrs,
                                    "blob fetch connected to remote peer"
                                );
                                match self.node.blobs().remote().fetch(conn, hash).await {
                                    Ok(_) => {
                                        info!(
                                            hash = %hash_text,
                                            peer_id = %peer.id,
                                            "blob fetch remote transfer completed"
                                        );
                                    }
                                    Err(error) => {
                                        warn!(
                                            hash = %hash_text,
                                            peer_id = %peer.id,
                                            addrs = ?peer.addrs,
                                            error = %error,
                                            "blob fetch remote transfer failed"
                                        );
                                        continue;
                                    }
                                }
                                match self.node.blobs().blobs().get_bytes(hash).await {
                                    Ok(bytes) => return Ok(Some(bytes.to_vec())),
                                    Err(error) => {
                                        warn!(
                                            hash = %hash_text,
                                            peer_id = %peer.id,
                                            error = %error,
                                            "blob fetch transfer completed but blob still missing locally"
                                        );
                                    }
                                }
                            }
                            Err(error) => {
                                warn!(
                                    hash = %hash_text,
                                    peer_id = %peer.id,
                                    addrs = ?peer.addrs,
                                    error = %error,
                                    "blob fetch connect failed"
                                );
                            }
                        }
                    }
                }
                warn!(hash = %hash_text, "blob fetch exhausted remote peers without success");
                Ok(None)
            }
        }
    }

    async fn pin_blob(&self, hash: &BlobHash) -> Result<()> {
        self.pinned.write().await.insert(hash.as_str().to_string());
        Ok(())
    }

    async fn blob_status(&self, hash: &BlobHash) -> Result<BlobStatus> {
        if self.pinned.read().await.contains(hash.as_str()) {
            return Ok(BlobStatus::Pinned);
        }
        Ok(match self.fetch_blob(hash).await? {
            Some(_) => BlobStatus::Available,
            None => BlobStatus::Missing,
        })
    }

    async fn import_peer_ticket(&self, ticket: &str) -> Result<()> {
        let endpoint_addr = parse_endpoint_ticket(ticket)?;
        self.node
            .discovery()
            .add_endpoint_info(endpoint_addr.clone());
        self.imported_peers
            .lock()
            .await
            .insert(endpoint_addr.id.to_string(), endpoint_addr);
        Ok(())
    }

    async fn learn_peer(&self, endpoint_id: &str) -> Result<()> {
        self.record_learned_peer(endpoint_id).await
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
        Ok(())
    }

    async fn assist_peer_ids(&self) -> Result<Vec<String>> {
        Ok(self.available_fetch_peer_ids().await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    use iroh::Endpoint;
    use kukuri_transport::{TransportNetworkConfig, encode_endpoint_ticket};
    use tempfile::tempdir;
    use tokio::time::{Duration, sleep, timeout};

    fn loopback_ticket(endpoint: &Endpoint, config: &TransportNetworkConfig) -> String {
        let endpoint_addr = endpoint.addr();
        let bound_sockets = endpoint.bound_sockets();
        let ticket_config = TransportNetworkConfig {
            bind_addr: config.bind_addr,
            advertised_host: config.advertised_host.clone().or_else(|| {
                bound_sockets
                    .iter()
                    .find(|addr| addr.ip().is_loopback())
                    .or_else(|| {
                        bound_sockets
                            .iter()
                            .find(|addr| is_ticket_host_candidate(**addr))
                    })
                    .map(|addr| addr.ip().to_string())
            }),
            advertised_port: config.advertised_port.or_else(|| {
                bound_sockets
                    .iter()
                    .find(|addr| addr.port() != 0)
                    .map(|addr| addr.port())
            }),
        };
        encode_endpoint_ticket(&endpoint_addr, &ticket_config).expect("sender ticket")
    }

    fn is_ticket_host_candidate(addr: SocketAddr) -> bool {
        !addr.ip().is_unspecified()
    }

    #[tokio::test]
    async fn blob_roundtrip_basic() {
        let node = IrohDocsNode::memory().await.expect("memory node");
        let blobs = IrohBlobService::new(node);
        let stored = blobs
            .put_blob(b"hello blob".to_vec(), "text/plain")
            .await
            .expect("put blob");

        let payload = blobs
            .fetch_blob(&stored.hash)
            .await
            .expect("fetch blob")
            .expect("blob bytes");
        assert_eq!(payload, b"hello blob".to_vec());

        assert_eq!(
            blobs.blob_status(&stored.hash).await.expect("blob status"),
            BlobStatus::Available
        );
        blobs.pin_blob(&stored.hash).await.expect("pin blob");
        assert_eq!(
            blobs.blob_status(&stored.hash).await.expect("blob status"),
            BlobStatus::Pinned
        );
    }

    #[tokio::test]
    async fn remote_fetch_roundtrip_after_ticket_import() {
        let sender_dir = tempdir().expect("sender tempdir");
        let receiver_dir = tempdir().expect("receiver tempdir");
        let config = TransportNetworkConfig::loopback();

        let sender_node = IrohDocsNode::persistent_with_config(sender_dir.path(), config.clone())
            .await
            .expect("sender node");
        let receiver_node =
            IrohDocsNode::persistent_with_config(receiver_dir.path(), config.clone())
                .await
                .expect("receiver node");

        let sender = IrohBlobService::new(sender_node.clone());
        let receiver = IrohBlobService::new(receiver_node);

        let ticket = loopback_ticket(sender_node.endpoint(), &config);
        receiver
            .import_peer_ticket(&ticket)
            .await
            .expect("import ticket");

        let stored = sender
            .put_blob(b"video-remote-roundtrip".to_vec(), "video/mp4")
            .await
            .expect("put blob");

        let payload = receiver.fetch_blob(&stored.hash).await.expect("fetch blob");

        assert_eq!(payload, Some(b"video-remote-roundtrip".to_vec()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_fetch_uses_learned_remote_info_when_imported_ticket_is_stale() {
        let sender_dir = tempdir().expect("sender tempdir");
        let receiver_dir = tempdir().expect("receiver tempdir");
        let config = TransportNetworkConfig::loopback();

        let sender_node = IrohDocsNode::persistent_with_config(sender_dir.path(), config.clone())
            .await
            .expect("sender node");
        let receiver_node =
            IrohDocsNode::persistent_with_config(receiver_dir.path(), config.clone())
                .await
                .expect("receiver node");

        let sender = IrohBlobService::new(sender_node.clone());
        let receiver = IrohBlobService::new(receiver_node.clone());

        let stale_sender_ticket = format!("{}@127.0.0.1:1", sender_node.endpoint().addr().id);
        receiver
            .import_peer_ticket(&stale_sender_ticket)
            .await
            .expect("import stale sender ticket");

        let receiver_addr = receiver_node.endpoint().addr();
        let connection = sender_node
            .endpoint()
            .connect(receiver_addr, iroh_blobs::ALPN)
            .await
            .expect("seed incoming sender connection");
        drop(connection);

        timeout(Duration::from_secs(5), async {
            loop {
                if receiver_node
                    .endpoint()
                    .remote_info(sender_node.endpoint().addr().id)
                    .await
                    .is_some()
                {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("receiver should learn sender remote info");

        let stored = sender
            .put_blob(b"stale-ticket-fallback".to_vec(), "video/mp4")
            .await
            .expect("put blob");

        let payload = receiver.fetch_blob(&stored.hash).await.expect("fetch blob");

        assert_eq!(payload, Some(b"stale-ticket-fallback".to_vec()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn connect_candidates_prefers_relay_hint_before_learned_remote_info() {
        let sender_dir = tempdir().expect("sender tempdir");
        let receiver_dir = tempdir().expect("receiver tempdir");
        let config = TransportNetworkConfig::loopback();

        let sender_node = IrohDocsNode::persistent_with_config(sender_dir.path(), config.clone())
            .await
            .expect("sender node");
        let receiver_node = IrohDocsNode::persistent_with_config(receiver_dir.path(), config)
            .await
            .expect("receiver node");

        let receiver = IrohBlobService::new(receiver_node.clone());
        let relay_url = "https://relay.example.invalid/".parse().expect("relay url");
        let sender_addr =
            iroh::EndpointAddr::new(sender_node.endpoint().id()).with_relay_url(relay_url);

        let seeded = sender_node
            .endpoint()
            .connect(receiver_node.endpoint().addr(), iroh_blobs::ALPN)
            .await
            .expect("seed connection");
        drop(seeded);

        timeout(Duration::from_secs(5), async {
            loop {
                if receiver_node
                    .endpoint()
                    .remote_info(sender_node.endpoint().id())
                    .await
                    .is_some()
                {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("receiver should learn sender remote info");

        let candidates = receiver.connect_candidates(&sender_addr).await;
        assert!(!candidates.is_empty());
        assert_eq!(candidates[0], sender_addr);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn learn_peer_snapshots_remote_info_addrs_for_future_blob_fetches() {
        let sender_dir = tempdir().expect("sender tempdir");
        let receiver_dir = tempdir().expect("receiver tempdir");
        let config = TransportNetworkConfig::loopback();

        let sender_node = IrohDocsNode::persistent_with_config(sender_dir.path(), config.clone())
            .await
            .expect("sender node");
        let receiver_node = IrohDocsNode::persistent_with_config(receiver_dir.path(), config)
            .await
            .expect("receiver node");

        let sender = IrohBlobService::new(sender_node.clone());
        let receiver = IrohBlobService::new(receiver_node.clone());

        let seeded = sender_node
            .endpoint()
            .connect(receiver_node.endpoint().addr(), iroh_blobs::ALPN)
            .await
            .expect("seed connection");
        drop(seeded);

        timeout(Duration::from_secs(5), async {
            loop {
                if receiver_node
                    .endpoint()
                    .remote_info(sender_node.endpoint().id())
                    .await
                    .is_some()
                {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("receiver should learn sender remote info");

        receiver
            .learn_peer(&sender_node.endpoint().id().to_string())
            .await
            .expect("learn sender peer");

        let learned = receiver.fetch_peers().await;
        assert!(
            learned
                .iter()
                .find(|peer| peer.id == sender_node.endpoint().id())
                .is_some_and(|peer| !peer.is_empty()),
            "learned peer should retain usable address information"
        );

        let stored = sender
            .put_blob(b"learned-peer-fetch".to_vec(), "image/png")
            .await
            .expect("put blob");

        let payload = receiver.fetch_blob(&stored.hash).await.expect("fetch blob");
        assert_eq!(payload, Some(b"learned-peer-fetch".to_vec()));
    }
}
