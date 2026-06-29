//! secp256k1 schnorr 本番 signer / verify の contract テスト（#405）。
//!
//! - issuer_node_id が署名鍵の x-only 公開鍵 hex になる。
//! - 同一 body は決定論的に署名でき、verify が通る。
//! - body 改竄 / 別鍵署名 / issuer 詐称は verify に失敗する。

use kukuri_cn_safety::event::{ModerationEventBody, issue_signed_event};
use kukuri_cn_safety::provider::SubjectKind;
use kukuri_cn_safety::{
    Basis, ModerationAction, ModerationEventSigner, ReasonCode, SafetyCategory, SafetyLabel,
    Severity, Visibility,
};
use kukuri_cn_safety_runtime::{
    SAFETY_SIGNING_KEY_ENV, Secp256k1ModerationEventSigner, SignatureError, SignerKeyError,
    verify_signed_event,
};

// 決定論的なテスト鍵（32 byte hex）。テスト専用であり本番鍵ではない。
const TEST_SECRET_A: &str = "0000000000000000000000000000000000000000000000000000000000000001";
const TEST_SECRET_B: &str = "0000000000000000000000000000000000000000000000000000000000000002";

fn sample_body(issuer_node_id: &str) -> ModerationEventBody {
    ModerationEventBody {
        id: "evt-1".to_string(),
        issuer_node_id: issuer_node_id.to_string(),
        target_type: SubjectKind::Blob,
        target_id: "bafy-target".to_string(),
        action: ModerationAction::Exclude,
        labels: vec![SafetyLabel::new(SafetyCategory::Csam)],
        reason_code: ReasonCode::CsamConfirmed,
        severity: Severity::Critical,
        confidence: None,
        basis: Basis::KnownHashMatch,
        visibility: Visibility::SubscribedNodes,
        policy_version: "2026-06-public-node-v1".to_string(),
        created_at: "2026-06-29T00:00:00Z".to_string(),
    }
}

#[test]
fn issuer_node_id_is_signing_key_xonly_pubkey() {
    let signer = Secp256k1ModerationEventSigner::from_secret(TEST_SECRET_A).unwrap();
    // x-only 公開鍵 hex は 64 文字（32 byte）。
    assert_eq!(signer.issuer_node_id().len(), 64);
    assert!(
        signer
            .issuer_node_id()
            .chars()
            .all(|c| c.is_ascii_hexdigit())
    );
}

#[test]
fn sign_is_deterministic_and_verifies() {
    let signer = Secp256k1ModerationEventSigner::from_secret(TEST_SECRET_A).unwrap();
    let body = sample_body(signer.issuer_node_id());

    // schnorr 署名は nonce により毎回異なり得るが、いずれも検証は通る。
    let signed = issue_signed_event(body.clone(), &signer);
    assert_eq!(signed.body, body);
    verify_signed_event(&signed).expect("valid signature verifies");

    // 署名対象 digest の決定性（同一 body の sign を 2 回検証）。
    let signed_again = issue_signed_event(body, &signer);
    verify_signed_event(&signed_again).expect("valid signature verifies");
}

#[test]
fn tampered_body_fails_verification() {
    let signer = Secp256k1ModerationEventSigner::from_secret(TEST_SECRET_A).unwrap();
    let body = sample_body(signer.issuer_node_id());
    let mut signed = issue_signed_event(body, &signer);

    // body を 1 フィールド改竄すると canonical digest が変わり検証に失敗する。
    signed.body.target_id = "tampered".to_string();
    assert_eq!(
        verify_signed_event(&signed),
        Err(SignatureError::VerificationFailed)
    );
}

#[test]
fn signature_from_other_key_fails_verification() {
    let signer_a = Secp256k1ModerationEventSigner::from_secret(TEST_SECRET_A).unwrap();
    let signer_b = Secp256k1ModerationEventSigner::from_secret(TEST_SECRET_B).unwrap();

    // issuer は A だが、B の鍵で署名する。
    let body = sample_body(signer_a.issuer_node_id());
    let mut signed = issue_signed_event(body.clone(), &signer_a);
    signed.signature = signer_b.sign(&body);

    assert_eq!(
        verify_signed_event(&signed),
        Err(SignatureError::VerificationFailed)
    );
}

#[test]
fn spoofed_issuer_fails_verification() {
    let signer_a = Secp256k1ModerationEventSigner::from_secret(TEST_SECRET_A).unwrap();
    let signer_b = Secp256k1ModerationEventSigner::from_secret(TEST_SECRET_B).unwrap();

    // B が署名した event の issuer を A に書き換える（issuer 詐称）。
    let body = sample_body(signer_b.issuer_node_id());
    let mut signed = issue_signed_event(body, &signer_b);
    signed.body.issuer_node_id = signer_a.issuer_node_id().to_string();

    assert_eq!(
        verify_signed_event(&signed),
        Err(SignatureError::VerificationFailed)
    );
}

#[test]
fn invalid_issuer_encoding_is_reported() {
    let signer = Secp256k1ModerationEventSigner::from_secret(TEST_SECRET_A).unwrap();
    let body = sample_body("not-a-valid-pubkey");
    let signed = issue_signed_event(body, &signer);
    assert_eq!(
        verify_signed_event(&signed),
        Err(SignatureError::InvalidIssuer)
    );
}

#[test]
fn invalid_secret_is_rejected() {
    assert!(matches!(
        Secp256k1ModerationEventSigner::from_secret("not-a-key"),
        Err(SignerKeyError::InvalidKey { .. })
    ));
    assert!(matches!(
        Secp256k1ModerationEventSigner::from_secret(""),
        Err(SignerKeyError::Missing)
    ));
    assert!(matches!(
        Secp256k1ModerationEventSigner::from_secret("change-me-please"),
        Err(SignerKeyError::Placeholder)
    ));
}

#[test]
fn from_env_reads_injected_key() {
    // env var を順次操作するため、1 テスト内で逐次検証する（並行 env 変更を避ける）。
    // SAFETY: テストはこの env を専有する単一テスト。
    unsafe {
        std::env::remove_var(SAFETY_SIGNING_KEY_ENV);
    }
    assert!(matches!(
        Secp256k1ModerationEventSigner::from_env(),
        Err(SignerKeyError::Missing)
    ));

    unsafe {
        std::env::set_var(SAFETY_SIGNING_KEY_ENV, TEST_SECRET_A);
    }
    let from_env = Secp256k1ModerationEventSigner::from_env().unwrap();
    let from_secret = Secp256k1ModerationEventSigner::from_secret(TEST_SECRET_A).unwrap();
    assert_eq!(from_env.issuer_node_id(), from_secret.issuer_node_id());

    unsafe {
        std::env::remove_var(SAFETY_SIGNING_KEY_ENV);
    }
}
