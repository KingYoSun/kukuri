pub mod error;
pub mod events;
pub mod message;
pub mod topic_mesh;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use error::{P2PError, Result};
pub use events::P2PEvent;
#[allow(unused_imports)]
pub use message::{GLOBAL_TOPIC, GossipMessage, MessageType, generate_topic_id, user_topic_id};
#[allow(unused_imports)]
pub use topic_mesh::{TopicMesh, TopicStats};
