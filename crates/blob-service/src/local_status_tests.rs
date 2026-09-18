//! #1152: 表示用の状態確認（`local_blob_status`）が remote の blob を取得・永続化しないことを、
//! 実 Iroh 2 ノードで固定する。

use std::str::FromStr;

use kukuri_iroh_node::IrohDocsNode;
use kukuri_transport::TransportNetworkConfig;
use tempfile::tempdir;

use crate::tests::loopback_ticket;
use crate::{BlobService, BlobStatus, IrohBlobService};

#[tokio::test]
async fn local_blob_status_does_not_fetch_or_persist_remote_blob() {
    // #1152: 表示用の状態確認はローカルの有無だけを返し、remote peer が持つ blob を
    // 取得・永続化しない(`blob_status` は remote 取得で確かめるため挙動が異なる)。
    let sender_dir = tempdir().expect("sender tempdir");
    let receiver_dir = tempdir().expect("receiver tempdir");
    let config = TransportNetworkConfig::loopback();

    let sender_node = IrohDocsNode::persistent_with_config(sender_dir.path(), config.clone())
        .await
        .expect("sender node");
    let receiver_node = IrohDocsNode::persistent_with_config(receiver_dir.path(), config.clone())
        .await
        .expect("receiver node");

    let sender = IrohBlobService::new(sender_node.clone());
    let receiver = IrohBlobService::new(receiver_node.clone());
    let ticket = loopback_ticket(sender_node.endpoint(), &config);
    receiver
        .import_peer_ticket(&ticket)
        .await
        .expect("import ticket");

    let stored = sender
        .put_blob(b"remote-only-attachment".to_vec(), "image/png")
        .await
        .expect("put blob");
    let hash = iroh_blobs::Hash::from_str(stored.hash.as_str()).expect("hash");

    assert_eq!(
        receiver
            .local_blob_status(&stored.hash)
            .await
            .expect("receiver local status"),
        BlobStatus::Missing
    );
    assert!(
        receiver_node.blobs().blobs().get_bytes(hash).await.is_err(),
        "local status check must not persist the remote blob"
    );

    assert_eq!(
        sender
            .local_blob_status(&stored.hash)
            .await
            .expect("sender local status"),
        BlobStatus::Available
    );
    sender.pin_blob(&stored.hash).await.expect("pin blob");
    assert_eq!(
        sender
            .local_blob_status(&stored.hash)
            .await
            .expect("pinned local status"),
        BlobStatus::Pinned
    );

    // 取得を伴う `blob_status` とは区別される: remote から取得した後はローカルに在る。
    assert_eq!(
        receiver
            .blob_status(&stored.hash)
            .await
            .expect("receiver fetching status"),
        BlobStatus::Available
    );
    assert_eq!(
        receiver
            .local_blob_status(&stored.hash)
            .await
            .expect("receiver local status after fetch"),
        BlobStatus::Available
    );
}
