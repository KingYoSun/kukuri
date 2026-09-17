use async_trait::async_trait;
use kukuri_cn_safety::{
    FetchedMedia, MediaFetcher, ProviderScanRequest, SafetyProvider, ScanError, SubjectKind,
    provider::ScanReferenceGuard,
};
use kukuri_cn_safety_arachnid::{
    ProjectArachnidShieldProvider, ShieldCredentials, ShieldProviderConfig,
};
use kukuri_cn_safety_vlm::{
    CapabilityProfile, VlmCredentials, VlmModerationProvider, VlmProviderConfig, VlmResponseFormat,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

struct Guard(Arc<AtomicBool>);
#[async_trait]
impl ScanReferenceGuard for Guard {
    async fn check(&self) -> Result<(), ScanError> {
        if self.0.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(ScanError::Unavailable("reference revoked".into()))
        }
    }
}
struct RevokingFetcher(Arc<AtomicBool>);
#[async_trait]
impl MediaFetcher for RevokingFetcher {
    async fn fetch(&self, _: &str, _: Option<&str>) -> Result<FetchedMedia, ScanError> {
        tokio::task::yield_now().await;
        self.0.store(false, Ordering::SeqCst);
        Ok(FetchedMedia {
            bytes: vec![0xff, 0xd8, 0xff, 0xe0],
            content_type: "image/jpeg".into(),
        })
    }
}
fn arachnid(url: &str) -> ProjectArachnidShieldProvider {
    ProjectArachnidShieldProvider::with_credentials(
        &ShieldProviderConfig {
            api_base_url: url.into(),
            ..Default::default()
        },
        ShieldCredentials::new("synthetic-user", "synthetic-password"),
    )
    .expect("synthetic provider fixture")
}
async fn assert_revocation_blocks(
    provider: Box<dyn SafetyProvider>,
    server: MockServer,
    active: Arc<AtomicBool>,
) {
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"classification":"no-known-match","choices":[{"message":{"content":"safe"}}]})))
        .mount(&server).await;
    let request = ProviderScanRequest::for_subject(SubjectKind::Blob, "a".repeat(64))
        .with_media_hint("a".repeat(64));
    let result = provider.scan_guarded(&request, &Guard(active)).await;
    let hits = server
        .received_requests()
        .await
        .expect("synthetic provider fixture")
        .len();
    println!(
        "provider={}, HTTP hits after revocation={hits}, result={result:?}",
        provider.name()
    );
    assert_eq!(
        hits, 0,
        "revoked reference must not be sent after fetch completes"
    );
    assert!(result.is_err());
}
#[tokio::test]
async fn arachnid_rechecks_guard_after_media_fetch() {
    let server = MockServer::start().await;
    let active = Arc::new(AtomicBool::new(true));
    let provider =
        arachnid(&server.uri()).with_media_fetcher(Arc::new(RevokingFetcher(active.clone())));
    assert_revocation_blocks(Box::new(provider), server, active).await;
}
#[tokio::test]
async fn vlm_rechecks_guard_after_media_fetch() {
    let server = MockServer::start().await;
    let active = Arc::new(AtomicBool::new(true));
    let provider = VlmModerationProvider::with_credentials(
        &VlmProviderConfig {
            api_base_url: server.uri(),
            api_key_env: "AUDIT_UNUSED".into(),
            model: "synthetic-model".into(),
            response_format: VlmResponseFormat::Guard,
            timeout: std::time::Duration::from_secs(2),
        },
        VlmCredentials::new("synthetic-key"),
        CapabilityProfile::General,
    )
    .expect("synthetic provider fixture")
    .with_media_fetcher(Arc::new(RevokingFetcher(active.clone())));
    assert_revocation_blocks(Box::new(provider), server, active).await;
}
#[test]
fn arachnid_endpoint_change_invalidates_content_identity() {
    let first = arachnid("http://127.0.0.1:30001");
    let second = arachnid("http://127.0.0.1:30002");
    println!(
        "first={}, second={}",
        first.config_fingerprint(),
        second.config_fingerprint()
    );
    assert_ne!(
        first.config_fingerprint(),
        second.config_fingerprint(),
        "a provider endpoint/config change must invalidate cached content"
    );
}
