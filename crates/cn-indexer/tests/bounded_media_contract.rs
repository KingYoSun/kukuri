use async_trait::async_trait;
use kukuri_blob_service::{BlobService, BlobStatus, StoredBlob};
use kukuri_cn_indexer::{config::MediaFetchConfig, media_fetcher::BlobMediaFetcher};
use kukuri_cn_safety::MediaFetcher;
use kukuri_core::BlobHash;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Default)]
struct ObservedBoundedBlob {
    unbounded: AtomicUsize,
    bounded: AtomicUsize,
}
#[async_trait]
impl BlobService for ObservedBoundedBlob {
    async fn put_blob(&self, _: Vec<u8>, _: &str) -> anyhow::Result<StoredBlob> {
        anyhow::bail!("no durable writes")
    }
    async fn fetch_blob(&self, _: &BlobHash) -> anyhow::Result<Option<Vec<u8>>> {
        self.unbounded.fetch_add(1, Ordering::SeqCst);
        Ok(Some(vec![0xff, 0xd8, 0xff, 0xe0]))
    }
    async fn fetch_blob_ephemeral_bounded(
        &self,
        _: &BlobHash,
        max: u64,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        assert_eq!(max, 1024);
        self.bounded.fetch_add(1, Ordering::SeqCst);
        Ok(Some(vec![0xff, 0xd8, 0xff, 0xe0]))
    }
    async fn pin_blob(&self, _: &BlobHash) -> anyhow::Result<()> {
        anyhow::bail!("no pins")
    }
    async fn blob_status(&self, _: &BlobHash) -> anyhow::Result<BlobStatus> {
        Ok(BlobStatus::Missing)
    }
    async fn import_peer_ticket(&self, _: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn media_fetch_bounds_ingress_before_allocating_whole_blob() {
    let blob = Arc::new(ObservedBoundedBlob::default());
    let fetcher = BlobMediaFetcher::new(
        blob.clone(),
        MediaFetchConfig {
            max_bytes: 1024,
            ..Default::default()
        },
    );
    fetcher
        .fetch(&"a".repeat(64), Some("image/jpeg"))
        .await
        .expect("media");
    assert_eq!(
        blob.unbounded.load(Ordering::SeqCst),
        0,
        "scan must never call the unbounded/durable fallback"
    );
    assert_eq!(blob.bounded.load(Ordering::SeqCst), 1);
}
