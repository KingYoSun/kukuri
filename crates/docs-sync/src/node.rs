use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock as StdRwLock};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use iroh::address_lookup::MemoryLookup;
use iroh::protocol::Router;
use iroh::{Endpoint, EndpointAddr, RelayUrl, Watcher};
use iroh_blobs::api::Store as BlobStore;
use iroh_blobs::store::{fs::options::Options as BlobStoreOptions, mem::MemStore};
use iroh_docs::api::DocsApi;
use iroh_gossip::net::Gossip;
use kukuri_transport::{
    DhtDiscoveryOptions, TransportNetworkConfig, TransportRelayConfig, build_endpoint_builder,
    prepare_endpoint_for_discovery, sync_endpoint_relay_config,
};
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::warn;

#[cfg(test)]
use iroh::tls::CaRootsConfig;

const ENDPOINT_SECRET_FILE_NAME: &str = "endpoint-secret.json";

fn relay_activation_timeout() -> Duration {
    Duration::from_secs(10)
}

pub struct IrohDocsNode {
    endpoint: Endpoint,
    gossip: Gossip,
    discovery: Arc<MemoryLookup>,
    relay_urls: Arc<StdRwLock<Vec<RelayUrl>>>,
    router: Arc<Router>,
    docs: DocsApi,
    blobs: BlobStore,
    endpoint_publish_task: Option<JoinHandle<()>>,
    shutdown_started: AtomicBool,
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
        let relay_urls = Arc::new(StdRwLock::new(relay_config.parsed_relay_urls()?));
        let mut endpoint_builder = build_endpoint_builder(
            Endpoint::empty_builder().relay_mode(relay_config.relay_mode()?),
            &discovery,
            Some(&dht_options),
            Arc::clone(&relay_urls),
        )?;
        #[cfg(test)]
        {
            endpoint_builder =
                endpoint_builder.ca_roots_config(CaRootsConfig::insecure_skip_verify());
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
            relay_urls,
            router: Arc::new(router),
            docs: docs.api().clone(),
            blobs,
            endpoint_publish_task,
            shutdown_started: AtomicBool::new(false),
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
        self.relay_urls
            .read()
            .expect("docs sync relay urls poisoned")
            .clone()
    }

    pub async fn apply_relay_config(&self, relay_config: TransportRelayConfig) -> Result<()> {
        let relay_config = relay_config.normalized();
        let next_relay_urls = relay_config.parsed_relay_urls()?;
        let current_relay_urls = self
            .relay_urls
            .read()
            .expect("docs sync relay urls poisoned")
            .clone();
        sync_endpoint_relay_config(&self.endpoint, &current_relay_urls, &next_relay_urls).await?;
        if !next_relay_urls.is_empty() {
            if current_relay_urls.is_empty() {
                let endpoint = self.endpoint.clone();
                tokio::spawn(async move {
                    endpoint.online().await;
                });
            }
            let mut addr_watcher = self.endpoint.watch_addr();
            let expected_relays = next_relay_urls.iter().cloned().collect::<BTreeSet<_>>();
            let relay_ready = |addr: &EndpointAddr| {
                addr.relay_urls()
                    .any(|relay_url| expected_relays.contains(relay_url))
            };
            if !relay_ready(&addr_watcher.get()) {
                match timeout(relay_activation_timeout(), async move {
                    let mut stream = addr_watcher.stream();
                    while let Some(addr) = stream.next().await {
                        if relay_ready(&addr) {
                            return Ok::<(), anyhow::Error>(());
                        }
                    }
                    bail!("relay address watcher ended before any configured relay became active")
                })
                .await
                {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        warn!(
                            relay_urls = ?next_relay_urls,
                            error = %error,
                            "configured relay did not become active during initial wait"
                        );
                    }
                    Err(error) => {
                        warn!(
                            relay_urls = ?next_relay_urls,
                            error = %error,
                            "timed out waiting for live relay connectivity; continuing with configured relay"
                        );
                    }
                }
            }
        }
        *self
            .relay_urls
            .write()
            .expect("docs sync relay urls poisoned") = next_relay_urls;
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
        if self.shutdown_started.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
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
        if self.shutdown_started.swap(true, Ordering::AcqRel) {
            return;
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
