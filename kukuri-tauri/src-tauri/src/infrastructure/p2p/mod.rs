pub mod network_service;
pub mod gossip_service;
pub mod event_distributor;
pub mod iroh_network_service;
pub mod iroh_gossip_service;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DistributionStrategy {
    Hybrid,
    Nostr,
    P2P,
}

pub use network_service::{NetworkService, NetworkStats, Peer};
pub use gossip_service::GossipService;
pub use event_distributor::EventDistributor;
pub use iroh_network_service::IrohNetworkService;
pub use iroh_gossip_service::IrohGossipService;