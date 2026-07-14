use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use kukuri_blob_service::{BlobService, BlobStatus, IrohBlobService, StoredBlob};
use kukuri_core::{BlobHash, GossipHint, ReplicaId, TopicId};
use kukuri_docs_sync::{
    DocEventStream, DocFetchPolicy, DocOp, DocQuery, DocRecord, DocsSync, IrohDocsSync,
};
use kukuri_iroh_node::IrohDocsNode;
use kukuri_transport::{
    ConnectMode, DhtDiscoveryOptions, DiscoveryMode, DiscoverySnapshot, HintStream, HintTransport,
    IrohGossipTransport, PeerSnapshot, SeedPeer, Transport, TransportNetworkConfig,
    TransportRelayConfig,
};
use tokio::sync::{Mutex, RwLock};
use tracing::info;

use crate::discovery::{DiscoveryConfig, normalize_seed_peers};

pub(crate) struct BoundIrohStack {
    pub(crate) node: Arc<IrohDocsNode>,
    pub(crate) transport: Arc<IrohGossipTransport>,
    pub(crate) docs_sync: Arc<IrohDocsSync>,
    pub(crate) blob_service: Arc<IrohBlobService>,
}

/// ホットスワップ可能なサービスラッパーを 1 つ生成する。
///
/// `SharedIrohStack::rebuild` は iroh ノードを作り直すたびに transport / docs-sync /
/// blob-service の実体を差し替える。各ラッパーは `Arc<RwLock<Arc<T>>>` を保持し、
/// trait メソッドはすべて「その時点の実体を `current()` で取り出して素通し」する純転送で、
/// 3 種で構造もメソッド転送も同型だったため declarative macro で 1 箇所に集約する。
///
/// 文法: `struct <Name> wrapping <Inner>;` に続けて、実装する trait ごとに
/// `#[async_trait] impl <Trait> { async fn <m>(<args>) -> <ret>; ... }` を並べる
/// (メソッドは `&self` を暗黙に取り、`self.current().await.<m>(args).await` へ転送される)。
macro_rules! reloadable_service {
    (
        $(#[$struct_meta:meta])*
        $vis:vis struct $name:ident wrapping $inner:ty;
        $(
            $(#[$impl_meta:meta])*
            impl $trait:path {
                $(
                    async fn $method:ident ( $( $arg:ident : $arg_ty:ty ),* $(,)? ) -> $ret:ty;
                )*
            }
        )*
    ) => {
        #[derive(Clone)]
        $(#[$struct_meta])*
        $vis struct $name {
            inner: Arc<RwLock<Arc<$inner>>>,
        }

        impl $name {
            pub(crate) fn new(inner: Arc<$inner>) -> Self {
                Self {
                    inner: Arc::new(RwLock::new(inner)),
                }
            }

            pub(crate) async fn current(&self) -> Arc<$inner> {
                self.inner.read().await.clone()
            }

            pub(crate) async fn replace(&self, inner: Arc<$inner>) {
                *self.inner.write().await = inner;
            }
        }

        $(
            $(#[$impl_meta])*
            impl $trait for $name {
                $(
                    async fn $method(&self, $( $arg : $arg_ty ),* ) -> $ret {
                        self.current().await.$method($( $arg ),*).await
                    }
                )*
            }
        )*
    };
}

reloadable_service! {
    pub(crate) struct ReloadableTransport wrapping IrohGossipTransport;

    #[async_trait]
    impl Transport {
        async fn peers() -> Result<PeerSnapshot>;
        async fn export_ticket() -> Result<Option<String>>;
        async fn import_ticket(ticket: &str) -> Result<()>;
        async fn configure_discovery(
            mode: DiscoveryMode,
            env_locked: bool,
            configured_seed_peers: Vec<SeedPeer>,
            bootstrap_seed_peers: Vec<SeedPeer>,
        ) -> Result<()>;
        async fn discovery() -> Result<DiscoverySnapshot>;
    }

    #[async_trait]
    impl HintTransport {
        async fn subscribe_hints(topic: &TopicId) -> Result<HintStream>;
        async fn unsubscribe_hints(topic: &TopicId) -> Result<()>;
        async fn publish_hint(topic: &TopicId, hint: GossipHint) -> Result<()>;
    }
}

reloadable_service! {
    pub(crate) struct ReloadableDocsSync wrapping IrohDocsSync;

    #[async_trait]
    impl DocsSync {
        async fn open_replica(replica_id: &ReplicaId) -> Result<()>;
        async fn register_private_replica_secret(
            replica_id: &ReplicaId,
            namespace_secret_hex: &str,
        ) -> Result<()>;
        async fn remove_private_replica_secret(replica_id: &ReplicaId) -> Result<()>;
        async fn apply_doc_op(replica_id: &ReplicaId, op: DocOp) -> Result<()>;
        async fn query_replica_with_policy(
            replica_id: &ReplicaId,
            query: DocQuery,
            policy: DocFetchPolicy,
        ) -> Result<Vec<DocRecord>>;
        async fn subscribe_replica(replica_id: &ReplicaId) -> Result<DocEventStream>;
        async fn import_peer_ticket(ticket: &str) -> Result<()>;
        async fn learn_peer(endpoint_id: &str) -> Result<()>;
        async fn restart_replica_sync(replica_id: &ReplicaId) -> Result<()>;
        async fn set_seed_peers(peers: Vec<SeedPeer>) -> Result<()>;
        async fn assist_peer_ids() -> Result<Vec<String>>;
    }
}

reloadable_service! {
    pub(crate) struct ReloadableBlobService wrapping IrohBlobService;

    #[async_trait]
    impl BlobService {
        async fn put_blob(data: Vec<u8>, mime: &str) -> Result<StoredBlob>;
        async fn fetch_blob(hash: &BlobHash) -> Result<Option<Vec<u8>>>;
        async fn pin_blob(hash: &BlobHash) -> Result<()>;
        async fn blob_status(hash: &BlobHash) -> Result<BlobStatus>;
        async fn import_peer_ticket(ticket: &str) -> Result<()>;
        async fn learn_peer(endpoint_id: &str) -> Result<()>;
        async fn set_seed_peers(peers: Vec<SeedPeer>) -> Result<()>;
        async fn assist_peer_ids() -> Result<Vec<String>>;
    }
}

pub(crate) struct SharedIrohStack {
    pub(crate) current: Mutex<Option<BoundIrohStack>>,
    pub(crate) transport: Arc<ReloadableTransport>,
    pub(crate) docs_sync: Arc<ReloadableDocsSync>,
    pub(crate) blob_service: Arc<ReloadableBlobService>,
    pub(crate) root: PathBuf,
    pub(crate) network_config: TransportNetworkConfig,
    pub(crate) dht_options: DhtDiscoveryOptions,
}

fn should_rebuild_runtime_connectivity(
    current_relay_urls: &[String],
    next_relay_urls: &[String],
) -> bool {
    current_relay_urls != next_relay_urls
}

pub(crate) fn effective_seed_peers(
    discovery_config: &DiscoveryConfig,
    bootstrap_seed_peers: &[SeedPeer],
) -> Vec<SeedPeer> {
    normalize_seed_peers(
        discovery_config
            .seed_peers
            .iter()
            .cloned()
            .chain(bootstrap_seed_peers.iter().cloned())
            .collect(),
    )
}

pub(crate) fn effective_dht_options(
    dht_options: &DhtDiscoveryOptions,
    bootstrap_seed_peers: &[SeedPeer],
    relay_config: &TransportRelayConfig,
) -> DhtDiscoveryOptions {
    if relay_config.connect_mode() == ConnectMode::DirectOrRelay && !bootstrap_seed_peers.is_empty()
    {
        DhtDiscoveryOptions::disabled()
    } else {
        dht_options.clone()
    }
}

impl SharedIrohStack {
    pub(crate) async fn new(
        root: &Path,
        network_config: TransportNetworkConfig,
        discovery_config: &DiscoveryConfig,
        bootstrap_seed_peers: &[SeedPeer],
        dht_options: DhtDiscoveryOptions,
        relay_config: TransportRelayConfig,
    ) -> Result<Self> {
        let dht_options = effective_dht_options(&dht_options, bootstrap_seed_peers, &relay_config);
        let current = BoundIrohStack::new(
            root,
            network_config.clone(),
            discovery_config,
            bootstrap_seed_peers,
            dht_options.clone(),
            relay_config,
        )
        .await?;
        let transport = Arc::new(ReloadableTransport::new(current.transport.clone()));
        let docs_sync = Arc::new(ReloadableDocsSync::new(current.docs_sync.clone()));
        let blob_service = Arc::new(ReloadableBlobService::new(current.blob_service.clone()));
        Ok(Self {
            current: Mutex::new(Some(current)),
            transport,
            docs_sync,
            blob_service,
            root: root.to_path_buf(),
            network_config,
            dht_options,
        })
    }

    pub(crate) async fn rebuild(
        &self,
        discovery_config: &DiscoveryConfig,
        bootstrap_seed_peers: &[SeedPeer],
        relay_config: TransportRelayConfig,
    ) -> Result<()> {
        let relay_config = relay_config.normalized();
        let dht_options =
            effective_dht_options(&self.dht_options, bootstrap_seed_peers, &relay_config);
        let previous = self
            .current
            .lock()
            .await
            .take()
            .context("missing active iroh stack during rebuild")?;
        let transport_peer_state = previous.transport.peer_state().await;
        let docs_peer_state = previous.docs_sync.peer_state().await;
        let blob_peer_state = previous.blob_service.peer_state().await;
        info!(
            relay_url_count = relay_config.iroh_relay_urls.len(),
            discovery_mode = ?discovery_config.mode,
            "rebuilding iroh stack after runtime relay connectivity change"
        );
        previous.shutdown().await;
        let next = BoundIrohStack::new(
            &self.root,
            self.network_config.clone(),
            discovery_config,
            bootstrap_seed_peers,
            dht_options,
            relay_config,
        )
        .await?;
        next.transport
            .restore_peer_state(transport_peer_state)
            .await?;
        next.docs_sync.restore_peer_state(docs_peer_state).await?;
        next.blob_service
            .restore_peer_state(blob_peer_state)
            .await?;
        self.transport.replace(next.transport.clone()).await;
        self.docs_sync.replace(next.docs_sync.clone()).await;
        self.blob_service.replace(next.blob_service.clone()).await;
        *self.current.lock().await = Some(next);
        Ok(())
    }

    pub(crate) async fn apply_runtime_connectivity(
        &self,
        discovery_config: &DiscoveryConfig,
        bootstrap_seed_peers: &[SeedPeer],
        relay_config: TransportRelayConfig,
    ) -> Result<()> {
        let relay_config = relay_config.normalized();
        let next_relay_urls = relay_config
            .parsed_relay_urls()?
            .into_iter()
            .map(|url| url.to_string())
            .collect::<Vec<_>>();
        let current_relay_urls = {
            let current = self.current.lock().await;
            current
                .as_ref()
                .context("missing active iroh stack while reading relay urls")?
                .node
                .relay_urls()
                .await
                .into_iter()
                .map(|url| url.to_string())
                .collect::<Vec<_>>()
        };
        if should_rebuild_runtime_connectivity(&current_relay_urls, &next_relay_urls) {
            info!(
                current_relay_url_count = current_relay_urls.len(),
                next_relay_url_count = next_relay_urls.len(),
                discovery_mode = ?discovery_config.mode,
                "runtime relay connectivity change requires stack rebuild"
            );
            return self
                .rebuild(discovery_config, bootstrap_seed_peers, relay_config)
                .await;
        }
        let current = self.current.lock().await;
        let current = current
            .as_ref()
            .context("missing active iroh stack while applying runtime connectivity")?;
        current
            .node
            .apply_relay_config(relay_config.clone())
            .await?;
        current.transport.update_relay_config(relay_config).await?;
        current
            .transport
            .configure_discovery(
                discovery_config.mode.clone(),
                discovery_config.env_locked,
                discovery_config.seed_peers.clone(),
                bootstrap_seed_peers.to_vec(),
            )
            .await?;
        let effective_seed_peers = effective_seed_peers(discovery_config, bootstrap_seed_peers);
        current
            .docs_sync
            .set_seed_peers(effective_seed_peers.clone())
            .await?;
        current
            .blob_service
            .set_seed_peers(effective_seed_peers)
            .await?;
        Ok(())
    }

    pub(crate) async fn force_rebuild_runtime_connectivity(
        &self,
        discovery_config: &DiscoveryConfig,
        bootstrap_seed_peers: &[SeedPeer],
        relay_config: TransportRelayConfig,
    ) -> Result<()> {
        self.rebuild(discovery_config, bootstrap_seed_peers, relay_config)
            .await
    }

    pub(crate) async fn shutdown(&self) {
        if let Some(current) = self.current.lock().await.take() {
            let _ =
                tokio::time::timeout(std::time::Duration::from_secs(15), current.shutdown()).await;
        }
    }

    #[cfg(test)]
    pub(crate) async fn endpoint(&self) -> iroh::Endpoint {
        self.current
            .lock()
            .await
            .as_ref()
            .expect("missing active iroh stack")
            .node
            .endpoint()
            .clone()
    }
}

impl BoundIrohStack {
    pub(crate) async fn new(
        root: &Path,
        network_config: TransportNetworkConfig,
        discovery_config: &DiscoveryConfig,
        bootstrap_seed_peers: &[SeedPeer],
        dht_options: DhtDiscoveryOptions,
        relay_config: TransportRelayConfig,
    ) -> Result<Self> {
        let relay_config = relay_config.normalized();
        let node = IrohDocsNode::persistent_with_discovery_config(
            root,
            network_config.clone(),
            dht_options,
            relay_config.clone(),
        )
        .await?;
        let transport = Arc::new(IrohGossipTransport::from_shared_parts(
            node.endpoint().clone(),
            node.gossip().clone(),
            node.discovery(),
            network_config,
            relay_config.clone(),
        )?);
        let docs_sync = Arc::new(IrohDocsSync::new(node.clone()));
        let blob_service = Arc::new(IrohBlobService::new(node.clone()));
        transport
            .configure_discovery(
                discovery_config.mode.clone(),
                discovery_config.env_locked,
                discovery_config.seed_peers.clone(),
                bootstrap_seed_peers.to_vec(),
            )
            .await?;
        let effective_seed_peers = effective_seed_peers(discovery_config, bootstrap_seed_peers);
        docs_sync
            .set_seed_peers(effective_seed_peers.clone())
            .await?;
        blob_service.set_seed_peers(effective_seed_peers).await?;
        Ok(Self {
            node,
            transport,
            docs_sync,
            blob_service,
        })
    }

    pub(crate) async fn shutdown(&self) {
        self.transport.shutdown().await;
        self.docs_sync.shutdown().await;
        let _ = self.node.clone().shutdown().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kukuri_blob_service::BlobService;
    use kukuri_docs_sync::DocsSync;
    use kukuri_transport::Transport;
    use tempfile::tempdir;
    use tokio::time::{Duration, timeout};

    #[test]
    fn runtime_connectivity_rebuild_helper_skips_rebuild_when_relay_urls_are_unchanged() {
        let relay_url = "https://relay.example.com".to_string();
        assert!(!should_rebuild_runtime_connectivity(
            std::slice::from_ref(&relay_url),
            std::slice::from_ref(&relay_url),
        ));
    }

    #[test]
    fn runtime_connectivity_rebuild_helper_rebuilds_for_static_peer_relay_change() {
        let current = "https://relay-a.example.com".to_string();
        let next = "https://relay-b.example.com".to_string();
        assert!(should_rebuild_runtime_connectivity(
            std::slice::from_ref(&current),
            std::slice::from_ref(&next),
        ));
    }

    #[test]
    fn runtime_connectivity_rebuild_helper_rebuilds_for_non_static_peer_relay_change() {
        let current = "https://relay-a.example.com".to_string();
        let next = "https://relay-b.example.com".to_string();
        assert!(should_rebuild_runtime_connectivity(
            std::slice::from_ref(&current),
            std::slice::from_ref(&next),
        ));
    }

    #[tokio::test]
    async fn runtime_connectivity_rebuild_preserves_manual_ticket_peers() {
        let (_relay_map, relay_url, _guard) = iroh::test_utils::run_relay_server()
            .await
            .expect("relay server");
        let dir = tempdir().expect("tempdir");
        let discovery_config = DiscoveryConfig::static_peer_default();
        let stack_a = SharedIrohStack::new(
            &dir.path().join("stack-a"),
            TransportNetworkConfig::loopback(),
            &discovery_config,
            &[],
            DhtDiscoveryOptions::disabled(),
            TransportRelayConfig::default(),
        )
        .await
        .expect("stack a");
        let stack_b = SharedIrohStack::new(
            &dir.path().join("stack-b"),
            TransportNetworkConfig::loopback(),
            &discovery_config,
            &[],
            DhtDiscoveryOptions::disabled(),
            TransportRelayConfig::default(),
        )
        .await
        .expect("stack b");

        let ticket_b = stack_b
            .transport
            .current()
            .await
            .export_ticket()
            .await
            .expect("export ticket b")
            .expect("ticket b value");
        stack_a
            .transport
            .current()
            .await
            .import_ticket(ticket_b.as_str())
            .await
            .expect("import transport ticket");
        stack_a
            .docs_sync
            .current()
            .await
            .import_peer_ticket(ticket_b.as_str())
            .await
            .expect("import docs ticket");
        stack_a
            .blob_service
            .current()
            .await
            .import_peer_ticket(ticket_b.as_str())
            .await
            .expect("import blob ticket");

        let current_guard = stack_a.current.lock().await;
        let current = current_guard
            .as_ref()
            .expect("current stack before rebuild");
        let transport_before = current.transport.peer_state().await;
        let docs_before = current.docs_sync.peer_state().await;
        let blob_before = current.blob_service.peer_state().await;
        drop(current_guard);

        timeout(
            Duration::from_secs(30),
            stack_a.rebuild(
                &discovery_config,
                &[],
                TransportRelayConfig {
                    iroh_relay_urls: vec![relay_url.to_string()],
                },
            ),
        )
        .await
        .expect("stack rebuild timeout")
        .expect("stack rebuild");

        let current_guard = stack_a.current.lock().await;
        let current = current_guard.as_ref().expect("current stack after rebuild");
        let transport_after = current.transport.peer_state().await;
        let docs_after = current.docs_sync.peer_state().await;
        let blob_after = current.blob_service.peer_state().await;
        drop(current_guard);

        assert_eq!(
            transport_after.imported_peers,
            transport_before.imported_peers
        );
        assert_eq!(docs_after.imported_peers, docs_before.imported_peers);
        assert_eq!(blob_after.imported_peers, blob_before.imported_peers);

        timeout(Duration::from_secs(30), stack_a.shutdown())
            .await
            .expect("stack a shutdown timeout");
        timeout(Duration::from_secs(30), stack_b.shutdown())
            .await
            .expect("stack b shutdown timeout");
    }

    #[tokio::test]
    async fn shared_stack_initializes_with_configured_relay_on_first_bind() {
        let (_relay_map, relay_url, _guard) = iroh::test_utils::run_relay_server()
            .await
            .expect("relay server");
        let dir = tempdir().expect("tempdir");
        let discovery_config = DiscoveryConfig::static_peer_default();
        let relay_config = TransportRelayConfig {
            iroh_relay_urls: vec![relay_url.to_string()],
        };
        let stack = SharedIrohStack::new(
            &dir.path().join("stack-relay"),
            TransportNetworkConfig::loopback(),
            &discovery_config,
            &[],
            DhtDiscoveryOptions::disabled(),
            relay_config,
        )
        .await
        .expect("stack");

        let current_guard = stack.current.lock().await;
        let current = current_guard.as_ref().expect("current stack");
        assert_eq!(current.node.relay_urls().await, vec![relay_url.clone()]);
        assert_eq!(
            current
                .transport
                .discovery()
                .await
                .expect("discovery")
                .connect_mode,
            ConnectMode::DirectOrRelay
        );
        drop(current_guard);

        timeout(Duration::from_secs(30), stack.shutdown())
            .await
            .expect("stack shutdown timeout");
    }
}
