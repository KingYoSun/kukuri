use std::collections::{BTreeMap, HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use kukuri_core::BlobHash;
use kukuri_docs_sync::IrohDocsNode;
use kukuri_transport::parse_endpoint_ticket;
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
}

#[derive(Clone)]
pub struct IrohBlobService {
    node: Arc<IrohDocsNode>,
    pinned: Arc<RwLock<HashSet<String>>>,
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
            imported_peers: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    async fn connect_candidates(
        &self,
        imported_peer: &iroh::EndpointAddr,
    ) -> Vec<iroh::EndpointAddr> {
        let mut candidates = Vec::new();
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
                let peers = self
                    .imported_peers
                    .lock()
                    .await
                    .values()
                    .cloned()
                    .collect::<Vec<_>>();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use kukuri_transport::{TransportNetworkConfig, encode_endpoint_ticket};
    use tempfile::tempdir;
    use tokio::time::{Duration, sleep, timeout};

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

        let ticket =
            encode_endpoint_ticket(&sender_node.endpoint().addr(), &config).expect("sender ticket");
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
}
