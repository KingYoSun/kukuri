use crate::infrastructure::p2p::gossip_service::GossipService;
use crate::infrastructure::p2p::iroh_gossip_service::IrohGossipService;
use crate::modules::p2p::P2PEvent;
use iroh::{Endpoint, NodeAddr};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use tokio::time::{Duration, sleep, timeout};

use super::logging::log_step;

pub(crate) const DEFAULT_JOIN_TIMEOUT: Duration = Duration::from_secs(15);
pub(crate) const DEFAULT_EVENT_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Debug)]
pub(crate) struct BootstrapContext {
    pub(crate) hints: Vec<String>,
    pub(crate) node_addrs: Vec<NodeAddr>,
}

pub(crate) async fn create_service(ctx: &BootstrapContext) -> IrohGossipService {
    let bind_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0);
    log_step!(
        "binding endpoint on {} and enabling DHT discovery (bootstrap hints: {})",
        bind_addr,
        ctx.hints.join(", ")
    );
    let endpoint = Arc::new(
        Endpoint::builder()
            .discovery_dht()
            .bind_addr_v4(bind_addr)
            .bind()
            .await
            .expect("failed to bind iroh endpoint"),
    );
    endpoint.online().await;
    for addr in &ctx.node_addrs {
        log_step!("adding bootstrap node addr {}", addr.node_id);
        let _ = endpoint.add_node_addr_with_source(addr.clone(), "integration-bootstrap");
        match endpoint.connect(addr.clone(), iroh_gossip::ALPN).await {
            Ok(_) => log_step!("connected to bootstrap {}", addr.node_id),
            Err(err) => log_step!("failed to connect to bootstrap {}: {:?}", addr.node_id, err),
        }
    }
    log_step!("endpoint ready, building gossip service");
    sleep(Duration::from_millis(200)).await;
    IrohGossipService::new(endpoint).expect("failed to create gossip service")
}

pub(crate) fn build_peer_hints(
    base: &[String],
    local_hints: &[Option<String>],
    self_idx: usize,
) -> Vec<String> {
    let mut result = base.to_vec();
    for (idx, hint) in local_hints.iter().enumerate() {
        if idx == self_idx {
            continue;
        }
        if let Some(h) = hint {
            if !result.contains(h) {
                result.push(h.clone());
            }
        }
    }
    result
}

pub(crate) async fn wait_for_topic_membership(
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

pub(crate) async fn wait_for_peer_join_event(
    receivers: &mut [&mut tokio::sync::mpsc::UnboundedReceiver<P2PEvent>],
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
            if let Ok(Some(evt)) =
                timeout(Duration::from_millis(150), async { rx.recv().await }).await
            {
                if matches!(evt, P2PEvent::PeerJoined { .. }) {
                    log_step!("received PeerJoined event after {:?}", start.elapsed());
                    return true;
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
