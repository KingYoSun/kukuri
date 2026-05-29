use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use crate::IrohDocsNode;

fn recovery_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = fs::read_dir(root)
        .expect("read root")
        .filter_map(|entry| {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            if path.is_dir() && name.starts_with("iroh-docs-recovery-") {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    dirs.sort();
    dirs
}

#[tokio::test]
async fn persistent_node_recovers_corrupt_docs_store() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    fs::write(root.join("docs.redb"), b"not a redb database").expect("write corrupt docs");
    fs::write(
        root.join("default-author"),
        "535f0f7a2bf4128ddf726b6dbd5d5eae9a7b9e05440344e63b7568b68ef8abd3",
    )
    .expect("write stale author");
    fs::write(
        root.join("endpoint-secret.json"),
        "{\"secret_key\":[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1]}",
    )
    .expect("write endpoint secret");

    let node = IrohDocsNode::persistent(root)
        .await
        .expect("node should recover corrupt docs store");
    node.shutdown().await.expect("shutdown");

    let dirs = recovery_dirs(root);
    assert_eq!(dirs.len(), 1);
    assert!(dirs[0].join("docs.redb").exists());
    assert!(dirs[0].join("default-author").exists());
    assert!(root.join("docs.redb").exists());
    assert!(root.join("default-author").exists());
    assert!(root.join("endpoint-secret.json").exists());
}

#[tokio::test]
async fn persistent_node_recovers_stale_default_author_without_docs_store() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    fs::write(
        root.join("default-author"),
        "535f0f7a2bf4128ddf726b6dbd5d5eae9a7b9e05440344e63b7568b68ef8abd3",
    )
    .expect("write stale author");

    let node = IrohDocsNode::persistent(root)
        .await
        .expect("node should recover stale default author");
    node.shutdown().await.expect("shutdown");

    let dirs = recovery_dirs(root);
    assert_eq!(dirs.len(), 1);
    assert!(dirs[0].join("docs.redb").exists());
    assert!(dirs[0].join("default-author").exists());
    assert!(root.join("docs.redb").exists());
    assert!(root.join("default-author").exists());
}

#[tokio::test]
async fn persistent_node_does_not_recover_healthy_store() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let node = IrohDocsNode::persistent(root)
        .await
        .expect("initial healthy node");
    node.shutdown().await.expect("initial shutdown");

    let restarted = IrohDocsNode::persistent(root)
        .await
        .expect("healthy node restart");
    restarted.shutdown().await.expect("restart shutdown");

    assert!(recovery_dirs(root).is_empty());
    assert!(root.join("docs.redb").exists());
    assert!(root.join("default-author").exists());
}
