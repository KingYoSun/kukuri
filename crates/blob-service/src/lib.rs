use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use kukuri_core::BlobHash;
use kukuri_iroh_node::{IrohDocsNode, remote_fetch};
use kukuri_transport::{PeerAddrBook, RemoteFetchRetryState, SeedPeer, parse_endpoint_ticket};
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
    /// scan 用の一時取得（#609）: remote から取得した bytes を**ローカルストアへ残さない**。
    ///
    /// ローカルに既在の blob はそのまま読む（別目的で存在するものは消さない）。既定実装は
    /// `fetch_blob` に委譲する（in-memory 実装等、恒久保存の概念が無い実装向け）。
    async fn fetch_blob_ephemeral(&self, hash: &BlobHash) -> Result<Option<Vec<u8>>> {
        self.fetch_blob(hash).await
    }
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
    // ピア台帳・接続候補・リトライ状態は kukuri-transport の共通実装(WP-H2)。
    // 台帳変化の bool は blob-service では使わない(レプリカへの配り直しが無いため)。
    peers: Arc<PeerAddrBook>,
    remote_fetch_retries: Arc<Mutex<RemoteFetchRetryState>>,
}

#[derive(Clone, Debug, Default)]
pub struct BlobPeerState {
    pub learned_peers: Vec<iroh::EndpointAddr>,
    pub imported_peers: Vec<iroh::EndpointAddr>,
}

#[derive(Clone, Default)]
pub struct MemoryBlobService {
    blobs: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    pinned: Arc<RwLock<HashSet<String>>>,
}

impl IrohBlobService {
    pub fn new(node: Arc<IrohDocsNode>) -> Self {
        let peers = Arc::new(PeerAddrBook::new(node.endpoint().clone(), node.discovery()));
        Self {
            node,
            pinned: Arc::new(RwLock::new(HashSet::new())),
            peers,
            remote_fetch_retries: Arc::new(Mutex::new(RemoteFetchRetryState::default())),
        }
    }

    // 本体ループは remote_fetch(WP-B14)へ移設。characterization テスト専用に残す。
    #[cfg(test)]
    async fn connect_candidates(
        &self,
        imported_peer: &iroh::EndpointAddr,
    ) -> Vec<iroh::EndpointAddr> {
        self.peers.connect_candidates(imported_peer).await
    }

    #[cfg(test)]
    async fn fetch_peers(&self) -> Vec<iroh::EndpointAddr> {
        self.peers.merged_peers().await
    }

    pub async fn peer_state(&self) -> BlobPeerState {
        BlobPeerState {
            learned_peers: self.peers.learned_peers_snapshot().await,
            imported_peers: self.peers.imported_peers_snapshot().await,
        }
    }

    pub async fn restore_peer_state(&self, state: BlobPeerState) -> Result<()> {
        for endpoint_addr in state.learned_peers {
            let _ = self.peers.insert_learned_peer_addr(endpoint_addr).await;
        }
        for endpoint_addr in state.imported_peers {
            self.peers.insert_imported_peer_addr(endpoint_addr).await;
        }
        Ok(())
    }

    async fn available_fetch_peer_ids(&self) -> Vec<String> {
        self.peers.available_peer_ids().await
    }

    async fn record_learned_peer(&self, endpoint_id: &str) -> Result<()> {
        let relay_urls = self.node.relay_urls().await;
        // 台帳変化の bool は使わない(docs-sync と違い配り直す対象が無い)。
        let _ = self
            .peers
            .record_learned_peer(endpoint_id, &relay_urls)
            .await?;
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
                // ループ本体(cooldown ゲート込み)は iroh-node の共通実装(WP-B14)。
                remote_fetch::fetch_bytes_with_cooldown(
                    &self.node,
                    &self.peers,
                    &self.remote_fetch_retries,
                    "blob",
                    hash_text.as_str(),
                    hash,
                    error,
                )
                .await
            }
        }
    }

    async fn fetch_blob_ephemeral(&self, hash: &BlobHash) -> Result<Option<Vec<u8>>> {
        let hash_text = hash.as_str().to_string();
        let hash = iroh_blobs::Hash::from_str(hash.as_str())?;
        match self.node.blobs().blobs().get_bytes(hash).await {
            Ok(bytes) => Ok(Some(bytes.to_vec())),
            Err(error) => {
                // remote からの取得はストアへ書き込まない(safety scan の一時 fetch。#609)。
                remote_fetch::fetch_bytes_ephemeral_with_cooldown(
                    &self.node,
                    &self.peers,
                    &self.remote_fetch_retries,
                    "blob",
                    hash_text.as_str(),
                    hash,
                    error,
                )
                .await
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
        self.peers.insert_imported_peer_addr(endpoint_addr).await;
        Ok(())
    }

    async fn learn_peer(&self, endpoint_id: &str) -> Result<()> {
        self.record_learned_peer(endpoint_id).await
    }

    async fn set_seed_peers(&self, peers: Vec<SeedPeer>) -> Result<()> {
        let relay_urls = self.node.relay_urls().await;
        self.peers.set_seed_peers(peers, &relay_urls).await
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

    // リトライ状態のテストは共通実装側(kukuri-transport::peers)へ移動した(WP-H2)。

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

    #[tokio::test]
    async fn ephemeral_fetch_returns_bytes_without_persisting_them_locally() {
        // safety scan の一時 fetch(#609): remote から取得できること、かつ取得後も
        // ローカルストアに blob が残らないこと(no-permanent-blob-storage)を固定する。
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

        let ticket = loopback_ticket(sender_node.endpoint(), &config);
        receiver
            .import_peer_ticket(&ticket)
            .await
            .expect("import ticket");

        let stored = sender
            .put_blob(b"scan-only-media".to_vec(), "image/png")
            .await
            .expect("put blob");

        let payload = receiver
            .fetch_blob_ephemeral(&stored.hash)
            .await
            .expect("ephemeral fetch");
        assert_eq!(payload, Some(b"scan-only-media".to_vec()));

        // ローカルストアには取り込まれていない(fetch_blob 経由だと remote fetch に
        // フォールバックしてしまうため、store を直接確認する)。
        let hash = iroh_blobs::Hash::from_str(stored.hash.as_str()).expect("hash");
        assert!(
            receiver_node.blobs().blobs().get_bytes(hash).await.is_err(),
            "ephemeral fetch must not persist the blob into the local store"
        );
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
    async fn connect_candidates_prefers_direct_remote_info_before_relay_hint() {
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
        assert_ne!(candidates[0], sender_addr);
        assert!(candidates[0].relay_urls().next().is_none());
        assert_eq!(candidates.last(), Some(&sender_addr));
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
