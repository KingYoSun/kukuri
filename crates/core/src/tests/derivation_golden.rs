//! rendezvous / topic id 派生の golden テスト(WP-S3 T4)。
//!
//! rendezvous key はクライアント間で一致して初めて機能する wire 契約
//! (community-node はキーを opaque に扱い再導出しない)。ドメイン文字列・
//! 区切りバイト・ハッシュ構成のいずれの変更もネットワーク分断になる。
//! fail した場合はテストでなく変更側(コード・依存更新)を疑うこと。

use crate::{
    TopicId, author_profile_topic_id, private_topic_rendezvous_key_hex_secret,
    public_topic_rendezvous_key,
};

#[test]
fn public_topic_rendezvous_key_matches_golden() {
    // blake3("kukuri:rendezvous:public-topic:v1" || 0x00 || topic)
    assert_eq!(
        public_topic_rendezvous_key(&TopicId::new("kukuri:topic:golden")),
        "7f7f9902af85354eeadf44b6b953f31d0ec8fe39def5235b1f9a9ff0e1209e7e"
    );
}

#[test]
fn private_topic_rendezvous_key_matches_golden() {
    // HMAC-SHA256(namespace_secret, "kukuri:rendezvous:private-topic:v1" || 0x00 || topic)
    let key = private_topic_rendezvous_key_hex_secret(
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        &TopicId::new("kukuri:topic:golden"),
    )
    .expect("private rendezvous key");
    assert_eq!(
        key,
        "72d34635686d4a3f177b033941879fa4dfc01c3c881abd92fe3eeb18561e63f7"
    );
}

#[test]
fn author_profile_topic_id_matches_golden() {
    assert_eq!(
        author_profile_topic_id("79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798")
            .as_str(),
        "kukuri:topic:profile:79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"
    );
}
