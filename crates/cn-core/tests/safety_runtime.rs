//! SafetyScanService / 構築境界の決定論的 contract テスト（#406）。DB 不要。
//!
//! mock provider + ローカル定義の固定 clock / 連番 id + 固定テスト鍵 signer +
//! in-memory store で、runtime 経由の scan → verdict → artifact 署名 / 永続化と
//! fail-closed を検証する。

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use kukuri_cn_core::{
    MemorySafetyArtifactStore, SafetyRuntimeConfig, SafetyRuntimeProviderEntry,
    SafetyRuntimeProvidersConfig, SafetyScanService, build_safety_orchestrator,
    build_safety_scan_service,
};
use kukuri_cn_safety::provider::{ProviderScanRequest, ScanError, SubjectKind};
use kukuri_cn_safety::{
    MockSafetyProvider, ModerationEventSigner, ReasonCode, RiskSignalTarget, SafetyCategory,
    SafetyProvider,
};
use kukuri_cn_safety_runtime::{
    EventIdGenerator, SafetyOrchestrator, ScanClock, Secp256k1ModerationEventSigner,
    verify_signed_event,
};

const SCANNED_AT: &str = "2026-07-02T09:00:00Z";
const TEST_SECRET: &str = "0000000000000000000000000000000000000000000000000000000000000001";

struct FixedClock(&'static str);
impl ScanClock for FixedClock {
    fn now_rfc3339(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Default)]
struct SequentialIdGenerator {
    next: AtomicU64,
}
impl EventIdGenerator for SequentialIdGenerator {
    fn next_id(&self) -> String {
        let n = self.next.fetch_add(1, Ordering::SeqCst);
        format!("evt-{n}")
    }
}

fn signer() -> Secp256k1ModerationEventSigner {
    Secp256k1ModerationEventSigner::from_secret(TEST_SECRET).unwrap()
}

fn orchestrator(issuer: &str, provider: MockSafetyProvider) -> Arc<SafetyOrchestrator> {
    Arc::new(
        SafetyOrchestrator::builder(
            issuer,
            Arc::new(FixedClock(SCANNED_AT)),
            Arc::new(SequentialIdGenerator::default()),
        )
        .provider(Arc::new(provider) as Arc<dyn SafetyProvider>)
        .build()
        .unwrap(),
    )
}

/// signer + memory store で組んだ service（本番構成と同型、DB のみ in-memory）。
fn signed_service(
    provider: MockSafetyProvider,
) -> (SafetyScanService, Arc<MemorySafetyArtifactStore>, String) {
    let signer = signer();
    let issuer = signer.issuer_node_id().to_string();
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let service = SafetyScanService::builder(orchestrator(&issuer, provider), store.clone())
        .signer(Arc::new(signer))
        .build()
        .unwrap();
    (service, store, issuer)
}

fn post_request(post_id: &str) -> ProviderScanRequest {
    ProviderScanRequest::for_subject(SubjectKind::Post, post_id)
}

// --- scan → verdict → artifact 永続化（#406 受け入れ条件の runtime 経由 contract） ---

#[tokio::test]
async fn runtime_scan_known_csam_persists_signed_event_and_risk_signal() {
    let provider =
        MockSafetyProvider::known_csam("mock-known-csam").with_known_hash_match("post-1");
    let (service, store, issuer) = signed_service(provider);

    let outcome = service
        .scan_and_record(&post_request("post-1"))
        .await
        .unwrap();

    // verdict は exclude / confirmed（fail-closed gate 側の判定に使う）。
    assert!(!outcome.report.verdict.is_indexable());
    assert_eq!(
        outcome.report.verdict.reason_code,
        ReasonCode::CsamConfirmed
    );

    // moderation event は実鍵署名され、検証に通る形で store に入る。
    let event = outcome.signed_event.expect("signed moderation event");
    verify_signed_event(&event).unwrap();
    assert_eq!(event.body.issuer_node_id, issuer);
    assert_eq!(event.body.target_id, "post-1");
    assert_eq!(store.events().len(), 1);
    assert_eq!(store.events()[0], event);

    // risk signal も issuer つきで store に入る（trust/relation reads の入力になる）。
    let signals = store.signals();
    assert_eq!(signals.len(), 1);
    let (signal_issuer, signal) = &signals[0];
    assert_eq!(signal_issuer, &issuer);
    assert_eq!(signal.target, RiskSignalTarget::PostId);
    assert_eq!(signal.target_id, "post-1");
    assert_eq!(signal.category, SafetyCategory::Csam);
    assert!(outcome.persisted_signal_id.is_some());
}

#[tokio::test]
async fn runtime_scan_allow_persists_no_artifacts() {
    // known CSAM provider の既知一致なし → allow（NoKnownMatch。safe の証明ではない）。
    let provider = MockSafetyProvider::known_csam("mock-known-csam");
    let (service, store, _issuer) = signed_service(provider);

    let outcome = service
        .scan_and_record(&post_request("post-clean"))
        .await
        .unwrap();

    assert!(outcome.report.verdict.is_indexable());
    assert!(outcome.signed_event.is_none());
    assert!(outcome.persisted_signal_id.is_none());
    assert!(store.events().is_empty());
    assert!(store.signals().is_empty());
}

// --- provider failure / unavailable の fail-closed（runtime 経由でも固定） ---

#[tokio::test]
async fn runtime_provider_failure_is_fail_closed_and_persists_no_risk_signal() {
    let provider = MockSafetyProvider::known_csam("mock-known-csam").default_failed();
    let (service, store, _issuer) = signed_service(provider);

    let outcome = service
        .scan_and_record(&post_request("post-x"))
        .await
        .unwrap();

    assert!(!outcome.report.verdict.is_indexable());
    assert_eq!(outcome.report.verdict.reason_code, ReasonCode::ScanFailed);
    // operational fail-closed は content の safety category を示さないため、
    // 虚偽の risk signal を作らない（監査用 moderation event は hold として残る）。
    assert!(outcome.persisted_signal_id.is_none());
    assert!(store.signals().is_empty());
    assert_eq!(store.events().len(), 1);
    assert_eq!(store.events()[0].body.reason_code, ReasonCode::ScanFailed);
}

#[tokio::test]
async fn runtime_provider_unavailable_is_fail_closed() {
    let provider = MockSafetyProvider::known_csam("mock-known-csam")
        .default_error(ScanError::Unavailable("mock provider down".to_string()));
    let (service, store, _issuer) = signed_service(provider);

    let outcome = service
        .scan_and_record(&post_request("post-x"))
        .await
        .unwrap();

    assert!(!outcome.report.verdict.is_indexable());
    assert_eq!(
        outcome.report.verdict.reason_code,
        ReasonCode::ProviderUnavailable
    );
    assert!(store.signals().is_empty());
}

// --- service builder の fail-closed ---

#[test]
fn runtime_requires_signer_when_signed_events_enabled() {
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let error = SafetyScanService::builder(
        orchestrator(
            "issuer-node",
            MockSafetyProvider::known_csam("mock-known-csam"),
        ),
        store,
    )
    .build()
    .unwrap_err();
    assert!(error.to_string().contains("no signer"), "{error}");
}

#[tokio::test]
async fn runtime_without_signed_events_persists_risk_signal_only() {
    let provider =
        MockSafetyProvider::known_csam("mock-known-csam").with_known_hash_match("post-1");
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let service = SafetyScanService::builder(orchestrator("issuer-node", provider), store.clone())
        .without_signed_events("issuer-node")
        .build()
        .unwrap();

    let outcome = service
        .scan_and_record(&post_request("post-1"))
        .await
        .unwrap();

    // moderation event は署名・永続化しない（report には未署名 body が残る）。
    assert!(outcome.signed_event.is_none());
    assert!(store.events().is_empty());
    assert!(outcome.report.moderation_event.is_some());
    // risk signal は署名対象ではないため引き続き永続化される。
    assert_eq!(store.signals().len(), 1);
    assert_eq!(store.signals()[0].0, "issuer-node");
}

// --- 構築境界（config → orchestrator / service）の fail-closed ---

fn mock_slot() -> Option<SafetyRuntimeProviderEntry> {
    Some(SafetyRuntimeProviderEntry {
        provider: "mock".to_string(),
        required: true,
    })
}

#[test]
fn build_orchestrator_rejects_unknown_provider_name() {
    let providers = SafetyRuntimeProvidersConfig {
        known_csam: Some(SafetyRuntimeProviderEntry {
            provider: "arachnid-shield".to_string(),
            required: false,
        }),
        ..Default::default()
    };
    let error = build_safety_orchestrator("issuer-node", &providers, None).unwrap_err();
    assert!(
        error.to_string().contains("unknown safety provider"),
        "{error}"
    );
}

#[test]
fn build_orchestrator_resolves_arachnid_shield_only_with_credentials() {
    // env は process-global なため、この 1 テスト内で「欠落 → Err」と「設定 → Ok」を順に
    // 検証する（他テストはこの env を読まない）。
    let providers = SafetyRuntimeProvidersConfig {
        known_csam: Some(SafetyRuntimeProviderEntry {
            provider: "project-arachnid-shield".to_string(),
            required: true,
        }),
        ..Default::default()
    };

    // credentials 欠落 → Err（起動 fail-closed）。エラーは env 名のみで値を含まない。
    unsafe {
        std::env::remove_var("PROJECT_ARACHNID_API_USERNAME");
        std::env::remove_var("PROJECT_ARACHNID_API_PASSWORD");
    }
    let error = build_safety_orchestrator("issuer-node", &providers, None).unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("project-arachnid-shield"), "{message}");
    assert!(
        message.contains("PROJECT_ARACHNID_API_USERNAME"),
        "{message}"
    );

    // credentials があれば構築できる。
    unsafe {
        std::env::set_var("PROJECT_ARACHNID_API_USERNAME", "operator-user");
        std::env::set_var("PROJECT_ARACHNID_API_PASSWORD", "operator-pass");
    }
    let result = build_safety_orchestrator("issuer-node", &providers, None);
    unsafe {
        std::env::remove_var("PROJECT_ARACHNID_API_USERNAME");
        std::env::remove_var("PROJECT_ARACHNID_API_PASSWORD");
    }
    result.expect("orchestrator should build once credentials are configured");
}

#[test]
fn build_orchestrator_rejects_arachnid_shield_outside_known_csam_slot() {
    // Shield は known-match provider。general / unknown_csam slot への指定は fail-closed。
    let providers = SafetyRuntimeProvidersConfig {
        known_csam: mock_slot(),
        general: Some(SafetyRuntimeProviderEntry {
            provider: "project-arachnid-shield".to_string(),
            required: false,
        }),
        ..Default::default()
    };
    let error = build_safety_orchestrator("issuer-node", &providers, None).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("only supports the `known_csam` slot"),
        "{error}"
    );
}

#[test]
fn build_orchestrator_rejects_empty_provider_set() {
    let providers = SafetyRuntimeProvidersConfig::default();
    let error = build_safety_orchestrator("issuer-node", &providers, None).unwrap_err();
    assert!(error.to_string().contains("no safety providers"), "{error}");
}

#[test]
fn build_scan_service_returns_none_without_providers() {
    let config = SafetyRuntimeConfig::default();
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let service = build_safety_scan_service(&config, store).unwrap();
    assert!(service.is_none());
}

#[test]
fn build_scan_service_requires_signing_key_when_emit_enabled() {
    let config = SafetyRuntimeConfig {
        providers: SafetyRuntimeProvidersConfig {
            known_csam: mock_slot(),
            ..Default::default()
        },
        ..Default::default()
    };
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let error = build_safety_scan_service(&config, store).unwrap_err();
    assert!(error.to_string().contains("no signing key"), "{error}");
}

#[tokio::test]
async fn build_scan_service_constructs_mock_orchestrator_and_scans() {
    // SystemScanClock + UuidEventIdGenerator + mock provider での組み立て（#406）。
    // issuer は署名鍵の公開鍵 hex に構造的に一致する。
    let config = SafetyRuntimeConfig {
        providers: SafetyRuntimeProvidersConfig {
            known_csam: mock_slot(),
            general: mock_slot(),
            unknown_csam: mock_slot(),
        },
        signing_key: Some(TEST_SECRET.to_string()),
        ..Default::default()
    };
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let service = build_safety_scan_service(&config, store.clone())
        .unwrap()
        .expect("service should be constructed");
    assert_eq!(service.issuer_node_id(), signer().issuer_node_id());

    // 既知一致なし → allow（artifact なし）まで runtime 経由で通ることを確認する。
    let outcome = service
        .scan_and_record(&post_request("post-clean"))
        .await
        .unwrap();
    assert!(outcome.report.verdict.is_indexable());
    assert!(store.events().is_empty());
    assert!(store.signals().is_empty());
}
