use crate::domain::p2p::P2PEvent;
use crate::infrastructure::p2p::iroh_gossip_service::IrohGossipService;
use crate::infrastructure::p2p::{DiscoveryOptions, gossip_service::GossipService};
use iroh::{Endpoint, EndpointAddr, RelayMode, address_lookup::MemoryLookup};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{Duration, sleep, timeout};

use super::logging::log_step;

pub const DEFAULT_JOIN_TIMEOUT: Duration = Duration::from_secs(15);
pub const DEFAULT_EVENT_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Debug)]
pub struct BootstrapContext {
    pub hints: Vec<String>,
    pub node_addrs: Vec<EndpointAddr>,
}

pub async fn create_service(ctx: &BootstrapContext) -> IrohGossipService {
    let bind_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0);
    log_step!(
        "binding endpoint on {} and enabling DHT discovery (bootstrap hints: {})",
        bind_addr,
        ctx.hints.join(", ")
    );
    let static_discovery = Arc::new(MemoryLookup::new());
    let builder = DiscoveryOptions::default()
        .apply_to_builder(Endpoint::empty_builder(RelayMode::Default))
        .address_lookup(static_discovery.clone())
        .bind_addr(bind_addr)
        .expect("failed to configure bind addr");
    let endpoint = Arc::new(builder.bind().await.expect("failed to bind iroh endpoint"));
    endpoint.online().await;
    for addr in &ctx.node_addrs {
        log_step!("adding bootstrap node addr {}", addr.id);
        static_discovery.add_endpoint_info(addr.clone());
        match endpoint.connect(addr.clone(), iroh_gossip::ALPN).await {
            Ok(_) => log_step!("connected to bootstrap {}", addr.id),
            Err(err) => log_step!("failed to connect to bootstrap {}: {:?}", addr.id, err),
        }
    }
    log_step!("endpoint ready, building gossip service");
    sleep(Duration::from_millis(200)).await;
    IrohGossipService::new(endpoint, static_discovery).expect("failed to create gossip service")
}

pub fn build_peer_hints(
    base: &[String],
    local_hints: &[Option<String>],
    self_idx: usize,
) -> Vec<String> {
    let mut result = base.to_vec();
    for (idx, hint) in local_hints.iter().enumerate() {
        if idx == self_idx {
            continue;
        }
        if let Some(h) = hint
            && !result.contains(h)
        {
            result.push(h.clone());
        }
    }
    result
}

pub async fn wait_for_topic_membership(
    service: &IrohGossipService,
    topic: &str,
    timeout_duration: Duration,
) -> bool {
    let target = topic.to_string();
    let start = tokio::time::Instant::now();
    while start.elapsed() < timeout_duration {
        log_step!(
            "checking joined topics for {} (elapsed {:?}/{:?})",
            topic,
            start.elapsed(),
            timeout_duration
        );
        if let Ok(joined) = service.get_joined_topics().await {
            log_step!("currently joined topics: {:?}", joined);
            if joined.iter().any(|t| t == &target) {
                return true;
            }
        }
        sleep(Duration::from_millis(100)).await;
    }
    false
}

pub async fn wait_for_peer_join_event(
    receivers: &mut [&mut broadcast::Receiver<P2PEvent>],
    max_wait: Duration,
) -> bool {
    log_step!(
        "waiting up to {:?} for peer join events across {} receivers",
        max_wait,
        receivers.len()
    );
    let start = tokio::time::Instant::now();
    while start.elapsed() < max_wait {
        for rx in receivers.iter_mut() {
            if let Ok(recv_result) =
                timeout(Duration::from_millis(150), async { rx.recv().await }).await
            {
                match recv_result {
                    Ok(P2PEvent::PeerJoined { .. }) => {
                        log_step!("received PeerJoined event after {:?}", start.elapsed());
                        return true;
                    }
                    Ok(_) => {}
                    Err(err) => {
                        log_step!("peer join receiver error: {:?}", err);
                    }
                }
            }
        }
    }
    log_step!(
        "timed out waiting for peer join events after {:?}",
        max_wait
    );
    false
}
