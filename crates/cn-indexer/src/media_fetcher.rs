//! `MediaFetcher` の本番実装（#609）。
//!
//! `media_hint`（blob hash）から scan 用に blob を**一時 fetch** する。取得は
//! `BlobService::fetch_blob_ephemeral` 経由で、remote から取得した bytes をローカルストアへ
//! 残さない（community node の no-permanent-blob-storage 前提。ADR 0028 §2.1 / ADR 0025 §2.3）。
//!
//! fail-closed の写像:
//! - hint が blob hash（64 hex）でない → `ScanError::Protocol`
//! - fetch 全体の timeout 超過 → `ScanError::Timeout`
//! - blob 未取得（未複製 / peer 不在 / cooldown）→ `ScanError::Unavailable`
//! - サイズ上限超過 → `ScanError::Protocol`
//! - content type 不明（hint 無し + magic bytes 判定不能）→ `ScanError::Protocol`
//!
//! いずれも呼び出し側（orchestrator / policy router）で hold へ写像され、allow に落ちない。

use std::sync::Arc;

use async_trait::async_trait;

use kukuri_blob_service::BlobService;
use kukuri_cn_safety::provider::{FetchedMedia, MediaFetcher, ScanError};
use kukuri_core::BlobHash;

use crate::config::MediaFetchConfig;

/// blob store からの一時 fetch で `MediaFetcher` を実装する。
pub struct BlobMediaFetcher {
    blob_service: Arc<dyn BlobService>,
    config: MediaFetchConfig,
}

impl BlobMediaFetcher {
    pub fn new(blob_service: Arc<dyn BlobService>, config: MediaFetchConfig) -> Self {
        Self {
            blob_service,
            config,
        }
    }
}

impl std::fmt::Debug for BlobMediaFetcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlobMediaFetcher")
            .field("config", &self.config)
            .finish()
    }
}

#[async_trait]
impl MediaFetcher for BlobMediaFetcher {
    async fn fetch(
        &self,
        media_hint: &str,
        content_type_hint: Option<&str>,
    ) -> Result<FetchedMedia, ScanError> {
        let hint = media_hint.trim();
        if !is_blob_hash(hint) {
            return Err(ScanError::Protocol(format!(
                "media hint is not a blob hash (expected 64 hex chars, got {} chars)",
                hint.len()
            )));
        }

        let hash = BlobHash::new(hint.to_string());
        let fetched = tokio::time::timeout(
            self.config.timeout,
            self.blob_service.fetch_blob_ephemeral(&hash),
        )
        .await
        .map_err(|_| {
            ScanError::Timeout(format!(
                "media fetch timed out after {}s",
                self.config.timeout.as_secs()
            ))
        })?
        .map_err(|error| ScanError::Unavailable(format!("media fetch failed: {error:#}")))?;

        let Some(bytes) = fetched else {
            return Err(ScanError::Unavailable(
                "referenced media blob is not retrievable yet (not replicated or no reachable \
                 peers); scan stays fail-closed"
                    .to_string(),
            ));
        };

        if bytes.len() as u64 > self.config.max_bytes {
            return Err(ScanError::Protocol(format!(
                "referenced media exceeds the scan size limit ({} > {} bytes)",
                bytes.len(),
                self.config.max_bytes
            )));
        }

        let content_type = match normalized_content_type(content_type_hint) {
            Some(mime) => mime,
            None => sniff_content_type(&bytes).ok_or_else(|| {
                ScanError::Protocol(
                    "media content type is unknown (no mime metadata and unrecognized magic \
                     bytes); refusing to scan blindly (fail-closed)"
                        .to_string(),
                )
            })?,
        };

        Ok(FetchedMedia {
            bytes,
            content_type,
        })
    }
}

/// blob hash（blake3 の 64 hex 表現）か。
fn is_blob_hash(hint: &str) -> bool {
    hint.len() == 64 && hint.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// 空白のみ / 空の hint は「無し」として扱う。
fn normalized_content_type(hint: Option<&str>) -> Option<String> {
    hint.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// 主要 media 形式の magic bytes から content type を推定する（mime metadata 欠落時の fallback）。
///
/// 判定できない形式は None（呼び出し側で fail-closed）。依存を増やさないため自前判定に留める。
fn sniff_content_type(bytes: &[u8]) -> Option<String> {
    const RIFF: &[u8] = b"RIFF";
    let mime = if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        "image/png"
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        "image/gif"
    } else if bytes.len() >= 12 && bytes.starts_with(RIFF) && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        "video/mp4"
    } else if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        "video/webm"
    } else {
        return None;
    };
    Some(mime.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    use kukuri_blob_service::MemoryBlobService;

    const MISSING_HASH: &str = "1111111111111111111111111111111111111111111111111111111111111111";

    fn fetcher_with(
        blob_service: Arc<dyn BlobService>,
        config: MediaFetchConfig,
    ) -> BlobMediaFetcher {
        BlobMediaFetcher::new(blob_service, config)
    }

    fn default_config() -> MediaFetchConfig {
        MediaFetchConfig::default()
    }

    const TINY_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
    ];

    #[tokio::test]
    async fn fetch_returns_bytes_with_the_hinted_content_type() {
        let blobs = Arc::new(MemoryBlobService::default());
        let stored = blobs
            .put_blob(TINY_PNG.to_vec(), "image/png")
            .await
            .expect("put blob");

        let fetcher = fetcher_with(blobs, default_config());
        let media = fetcher
            .fetch(stored.hash.as_str(), Some("image/png"))
            .await
            .expect("fetch succeeds");
        assert_eq!(media.bytes, TINY_PNG.to_vec());
        assert_eq!(media.content_type, "image/png");
    }

    #[tokio::test]
    async fn fetch_falls_back_to_magic_byte_sniffing_without_a_mime_hint() {
        let blobs = Arc::new(MemoryBlobService::default());
        let stored = blobs
            .put_blob(TINY_PNG.to_vec(), "application/octet-stream")
            .await
            .expect("put blob");

        let fetcher = fetcher_with(blobs, default_config());
        let media = fetcher
            .fetch(stored.hash.as_str(), None)
            .await
            .expect("fetch succeeds");
        assert_eq!(media.content_type, "image/png");
    }

    #[tokio::test]
    async fn unknown_content_type_is_protocol_error() {
        let blobs = Arc::new(MemoryBlobService::default());
        let stored = blobs
            .put_blob(b"plain text, no magic".to_vec(), "text/plain")
            .await
            .expect("put blob");

        let fetcher = fetcher_with(blobs, default_config());
        let error = fetcher
            .fetch(stored.hash.as_str(), None)
            .await
            .expect_err("unknown content type must fail closed");
        assert!(matches!(error, ScanError::Protocol(_)), "{error}");
    }

    #[tokio::test]
    async fn non_hash_hint_is_protocol_error() {
        let fetcher = fetcher_with(Arc::new(MemoryBlobService::default()), default_config());
        let error = fetcher
            .fetch("media-1751234567890-abcd1234", Some("image/png"))
            .await
            .expect_err("manifest ids must be expanded before reaching the fetcher");
        assert!(matches!(error, ScanError::Protocol(_)), "{error}");
    }

    #[tokio::test]
    async fn missing_blob_is_unavailable() {
        let fetcher = fetcher_with(Arc::new(MemoryBlobService::default()), default_config());
        let error = fetcher
            .fetch(MISSING_HASH, Some("image/png"))
            .await
            .expect_err("missing blob must fail closed");
        assert!(matches!(error, ScanError::Unavailable(_)), "{error}");
    }

    #[tokio::test]
    async fn oversized_blob_is_protocol_error() {
        let blobs = Arc::new(MemoryBlobService::default());
        let stored = blobs
            .put_blob(TINY_PNG.to_vec(), "image/png")
            .await
            .expect("put blob");

        let config = MediaFetchConfig {
            max_bytes: 4,
            ..MediaFetchConfig::default()
        };
        let fetcher = fetcher_with(blobs, config);
        let error = fetcher
            .fetch(stored.hash.as_str(), Some("image/png"))
            .await
            .expect_err("oversized blob must fail closed");
        assert!(matches!(error, ScanError::Protocol(_)), "{error}");
    }

    #[tokio::test]
    async fn slow_fetch_is_timeout_error() {
        /// fetch が返らない fake（timeout 検証用）。
        struct StallingBlobService;

        #[async_trait]
        impl BlobService for StallingBlobService {
            async fn put_blob(
                &self,
                _data: Vec<u8>,
                _mime: &str,
            ) -> anyhow::Result<kukuri_blob_service::StoredBlob> {
                unreachable!("not used")
            }

            async fn fetch_blob(&self, _hash: &BlobHash) -> anyhow::Result<Option<Vec<u8>>> {
                std::future::pending().await
            }

            async fn pin_blob(&self, _hash: &BlobHash) -> anyhow::Result<()> {
                Ok(())
            }

            async fn blob_status(
                &self,
                _hash: &BlobHash,
            ) -> anyhow::Result<kukuri_blob_service::BlobStatus> {
                Ok(kukuri_blob_service::BlobStatus::Missing)
            }

            async fn import_peer_ticket(&self, _ticket: &str) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let config = MediaFetchConfig {
            timeout: std::time::Duration::from_millis(50),
            ..MediaFetchConfig::default()
        };
        let fetcher = fetcher_with(Arc::new(StallingBlobService), config);
        let error = fetcher
            .fetch(MISSING_HASH, Some("image/png"))
            .await
            .expect_err("stalled fetch must fail closed");
        assert!(matches!(error, ScanError::Timeout(_)), "{error}");
    }
}
