pub mod error;
pub mod message;
pub mod gossip_manager;
pub mod topic_mesh;
pub mod event_sync;
pub mod hybrid_distributor;
pub mod peer_discovery;
pub mod commands;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use error::{P2PError, Result};
#[allow(unused_imports)]
pub use message::{GossipMessage, MessageType, generate_topic_id, GLOBAL_TOPIC, user_topic_id};
pub use gossip_manager::{GossipManager, P2PEvent};
#[allow(unused_imports)]
pub use topic_mesh::{TopicMesh, TopicStats};
pub use event_sync::EventSync;
#[allow(unused_imports)]
pub use hybrid_distributor::{HybridDistributor, HybridConfig, DeliveryPriority, DeliveryStrategy, DeliveryResult};
#[allow(unused_imports)]
pub use peer_discovery::PeerDiscovery;
#[allow(unused_imports)]
pub use commands::*;
