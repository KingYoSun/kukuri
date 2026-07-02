//! Project Arachnid Shield provider の contract テスト（#391 受け入れ条件）。
//!
//! wiremock で Shield API を模擬し、以下を固定する:
//! - Exact / Near Match → `Exclude`（critical / `csam_confirmed`）
//! - `test` classification → `Exclude` だが `provider_test_match`（`csam_confirmed` と区別）
//! - `no-known-match` → `NoKnownMatch`（allow だが **safe の証明ではない** reason）
//! - 401 / 5xx / timeout / fetcher 未構成 → fail-closed（`ScanError`、allow に落ちない）
//! - credentials / Match Data（near_match_details / sha 群）が結果・Debug・エラーに出ない

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use wiremock::matchers::{basic_auth, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use kukuri_cn_safety::provider::{
    FetchedMedia, MediaFetcher, ProviderScanRequest, ScanError, ScanOutcome, SubjectKind,
};
use kukuri_cn_safety::{
    ReasonCode, SafetyAction, SafetyCategory, SafetyPolicy, SafetyProvider,
    SafetyProviderCapability, route,
};
use kukuri_cn_safety_arachnid::{
    ProjectArachnidShieldProvider, ShieldConfigError, ShieldCredentials, ShieldProviderConfig,
};

const USERNAME: &str = "shield-user";
const PASSWORD: &str = "shield-pass";
/// Match Data 非露出テスト用の sentinel（模擬応答の near_match_details にのみ現れる値）。
const MATCH_DATA_SENTINEL: &str = "sentinel-sha256-0123456789abcdef";

fn test_config(base_url: &str) -> ShieldProviderConfig {
    ShieldProviderConfig {
        api_base_url: base_url.to_string(),
        timeout: Duration::from_millis(500),
        ..ShieldProviderConfig::default()
    }
}

fn provider_for(server_url: &str) -> ProjectArachnidShieldProvider {
    ProjectArachnidShieldProvider::with_credentials(
        &test_config(server_url),
        ShieldCredentials::new(USERNAME, PASSWORD),
    )
    .expect("provider construction should succeed")
    .with_media_fetcher(Arc::new(StaticFetcher))
}

fn media_request() -> ProviderScanRequest {
    ProviderScanRequest::for_subject(SubjectKind::Blob, "blob-1").with_media_hint("blake3:abc123")
}

/// 固定 bytes を返すテスト用 fetcher。
struct StaticFetcher;

#[async_trait]
impl MediaFetcher for StaticFetcher {
    async fn fetch(&self, _media_hint: &str) -> Result<FetchedMedia, ScanError> {
        Ok(FetchedMedia {
            bytes: vec![0xFF, 0xD8, 0xFF, 0xE0],
            content_type: "image/jpeg".to_string(),
        })
    }
}

/// Shield の `ScannedMedia` 応答（Match Data 込み。実応答と同じ形）。
fn scanned_media_body(classification: &str, match_type: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "classification": classification,
        "match_type": match_type,
        "near_match_details": [{
            "classification": "csam",
            "sha1_base32": "sentinel-sha1-abcdefg",
            "sha256_hex": MATCH_DATA_SENTINEL,
            "timestamp": 1719900000.0,
        }],
        "sha1_base32": "sentinel-submitted-sha1",
        "sha256_hex": "sentinel-submitted-sha256",
        "size_bytes": 4,
    })
}

async fn mock_media_scan(server: &MockServer, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/v1/media"))
        .and(basic_auth(USERNAME, PASSWORD))
        .and(header("content-type", "image/jpeg"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

#[tokio::test]
async fn exact_match_is_known_hash_match_and_routes_to_csam_confirmed_exclude() {
    let server = MockServer::start().await;
    mock_media_scan(&server, scanned_media_body("csam", Some("exact"))).await;

    let provider = provider_for(&server.uri());
    let result = provider
        .scan(&media_request())
        .await
        .expect("scan succeeds");

    assert_eq!(result.outcome, ScanOutcome::Completed);
    assert!(result.known_hash_match);
    assert_eq!(
        result.capability,
        SafetyProviderCapability::KnownCsamHashMatch
    );

    let verdict = route(
        std::slice::from_ref(&result),
        &SafetyPolicy::public_node_default(),
        "scanned-at",
    );
    assert_eq!(verdict.action, SafetyAction::Exclude);
    assert_eq!(verdict.reason_code, ReasonCode::CsamConfirmed);
    assert!(verdict.critical);
    assert!(!verdict.is_indexable());
}

#[tokio::test]
async fn near_match_uses_perceptual_hash_capability_and_is_excluded() {
    let server = MockServer::start().await;
    mock_media_scan(&server, scanned_media_body("csam", Some("near"))).await;

    let provider = provider_for(&server.uri());
    let result = provider
        .scan(&media_request())
        .await
        .expect("scan succeeds");

    assert!(result.known_hash_match);
    assert_eq!(
        result.capability,
        SafetyProviderCapability::PerceptualHashMatch
    );

    let verdict = route(
        std::slice::from_ref(&result),
        &SafetyPolicy::public_node_default(),
        "scanned-at",
    );
    assert_eq!(verdict.action, SafetyAction::Exclude);
    assert!(!verdict.is_indexable());
}

#[tokio::test]
async fn harmful_abusive_material_is_excluded_as_known_match() {
    let server = MockServer::start().await;
    mock_media_scan(
        &server,
        scanned_media_body("harmful-abusive-material", Some("exact")),
    )
    .await;

    let provider = provider_for(&server.uri());
    let result = provider
        .scan(&media_request())
        .await
        .expect("scan succeeds");

    assert!(result.known_hash_match);
    let verdict = route(
        std::slice::from_ref(&result),
        &SafetyPolicy::public_node_default(),
        "scanned-at",
    );
    assert_eq!(verdict.action, SafetyAction::Exclude);
    assert!(!verdict.is_indexable());
}

#[tokio::test]
async fn test_classification_routes_to_provider_test_match_not_csam_confirmed() {
    let server = MockServer::start().await;
    mock_media_scan(&server, scanned_media_body("test", Some("exact"))).await;

    let provider = provider_for(&server.uri());
    let result = provider
        .scan(&media_request())
        .await
        .expect("scan succeeds");

    // 実 CSAM の confirmed とは区別される（known_hash_match ではなく専用ラベル）。
    assert!(!result.known_hash_match);
    assert_eq!(result.labels.len(), 1);
    assert_eq!(result.labels[0].category, SafetyCategory::ProviderTest);

    let verdict = route(
        std::slice::from_ref(&result),
        &SafetyPolicy::public_node_default(),
        "scanned-at",
    );
    // exclude + 専用扱い: index には入れないが csam_confirmed でも critical でもない。
    assert_eq!(verdict.action, SafetyAction::Exclude);
    assert_eq!(verdict.reason_code, ReasonCode::ProviderTestMatch);
    assert!(!verdict.critical);
    assert!(!verdict.is_indexable());
}

#[tokio::test]
async fn no_known_match_is_not_treated_as_safe() {
    let server = MockServer::start().await;
    mock_media_scan(&server, scanned_media_body("no-known-match", None)).await;

    let provider = provider_for(&server.uri());
    let result = provider
        .scan(&media_request())
        .await
        .expect("scan succeeds");

    assert_eq!(result.outcome, ScanOutcome::NoKnownMatch);
    assert!(!result.known_hash_match);

    let verdict = route(
        std::slice::from_ref(&result),
        &SafetyPolicy::public_node_default(),
        "scanned-at",
    );
    // index は許すが、reason は Clean ではなく NoKnownMatch（safe の証明ではない）。
    assert_eq!(verdict.action, SafetyAction::Allow);
    assert_eq!(verdict.reason_code, ReasonCode::NoKnownMatch);
}

#[tokio::test]
async fn match_data_never_appears_in_scan_result_serialization() {
    let server = MockServer::start().await;
    mock_media_scan(&server, scanned_media_body("csam", Some("near"))).await;

    let provider = provider_for(&server.uri());
    let result = provider
        .scan(&media_request())
        .await
        .expect("scan succeeds");

    // ProviderScanResult（persist / signed event の材料）に Match Data が構造的に存在しない。
    let serialized = serde_json::to_string(&result).expect("result serializes");
    assert!(!serialized.contains(MATCH_DATA_SENTINEL));
    assert!(!serialized.contains("sentinel-sha1"));
    assert!(!serialized.contains("sentinel-submitted"));
    assert!(!serialized.contains("near_match_details"));
}

#[tokio::test]
async fn moderation_event_from_exact_match_contains_no_match_data() {
    use kukuri_cn_safety_runtime::{SafetyOrchestrator, SystemScanClock, UuidEventIdGenerator};

    let server = MockServer::start().await;
    mock_media_scan(&server, scanned_media_body("csam", Some("exact"))).await;

    let orchestrator = SafetyOrchestrator::builder(
        "issuer-node",
        Arc::new(SystemScanClock::new()),
        Arc::new(UuidEventIdGenerator::new()),
    )
    .provider(Arc::new(provider_for(&server.uri())))
    .build()
    .expect("orchestrator builds");

    let report = orchestrator.scan_subject(&media_request()).await;
    assert_eq!(report.verdict.reason_code, ReasonCode::CsamConfirmed);
    assert!(!report.verdict.is_indexable());

    // 署名・永続化・配布の材料になる moderation event / scan report 全体に Match Data が無い。
    let event = report
        .moderation_event
        .as_ref()
        .expect("non-indexable verdict produces a moderation event");
    let serialized_event = serde_json::to_string(event).expect("event serializes");
    assert!(!serialized_event.contains(MATCH_DATA_SENTINEL));
    let serialized_report = serde_json::to_string(&report).expect("report serializes");
    assert!(!serialized_report.contains(MATCH_DATA_SENTINEL));
    assert!(!serialized_report.contains("sentinel-sha1"));
    assert!(!serialized_report.contains("sentinel-submitted"));
}

#[tokio::test]
async fn unauthorized_fails_closed_without_leaking_credentials() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/media"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let provider = provider_for(&server.uri());
    let error = provider
        .scan(&media_request())
        .await
        .expect_err("401 must be an error (fail-closed)");

    let ScanError::Unavailable(message) = &error else {
        panic!("401 must map to ScanError::Unavailable, got {error:?}");
    };
    assert!(message.contains("credential"));
    assert!(!message.contains(USERNAME));
    assert!(!message.contains(PASSWORD));
}

#[tokio::test]
async fn server_error_fails_closed_as_unavailable() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/media"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let provider = provider_for(&server.uri());
    let error = provider
        .scan(&media_request())
        .await
        .expect_err("5xx must be an error (fail-closed)");
    assert!(matches!(error, ScanError::Unavailable(_)));
}

#[tokio::test]
async fn timeout_fails_closed_as_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/media"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(scanned_media_body("no-known-match", None))
                .set_delay(Duration::from_secs(5)),
        )
        .mount(&server)
        .await;

    let provider = provider_for(&server.uri());
    let error = provider
        .scan(&media_request())
        .await
        .expect_err("timeout must be an error (fail-closed)");
    assert!(matches!(error, ScanError::Timeout(_)));
}

#[tokio::test]
async fn unexpected_response_shape_fails_closed_as_protocol_error() {
    let server = MockServer::start().await;
    mock_media_scan(
        &server,
        // 未知 classification（API 将来拡張）は安全側でない解釈をせず Protocol エラーに倒す。
        scanned_media_body("brand-new-classification", None),
    )
    .await;

    let provider = provider_for(&server.uri());
    let error = provider
        .scan(&media_request())
        .await
        .expect_err("unknown classification must fail closed");
    assert!(matches!(error, ScanError::Protocol(_)));
}

#[tokio::test]
async fn media_hint_without_fetcher_fails_closed() {
    let provider = ProjectArachnidShieldProvider::with_credentials(
        &test_config("http://127.0.0.1:9"),
        ShieldCredentials::new(USERNAME, PASSWORD),
    )
    .expect("provider construction should succeed");

    let error = provider
        .scan(&media_request())
        .await
        .expect_err("media scan without fetcher must fail closed");
    assert!(matches!(error, ScanError::Unavailable(_)));
}

#[tokio::test]
async fn request_without_media_reports_no_known_match_scan() {
    let provider = ProjectArachnidShieldProvider::with_credentials(
        &test_config("http://127.0.0.1:9"),
        ShieldCredentials::new(USERNAME, PASSWORD),
    )
    .expect("provider construction should succeed");

    let request = ProviderScanRequest::for_subject(SubjectKind::Post, "post-1");
    let result = provider.scan(&request).await.expect("scan succeeds");

    // media の無い対象は照合対象が無い = NoKnownMatch（safe の証明ではない）。
    assert_eq!(result.outcome, ScanOutcome::NoKnownMatch);
    assert_eq!(
        result.capability,
        SafetyProviderCapability::KnownCsamHashMatch
    );
}

#[test]
fn debug_output_redacts_credentials() {
    let provider = ProjectArachnidShieldProvider::with_credentials(
        &test_config("http://127.0.0.1:9"),
        ShieldCredentials::new(USERNAME, PASSWORD),
    )
    .expect("provider construction should succeed");

    let debug = format!("{provider:?}");
    assert!(!debug.contains(USERNAME));
    assert!(!debug.contains(PASSWORD));
    assert!(debug.contains("<redacted>"));
}

#[test]
fn missing_credential_error_names_env_without_value() {
    let config = ShieldProviderConfig {
        api_username_env: "KUKURI_TEST_ARACHNID_MISSING_USER".to_string(),
        api_password_env: "KUKURI_TEST_ARACHNID_MISSING_PASSWORD".to_string(),
        ..ShieldProviderConfig::default()
    };
    let error = config
        .load_credentials()
        .expect_err("missing env must be an error (fail-closed)");
    assert_eq!(
        error,
        ShieldConfigError::MissingCredential {
            env: "KUKURI_TEST_ARACHNID_MISSING_USER".to_string(),
        }
    );
    assert!(
        error
            .to_string()
            .contains("KUKURI_TEST_ARACHNID_MISSING_USER")
    );
}
