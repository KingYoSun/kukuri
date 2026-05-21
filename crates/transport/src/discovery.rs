use std::sync::Arc;
#[cfg(test)]
use std::sync::RwLock as StdRwLock;
use std::time::Duration;

use anyhow::Result;
use iroh::Endpoint;
use iroh::address_lookup::MemoryLookup;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::debug;

use crate::config::{ConnectMode, DhtDiscoveryOptions, TransportRelayConfig};

const RELAY_ONLINE_STARTUP_TIMEOUT: Duration = Duration::from_secs(15);

pub async fn prepare_endpoint_for_discovery(
    endpoint: &Endpoint,
    discovery: &Arc<MemoryLookup>,
    dht_options: &DhtDiscoveryOptions,
    relay_config: &TransportRelayConfig,
) -> Result<Option<JoinHandle<()>>> {
    let relay_backed = relay_config.connect_mode() == ConnectMode::DirectOrRelay;
    if relay_backed {
        let endpoint = endpoint.clone();
        let discovery = Arc::clone(discovery);
        let online_task = tokio::spawn(async move {
            endpoint.online().await;
            discovery.add_endpoint_info(endpoint.addr());
        });
        match timeout(RELAY_ONLINE_STARTUP_TIMEOUT, async {
            let _ = online_task.await;
        })
        .await
        {
            Ok(()) => {}
            Err(error) => {
                debug!(
                    error = %error,
                    "timed out waiting for relay-backed endpoint to come online; continuing startup in background"
                );
            }
        }
    }
    discovery.add_endpoint_info(endpoint.addr());

    let _ = dht_options;
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures_util::StreamExt;
    use iroh::SecretKey;
    use iroh::address_lookup::{AddrFilter, AddressLookup};
    use iroh_mainline_address_lookup::DhtAddressLookup;
    use n0_mainline::{DhtBuilder, Testnet};

    use crate::iroh::bind_endpoint_with_options;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transport_relay_backed_dht_lookup_publishes_relay_info() {
        let testnet = Testnet::new(5).await.expect("testnet");
        let (_relay_map, relay_url, _guard) = iroh::test_utils::run_relay_server()
            .await
            .expect("relay server");
        let relay_config = TransportRelayConfig {
            iroh_relay_urls: vec![relay_url.to_string()],
        }
        .normalized();
        let secret_key = SecretKey::from_bytes(&[7u8; 32]);
        let relay_urls = Arc::new(StdRwLock::new(
            relay_config.parsed_relay_urls().expect("relay urls"),
        ));
        let (endpoint, _discovery, _publish_task) = bind_endpoint_with_options(
            std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::LOCALHOST,
                0,
            )),
            &DhtDiscoveryOptions::with_bootstrap(&testnet.bootstrap),
            &relay_config,
            relay_urls,
            Some(secret_key.clone()),
        )
        .await
        .expect("bind endpoint");

        let mut dht_builder = DhtBuilder::default();
        dht_builder.bootstrap(&testnet.bootstrap);
        let lookup = DhtAddressLookup::builder()
            .dht_builder(dht_builder)
            .no_publish()
            .addr_filter(AddrFilter::unfiltered())
            .build()
            .expect("dht lookup");
        timeout(Duration::from_secs(30), async {
            loop {
                if let Some(mut resolved) = lookup.resolve(endpoint.id())
                    && let Some(Ok(item)) = resolved.next().await
                    && item
                        .endpoint_info()
                        .relay_urls()
                        .any(|candidate| candidate == &relay_url)
                {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("endpoint relay info never published to DHT lookup");
    }
}
