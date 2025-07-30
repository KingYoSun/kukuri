pub mod commands;
pub mod error;
pub mod event_sync;
pub mod gossip_manager;
pub mod hybrid_distributor;
pub mod message;
pub mod peer_discovery;
pub mod topic_mesh;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use commands::*;
#[allow(unused_imports)]
pub use error::{P2PError, Result};
pub use event_sync::EventSync;
pub use gossip_manager::{GossipManager, P2PEvent};
#[allow(unused_imports)]
pub use hybrid_distributor::{
    DeliveryPriority, DeliveryResult, DeliveryStrategy, HybridConfig, HybridDistributor,
};
#[allow(unused_imports)]
pub use message::{generate_topic_id, user_topic_id, GossipMessage, MessageType, GLOBAL_TOPIC};
#[allow(unused_imports)]
pub use peer_discovery::PeerDiscovery;
#[allow(unused_imports)]
pub use topic_mesh::{TopicMesh, TopicStats};
