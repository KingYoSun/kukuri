use iroh_docs::NamespaceSecret;
use kukuri_core::{ReplicaId, TopicId, blob_hash};

pub(crate) fn public_replica_secret(replica_id: &ReplicaId) -> Option<NamespaceSecret> {
    if replica_id.as_str().starts_with("channel::") {
        return None;
    }
    let digest = blake3::hash(format!("kukuri-docs:{}", replica_id.as_str()).as_bytes());
    Some(NamespaceSecret::from_bytes(digest.as_bytes()))
}

pub fn topic_replica_id(topic_id: &str) -> ReplicaId {
    ReplicaId::new(format!("topic::{topic_id}"))
}

pub fn private_channel_replica_id(channel_id: &str) -> ReplicaId {
    ReplicaId::new(format!("channel::{channel_id}"))
}

pub fn private_channel_epoch_replica_id(channel_id: &str, epoch_id: &str) -> ReplicaId {
    ReplicaId::new(format!("channel::{channel_id}::epoch::{epoch_id}"))
}

pub fn private_channel_hint_topic(channel_id: &str) -> TopicId {
    TopicId::new(format!("private/{channel_id}"))
}

pub fn author_replica_id(author_pubkey: &str) -> ReplicaId {
    ReplicaId::new(format!("author::{author_pubkey}"))
}

pub fn device_replica_id(author_pubkey: &str, device_id: &str) -> ReplicaId {
    ReplicaId::new(format!("device::{author_pubkey}::{device_id}"))
}

pub fn stable_key(prefix: &str, key: &str) -> String {
    format!("{prefix}/{key}")
}

pub fn value_hash(value: impl AsRef<[u8]>) -> String {
    blob_hash(value).0
}
