//! replica 命名・namespace 派生の golden テスト(WP-S3 T4)。
//!
//! replica id とその blake3 派生 namespace はネットワーク全体の rendezvous を
//! 決める凍結境界(1 文字の変更で既存ピアと同じ replica を発見できなくなる)。
//! fail した場合はテストでなく変更側を疑うこと。

use kukuri_core::ReplicaId;

use crate::replicas::{
    author_replica_id, device_replica_id, private_channel_epoch_replica_id,
    private_channel_hint_topic, private_channel_replica_id, public_replica_secret, stable_key,
    topic_replica_id,
};

#[test]
fn replica_id_helpers_match_golden() {
    assert_eq!(topic_replica_id("demo").as_str(), "topic::demo");
    assert_eq!(
        private_channel_replica_id("chan-1").as_str(),
        "channel::chan-1"
    );
    assert_eq!(
        private_channel_epoch_replica_id("chan-1", "epoch-2").as_str(),
        "channel::chan-1::epoch::epoch-2"
    );
    assert_eq!(
        private_channel_hint_topic("chan-1").as_str(),
        "private/chan-1"
    );
    assert_eq!(author_replica_id("pubkey-a").as_str(), "author::pubkey-a");
    assert_eq!(
        device_replica_id("pubkey-a", "device-1").as_str(),
        "device::pubkey-a::device-1"
    );
    assert_eq!(stable_key("objects", "obj-1/state"), "objects/obj-1/state");
}

#[test]
fn public_replica_namespace_secret_matches_golden_digest() {
    // blake3("kukuri-docs:" || replica_id) がそのまま NamespaceSecret になる。
    let secret =
        public_replica_secret(&ReplicaId::new("topic::demo")).expect("public replica secret");
    assert_eq!(
        hex::encode(secret.to_bytes()),
        "ebd5f493d3f598727ea4b7b14bb42e5cec7d7b19adef0a3629586be201cddee1"
    );
}

#[test]
fn private_channel_replicas_never_get_public_namespace_secret() {
    assert!(public_replica_secret(&ReplicaId::new("channel::chan-1")).is_none());
    assert!(public_replica_secret(&ReplicaId::new("channel::chan-1::epoch::epoch-2")).is_none());
}
