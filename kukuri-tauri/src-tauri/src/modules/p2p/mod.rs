pub mod error;
pub mod message;
pub mod topic_mesh;
pub mod events;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use error::{P2PError, Result};
pub use events::P2PEvent;
#[allow(unused_imports)]
pub use message::{generate_topic_id, user_topic_id, GossipMessage, MessageType, GLOBAL_TOPIC};
#[allow(unused_imports)]
pub use topic_mesh::{TopicMesh, TopicStats};

