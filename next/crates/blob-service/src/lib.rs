use std::collections::{BTreeMap, HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use next_core::BlobHash;
use next_docs_sync::IrohDocsNode;
use next_transport::parse_endpoint_ticket;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

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
        let hash = iroh_blobs::Hash::from_str(hash.as_str())?;
        match self.node.blobs().blobs().get_bytes(hash).await {
            Ok(bytes) => Ok(Some(bytes.to_vec())),
            Err(_) => {
                let peers = self
                    .imported_peers
                    .lock()
                    .await
                    .values()
                    .cloned()
                    .collect::<Vec<_>>();
                for peer in peers {
                    if let Ok(conn) = self.node.endpoint().connect(peer, iroh_blobs::ALPN).await {
                        let _ = self.node.blobs().remote().fetch(conn, hash).await;
                        if let Ok(bytes) = self.node.blobs().blobs().get_bytes(hash).await {
                            return Ok(Some(bytes.to_vec()));
                        }
                    }
                }
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
}
