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

// ---- WP-S3 T3: 凍結 golden ----
// 以下の golden が fail した場合、テストではなく変更(コード・依存更新)の
// 互換影響を評価すること。署名対象 digest と検証互換は cn_safety.safety_events
// に永続済みの署名の有効性と直結する。

/// 署名対象 digest = sha256(canonical_bytes) の凍結。
/// canonical golden(cn-safety/tests/domain_model.rs)と対になる。
#[test]
fn canonical_digest_matches_golden() {
    use sha2::{Digest, Sha256};
    let body = sample_body("node-1");
    let digest = Sha256::digest(body.canonical_bytes());
    assert_eq!(
        hex::encode(digest),
        "3c32dc1941d40130bb5c2fc1e052d4e57c9ebcacbc90cd3851d10715a3fe5758"
    );
}

/// issuer_node_id = 署名鍵の x-only 公開鍵 hex の値レベル凍結。
#[test]
fn issuer_node_id_matches_golden_for_fixed_secret() {
    let signer = Secp256k1ModerationEventSigner::from_secret(TEST_SECRET_A).unwrap();
    assert_eq!(
        signer.issuer_node_id(),
        "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"
    );
}

/// 過去に生成・署名された SignedModerationEvent(2026-07-03 生成、
/// TEST_SECRET_A で署名)が verify を通り続け、serde 表現も不変であることの凍結。
/// 署名バイト自体は非決定的なため pin せず、fixture の検証互換のみを固定する。
#[test]
fn signed_event_fixture_still_verifies_and_reserializes_identically() {
    const FIXTURE: &str = r#"{"body":{"id":"evt-1","issuer_node_id":"79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798","target_type":"blob","target_id":"bafy-target","action":"exclude","labels":[{"category":"csam"}],"reason_code":"csam_confirmed","severity":"critical","basis":"known_hash_match","visibility":"subscribed_nodes","policy_version":"2026-06-public-node-v1","created_at":"2026-06-29T00:00:00Z"},"signature":"a7a5450205fe794a3f9f01a829fe1bb6bfafd55016ac76defc86bfeaf3d5635eab45362d204b7688e5fc68524ebe9a09c0943ce6b502aa6660ea6fab83be929f"}"#;
    let signed: kukuri_cn_safety::SignedModerationEvent =
        serde_json::from_str(FIXTURE).expect("fixture parses");
    verify_signed_event(&signed).expect("historically signed event must keep verifying");
    assert_eq!(serde_json::to_string(&signed).expect("serialize"), FIXTURE);
}
