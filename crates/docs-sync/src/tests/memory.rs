use crate::{DocsSync, MemoryDocsSync, private_channel_replica_id};

#[tokio::test]
async fn private_replica_requires_registered_capability() {
    let docs = MemoryDocsSync::default();
    let replica = private_channel_replica_id("private-a");

    let error = docs
        .open_replica(&replica)
        .await
        .expect_err("private replica should require capability");
    assert!(error.to_string().contains("capability"));

    let secret = kukuri_core::KukuriKeys::generate().export_secret_hex();
    docs.register_private_replica_secret(&replica, secret.as_str())
        .await
        .expect("register secret");
    docs.open_replica(&replica)
        .await
        .expect("open replica after registration");
}
