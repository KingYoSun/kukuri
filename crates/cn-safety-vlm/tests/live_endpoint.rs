//! 実機 OpenAI-compatible endpoint に対する end-to-end 検証（#420）。
//!
//! CI では実行しない（`#[ignore]`）。operator が用意した実 endpoint に対して手動で実行する:
//!
//! ```bash
//! COMMUNITY_NODE_VLM_API_BASE_URL=http://<host>:<port> \
//! COMMUNITY_NODE_VLM_MODEL=<model> \
//! COMMUNITY_NODE_VLM_RESPONSE_FORMAT=guard \
//! cargo test -p kukuri-cn-safety-vlm --test live_endpoint -- --ignored --nocapture
//! ```

use std::sync::Arc;

use async_trait::async_trait;

use kukuri_cn_safety::provider::{
    FetchedMedia, MediaFetcher, ProviderScanRequest, ScanError, ScanOutcome, SubjectKind,
};
use kukuri_cn_safety::{SafetyPolicy, SafetyProvider, route};
use kukuri_cn_safety_vlm::{CapabilityProfile, VlmModerationProvider, VlmProviderConfig};

/// 1x1 PNG（無害な単色画像）を返す fetcher。
struct TinyPngFetcher;

#[async_trait]
impl MediaFetcher for TinyPngFetcher {
    async fn fetch(&self, _media_hint: &str) -> Result<FetchedMedia, ScanError> {
        const TINY_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
        use base64::Engine as _;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(TINY_PNG_BASE64)
            .map_err(|e| ScanError::Protocol(e.to_string()))?;
        Ok(FetchedMedia {
            bytes,
            content_type: "image/png".to_string(),
        })
    }
}

fn live_provider() -> VlmModerationProvider {
    let config = VlmProviderConfig::from_env()
        .expect("set COMMUNITY_NODE_VLM_API_BASE_URL / COMMUNITY_NODE_VLM_MODEL to run live tests");
    VlmModerationProvider::new(&config, CapabilityProfile::General)
        .expect("provider construction")
        .with_media_fetcher(Arc::new(TinyPngFetcher))
}

fn permissive_policy() -> SafetyPolicy {
    // 実機検証は general provider 単独で行うため、known CSAM provider 必須の gate を外す。
    SafetyPolicy {
        require_known_csam: false,
        ..SafetyPolicy::public_node_default()
    }
}

#[tokio::test]
#[ignore = "requires a live openai-compatible endpoint (see module doc)"]
async fn live_harmless_text_is_indexable() {
    let provider = live_provider();
    let request = ProviderScanRequest::for_subject(SubjectKind::Post, "live-post-1")
        .with_text("今日は良い天気なので公園を散歩しました。");
    let result = provider.scan(&request).await.expect("scan succeeds");
    println!("harmless text result: {result:?}");
    assert_eq!(result.outcome, ScanOutcome::Completed);
    assert!(!result.known_hash_match);

    let verdict = route(
        std::slice::from_ref(&result),
        &permissive_policy(),
        "live-scan",
    );
    println!("harmless text verdict: {verdict:?}");
    assert!(verdict.is_indexable(), "harmless text must be indexable");
}

#[tokio::test]
#[ignore = "requires a live openai-compatible endpoint (see module doc)"]
async fn live_phishing_text_is_flagged_and_fail_closed() {
    let provider = live_provider();
    let request = ProviderScanRequest::for_subject(SubjectKind::Post, "live-post-2").with_text(
        "CONGRATULATIONS!!! You won $10,000,000! Click http://totally-real-prize.example/claim \
         NOW and enter your bank account number to claim your prize! Limited time only!!!",
    );
    let result = provider.scan(&request).await.expect("scan succeeds");
    println!("phishing text result: {result:?}");
    assert!(!result.known_hash_match);
    assert!(
        result.score.is_some_and(|s| s >= 70),
        "phishing should be detected with high confidence, got {:?}",
        result.score
    );

    let verdict = route(
        std::slice::from_ref(&result),
        &permissive_policy(),
        "live-scan",
    );
    println!("phishing text verdict: {verdict:?}");
    assert!(!verdict.is_indexable(), "flagged content must not index");
    assert!(
        !verdict.critical,
        "guard mode must not escalate to critical"
    );
}

#[tokio::test]
#[ignore = "requires a live openai-compatible endpoint (see module doc)"]
async fn live_image_scan_via_media_hint_succeeds() {
    let provider = live_provider();
    let request = ProviderScanRequest::for_subject(SubjectKind::Blob, "live-blob-1")
        .with_media_hint("blake3:live-test-image");
    let result = provider.scan(&request).await.expect("image scan succeeds");
    println!("image scan result: {result:?}");
    assert_eq!(result.outcome, ScanOutcome::Completed);
    assert!(!result.known_hash_match);

    let verdict = route(
        std::slice::from_ref(&result),
        &permissive_policy(),
        "live-scan",
    );
    println!("image verdict: {verdict:?}");
    assert!(
        verdict.is_indexable(),
        "a harmless 1x1 png must be indexable"
    );
}
