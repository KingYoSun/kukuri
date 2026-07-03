//! 署名 canonical 形式の凍結テスト(WP-S3 T1)。
//!
//! ここで固定しているのは protocol の凍結境界そのものである:
//! - envelope / DM frame / DM ack の canonical 文字列(1 バイトの変更で全既存署名が無効化する)
//! - envelope id の導出(hex(sha256(canonical)))
//! - 署名済み fixture に対する検証互換(canonical 再計算・schnorr 検証・AEAD 復号)
//! - serde_json の出力表現(キー順・数値表記・文字列エスケープ)への依存
//!
//! これらのテストが fail した場合、テストを直すのではなく、変更(コード・依存更新)の
//! 互換影響を評価して変更側を戻すこと。依存更新(serde_json / secp256k1 等)による
//! fail も検出対象であり、それがこのテストの目的である。
//!
//! 署名バイト(sig)は pin しない: `KukuriKeys::sign_schnorr` は aux_rand を毎回乱数生成
//! するため署名は非決定的で、pin すると恒常的に fail する。互換の凍結対象は「過去に
//! 生成された署名が verify を通り続けること」であり、下の署名済み fixture がそれを担う。
//!
//! fixture の再生成手順: 固定鍵 SECRET_A/B と本ファイルの固定入力(kind/tags/content/
//! created_at 等)を使い、sign_envelope_at / build_direct_message_ack /
//! encrypt_direct_message_frame の出力を serde_json::to_string したものを埋め込む
//! (このファイル導入 PR の golden_gen 一時テストを参照)。再生成が必要になるのは
//! 意図的な protocol 変更のときだけであり、その場合は migration 計画が先。

use crate::direct_messages::{
    canonical_direct_message_ack_payload, canonical_direct_message_frame_payload,
};
use crate::envelope::canonical_envelope_payload;
use crate::{
    DirectMessageAckV1, DirectMessageFrameV1, DirectMessagePayloadV1, KukuriEnvelope, KukuriKeys,
    Pubkey, build_direct_message_ack, decrypt_direct_message_frame, derive_direct_message_topic,
    direct_message_id_for_participants, sign_envelope_at,
};

/// 固定テスト鍵(secp256k1 の secret key = 1 / = 2)。実運用鍵ではない。
const SECRET_A: &str = "0000000000000000000000000000000000000000000000000000000000000001";
const SECRET_B: &str = "0000000000000000000000000000000000000000000000000000000000000002";
const PUBKEY_A: &str = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
const PUBKEY_B: &str = "c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5";

const DM_ID: &str = "dm-fff507b331171652f9c5ff4e5ed80847ea782a23f146c5d70e8f4acf2ab87778";
const DM_TOPIC: &str = "kukuri:dm:fc8118decb5a2cc5bfd7c0c8f403428aa33faf4b82a5125790c1f4991a003e73";

/// SECRET_A で created_at=1700000000 に署名した envelope(2026-07-03 生成)。
const ENVELOPE_FIXTURE: &str = r#"{"id":"029c604938bd3b122a84287be036a1b01035733358eacb892267a06579a8daa5","pubkey":"79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798","created_at":1700000000,"kind":"kukuri:post","tags":[["topic","kukuri:topic:demo"],["e","abc"]],"content":"hello \"golden\" \\ 日本語","sig":"20d97a809c957c98104883e5c9f00607612a1792970254abaf6195376e0c50ce82ae45236a6fdd5a3bdabd9fd95d1539cea39a60e5339e32c39cf3aa9833a7af"}"#;

/// SECRET_A が SECRET_B 宛に構築した ack(acked_at=1700000100)。
const ACK_FIXTURE: &str = r#"{"dm_id":"dm-fff507b331171652f9c5ff4e5ed80847ea782a23f146c5d70e8f4acf2ab87778","message_id":"msg-1","sender":"79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798","recipient":"c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5","acked_at":1700000100,"signature":"dd70ef0f943f357e63c53d70c9fad51f12f9e5c7a3b7784f911a05c13dd8ac60e215e02ba118886bac52b6683549836a50446e8b41e6dbff2570b6c9f44eeac3"}"#;

/// SECRET_A が SECRET_B 宛に text="golden frame" を暗号化した frame
/// (created_at=1700000050)。HKDF salt/info・AAD・ECDH・XChaCha20Poly1305 の
/// 全経路の互換をこの fixture の復号成功が凍結する。
const FRAME_FIXTURE: &str = r#"{"dm_id":"dm-fff507b331171652f9c5ff4e5ed80847ea782a23f146c5d70e8f4acf2ab87778","message_id":"msg-1","sender":"79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798","recipient":"c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5","created_at":1700000050,"nonce_hex":"0a06116e88df8a2028cb2ea0157da6f60a632f122002dd30","ciphertext_hex":"5ed68f75beae10b78e1a2f3025c0b055e8eff8b9375a7465022747c546e1720d71f91df098d236741058ea48bfd9cffefb33840dfc79a13213bce75ae3b529d4f1cf4ecc640152b6133fa5aa61e88afcf3c9","signature":"6944b2b988bb2c799a4a87751dc6b3ab15d009e8efbb442194dd1e9822c1b93d030ff49181848627edadaad6deb23d566674d2633fec01d043e199cfad543408"}"#;

fn keys_a() -> KukuriKeys {
    KukuriKeys::parse(SECRET_A).expect("fixed secret A parses")
}

fn keys_b() -> KukuriKeys {
    KukuriKeys::parse(SECRET_B).expect("fixed secret B parses")
}

fn demo_tags() -> Vec<Vec<String>> {
    vec![
        vec!["topic".to_string(), "kukuri:topic:demo".to_string()],
        vec!["e".to_string(), "abc".to_string()],
    ]
}

#[test]
fn envelope_canonical_payload_matches_golden() {
    let canonical = canonical_envelope_payload(
        PUBKEY_A,
        1_700_000_000,
        "kukuri:post",
        &demo_tags(),
        "hello \"golden\" \\ 日本語",
    )
    .expect("canonical payload");
    // 期待値は生リテラル(実装関数を期待値側に使わない)。エスケープ表現・
    // 数値表記・要素順・先頭のバージョンリテラル 0 まで含めて凍結する。
    assert_eq!(
        canonical,
        r#"[0,"79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",1700000000,"kukuri:post",[["topic","kukuri:topic:demo"],["e","abc"]],"hello \"golden\" \\ 日本語"]"#
    );
}

#[test]
fn envelope_signed_with_fixed_key_has_golden_id_and_verifies() {
    let envelope = sign_envelope_at(
        &keys_a(),
        "kukuri:post",
        demo_tags(),
        "hello \"golden\" \\ 日本語".to_string(),
        1_700_000_000,
    )
    .expect("sign envelope");
    assert_eq!(envelope.pubkey.as_str(), PUBKEY_A);
    assert_eq!(
        envelope.id.as_str(),
        "029c604938bd3b122a84287be036a1b01035733358eacb892267a06579a8daa5"
    );
    envelope.verify().expect("freshly signed envelope verifies");
}

#[test]
fn envelope_fixture_still_verifies_and_reserializes_identically() {
    let envelope: KukuriEnvelope = serde_json::from_str(ENVELOPE_FIXTURE).expect("fixture parses");
    envelope
        .verify()
        .expect("historically signed envelope must keep verifying");
    // KukuriEnvelope の serde 表現(フィールド名・順序)も wire/storage 契約。
    assert_eq!(
        serde_json::to_string(&envelope).expect("serialize"),
        ENVELOPE_FIXTURE
    );
}

#[test]
fn dm_identifiers_for_fixed_keys_match_golden() {
    let a = keys_a();
    let b = keys_b();
    assert_eq!(
        direct_message_id_for_participants(&a.public_key(), &b.public_key()),
        DM_ID
    );
    // 参加者ソートにより方向非依存であることも凍結対象。
    assert_eq!(
        direct_message_id_for_participants(&b.public_key(), &a.public_key()),
        DM_ID
    );
    let topic_ab = derive_direct_message_topic(&a, &b.public_key()).expect("topic a->b");
    let topic_ba = derive_direct_message_topic(&b, &a.public_key()).expect("topic b->a");
    assert_eq!(topic_ab.as_str(), DM_TOPIC);
    assert_eq!(topic_ba.as_str(), DM_TOPIC);
}

#[test]
fn dm_ack_canonical_matches_golden() {
    let ack = DirectMessageAckV1 {
        dm_id: DM_ID.to_string(),
        message_id: "msg-1".to_string(),
        sender: Pubkey::from(PUBKEY_A),
        recipient: Pubkey::from(PUBKEY_B),
        acked_at: 1_700_000_100,
        signature: String::new(),
    };
    assert_eq!(
        canonical_direct_message_ack_payload(&ack).expect("ack canonical"),
        r#"[0,"dm-fff507b331171652f9c5ff4e5ed80847ea782a23f146c5d70e8f4acf2ab87778","msg-1","79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798","c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",1700000100]"#
    );
}

#[test]
fn dm_ack_built_with_fixed_inputs_verifies() {
    let ack = build_direct_message_ack(
        &keys_a(),
        DM_ID,
        "msg-1",
        &keys_b().public_key(),
        1_700_000_100,
    )
    .expect("build ack");
    assert_eq!(ack.sender.as_str(), PUBKEY_A);
    ack.verify().expect("freshly built ack verifies");
}

#[test]
fn dm_ack_fixture_still_verifies_and_reserializes_identically() {
    let ack: DirectMessageAckV1 = serde_json::from_str(ACK_FIXTURE).expect("fixture parses");
    ack.verify()
        .expect("historically signed ack must keep verifying");
    assert_eq!(serde_json::to_string(&ack).expect("serialize"), ACK_FIXTURE);
}

#[test]
fn dm_frame_canonical_matches_golden() {
    let frame: DirectMessageFrameV1 = serde_json::from_str(FRAME_FIXTURE).expect("fixture parses");
    assert_eq!(
        canonical_direct_message_frame_payload(&frame).expect("frame canonical"),
        r#"[0,"dm-fff507b331171652f9c5ff4e5ed80847ea782a23f146c5d70e8f4acf2ab87778","msg-1","79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798","c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",1700000050,"0a06116e88df8a2028cb2ea0157da6f60a632f122002dd30","5ed68f75beae10b78e1a2f3025c0b055e8eff8b9375a7465022747c546e1720d71f91df098d236741058ea48bfd9cffefb33840dfc79a13213bce75ae3b529d4f1cf4ecc640152b6133fa5aa61e88afcf3c9"]"#
    );
}

#[test]
fn dm_frame_fixture_decrypts_for_recipient_and_reserializes_identically() {
    let frame: DirectMessageFrameV1 = serde_json::from_str(FRAME_FIXTURE).expect("fixture parses");
    // 復号成功 = frame.verify()(canonical + schnorr)+ ECDH 共有秘密 +
    // HKDF(root/frame salt・info)+ AAD 文字列 + XChaCha20Poly1305 の互換が
    // すべて保たれていることの単一証明。
    let payload =
        decrypt_direct_message_frame(&keys_b(), &frame).expect("recipient decrypts fixture");
    assert_eq!(
        payload,
        DirectMessagePayloadV1 {
            text: Some("golden frame".to_string()),
            reply_to: None,
            attachment_manifest: None,
        }
    );
    assert_eq!(
        serde_json::to_string(&frame).expect("serialize"),
        FRAME_FIXTURE
    );
}
