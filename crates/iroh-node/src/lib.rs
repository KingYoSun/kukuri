//! iroh ノードの所有権を持つ crate(WP-H2)。
//!
//! Endpoint / Router / Gossip / Docs / Blobs / discovery / relay 設定と、
//! endpoint-secret の永続化・ストア破損からの回復を `IrohDocsNode` が所有する。
//! docs-sync / blob-service / transport(部品借用)/ desktop-runtime はここに依存する。
//! かつては docs-sync が置き場所だったが、「docs-sync が基盤の持ち主」という歪みを
//! 解消するため独立させた(挙動不変の移動)。

mod node;
pub mod remote_fetch;

#[cfg(test)]
mod tests;

pub use node::IrohDocsNode;

impl IrohDocsNode {
    pub async fn read_local_blob(&self, hash: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let hash = hash.parse::<iroh_blobs::Hash>()?;
        Ok(self
            .blobs()
            .blobs()
            .get_bytes(hash)
            .await
            .ok()
            .map(|b| b.to_vec()))
    }
}

/// Read an inactive account's existing blob store without creating an endpoint.
/// The caller must hold the account lifecycle guard; never use for an active store.
pub async fn read_offline_blob(
    root: &std::path::Path,
    hash: &str,
) -> anyhow::Result<Option<Vec<u8>>> {
    use iroh_blobs::store::fs::{FsStore, options::Options};
    if !root.join("blobs.db").exists() {
        return Ok(None);
    }
    let hash = hash.parse::<iroh_blobs::Hash>()?;
    let store = FsStore::load_with_opts(root.join("blobs.db"), Options::new(root)).await?;
    let result = store
        .blobs()
        .get_bytes(hash)
        .await
        .ok()
        .map(|bytes| bytes.to_vec());
    store.shutdown().await?;
    Ok(result)
}
