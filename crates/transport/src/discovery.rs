use std::sync::Arc;
#[cfg(test)]
use std::sync::RwLock as StdRwLock;
use std::time::Duration;

use anyhow::{Context, Result};
#[cfg(test)]
use iroh::RelayUrl;
#[cfg(test)]
use iroh::TransportAddr;
use iroh::address_lookup::{EndpointInfo, MemoryLookup};
use iroh::{Endpoint, SecretKey};
use pkarr::Client as PkarrClient;
use pkarr::{SignedPacket, Timestamp};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tracing::{debug, warn};

use crate::config::{ConnectMode, DhtDiscoveryOptions, TransportRelayConfig};

const IROH_TXT_NAME: &str = "_iroh";
const DHT_PUBLISH_TTL_SECONDS: u32 = 30;
const DHT_PUBLISH_RETRY_INTERVAL: Duration = Duration::from_secs(2);
const DHT_PUBLISH_REPUBLISH_INTERVAL: Duration = Duration::from_secs(30);
const DHT_PUBLISH_STARTUP_TIMEOUT: Duration = Duration::from_secs(6);

pub async fn prepare_endpoint_for_discovery(
    endpoint: &Endpoint,
    discovery: &Arc<MemoryLookup>,
    dht_options: &DhtDiscoveryOptions,
    relay_config: &TransportRelayConfig,
) -> Result<Option<JoinHandle<()>>> {
    let relay_backed = relay_config.connect_mode() == ConnectMode::DirectOrRelay;
    if relay_backed {
        endpoint.online().await;
    }
    discovery.add_endpoint_info(endpoint.addr());

    let Some(client) = dht_options.publish_client()? else {
        return Ok(None);
    };

    match timeout(DHT_PUBLISH_STARTUP_TIMEOUT, async {
        loop {
            match publish_endpoint_addr_once(endpoint, &client).await {
                Ok(true) => return Ok::<(), anyhow::Error>(()),
                Ok(false) => sleep(DHT_PUBLISH_RETRY_INTERVAL).await,
                Err(error) => {
                    debug!("initial endpoint publish retrying: {error:#}");
                    sleep(DHT_PUBLISH_RETRY_INTERVAL).await;
                }
            }
        }
    })
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            if relay_backed {
                debug!(
                    "initial endpoint publication failed; continuing with relay-only startup: {error:#}"
                );
            } else {
                warn!("initial endpoint publication failed: {error:#}");
            }
        }
        Err(_) => {
            if relay_backed {
                debug!(
                    "initial endpoint publication timed out; continuing with relay-only startup"
                );
            } else {
                warn!("initial endpoint publication timed out; continuing with background retries");
            }
        }
    }

    let endpoint = endpoint.clone();
    let task = tokio::spawn(async move {
        loop {
            let delay = match publish_endpoint_addr_once(&endpoint, &client).await {
                Ok(true) => DHT_PUBLISH_REPUBLISH_INTERVAL,
                Ok(false) => DHT_PUBLISH_RETRY_INTERVAL,
                Err(error) => {
                    if relay_backed {
                        debug!(
                            "failed to publish endpoint address to pkarr; relay path remains available: {error:#}"
                        );
                    } else {
                        warn!("failed to publish endpoint address to pkarr: {error:#}");
                    }
                    DHT_PUBLISH_RETRY_INTERVAL
                }
            };
            sleep(delay).await;
        }
    });
    Ok(Some(task))
}

async fn publish_endpoint_addr_once(endpoint: &Endpoint, client: &PkarrClient) -> Result<bool> {
    let endpoint_addr = endpoint.addr();
    if endpoint_addr.is_empty() {
        return Ok(false);
    }
    let endpoint_info = EndpointInfo::from(endpoint_addr);
    let public_key =
        pkarr::PublicKey::try_from(endpoint.id().as_bytes()).expect("pkarr public key");
    let previous_timestamp = client
        .resolve_most_recent(&public_key)
        .await
        .map(|packet| packet.timestamp());
    let now = Timestamp::now();
    let timestamp = match previous_timestamp {
        Some(previous) if previous >= now => previous + 1,
        _ => now,
    };
    let signed_packet = build_signed_packet_with_timestamp(
        &endpoint_info,
        endpoint.secret_key(),
        DHT_PUBLISH_TTL_SECONDS,
        timestamp,
    )?;
    client
        .publish(&signed_packet, previous_timestamp)
        .await
        .context("pkarr publish failed")?;
    Ok(true)
}

pub(crate) fn build_signed_packet_with_timestamp(
    endpoint_info: &EndpointInfo,
    secret_key: &SecretKey,
    ttl: u32,
    timestamp: Timestamp,
) -> Result<SignedPacket> {
    use pkarr::dns::{self, rdata};

    let keypair = pkarr::Keypair::from_secret_key(&secret_key.to_bytes());
    let mut builder = SignedPacket::builder().timestamp(timestamp);
    let name = dns::Name::new(IROH_TXT_NAME).expect("iroh txt name");
    for entry in endpoint_info.to_txt_strings() {
        let mut txt = rdata::TXT::new();
        txt.add_string(&entry)
            .context("invalid endpoint info txt entry")?;
        builder = builder.txt(name.clone(), txt.into_owned(), ttl);
    }
    builder
        .sign(&keypair)
        .context("failed to sign endpoint info packet")
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::iroh::bind_endpoint_with_options;

    use pkarr::mainline::Testnet;

    fn dht_test_client(testnet: &Testnet) -> PkarrClient {
        let mut builder = PkarrClient::builder();
        builder.no_default_network().bootstrap(&testnet.bootstrap);
        builder.build().expect("pkarr client")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transport_relay_backed_dht_publish_replaces_newer_stale_packet() {
        let testnet = Testnet::new(5).expect("testnet");
        let (_relay_map, relay_url, _guard) = iroh::test_utils::run_relay_server()
            .await
            .expect("relay server");
        let relay_config = TransportRelayConfig {
            iroh_relay_urls: vec![relay_url.to_string()],
        }
        .normalized();
        let secret_key = SecretKey::from_bytes(&[7u8; 32]);
        let client = dht_test_client(&testnet);
        let stale_info = EndpointInfo::from_parts(
            secret_key.public(),
            iroh::address_lookup::EndpointData::new([TransportAddr::Relay(
                "https://stale-relay.invalid"
                    .parse::<RelayUrl>()
                    .expect("stale relay url"),
            )]),
        );
        let stale_packet = build_signed_packet_with_timestamp(
            &stale_info,
            &secret_key,
            30,
            Timestamp::now() + 300_000_000,
        )
        .expect("build stale packet");
        client
            .publish(&stale_packet, None)
            .await
            .expect("publish stale packet");

        let relay_urls = Arc::new(StdRwLock::new(
            relay_config.parsed_relay_urls().expect("relay urls"),
        ));
        let (endpoint, _discovery, _publish_task) = bind_endpoint_with_options(
            std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::LOCALHOST,
                0,
            )),
            &DhtDiscoveryOptions::with_client(client.clone()),
            &relay_config,
            relay_urls,
            Some(secret_key.clone()),
        )
        .await
        .expect("bind endpoint");

        let public_key =
            pkarr::PublicKey::try_from(endpoint.id().as_bytes()).expect("pkarr public key");
        timeout(Duration::from_secs(6), async {
            loop {
                if let Some(packet) = client.resolve_most_recent(&public_key).await
                    && let Ok(info) = EndpointInfo::from_pkarr_signed_packet(&packet)
                    && info.relay_urls().any(|candidate| candidate == &relay_url)
                {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("endpoint relay info never replaced stale packet");
    }
}
