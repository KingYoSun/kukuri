use std::path::PathBuf;

use base64::Engine as _;
use minisign_verify::{PublicKey, Signature};

fn decode_base64_text(value: &str) -> String {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(value.trim())
        .expect("value must be base64");
    String::from_utf8(bytes).expect("decoded value must be UTF-8")
}

#[test]
#[ignore = "run through scripts/release/test-published-updater-signature.ps1"]
fn published_bundle_accepts_only_its_valid_signature() {
    let bundle_path = PathBuf::from(
        std::env::var("KUKURI_UPDATER_BUNDLE").expect("KUKURI_UPDATER_BUNDLE is required"),
    );
    let signature_path = PathBuf::from(
        std::env::var("KUKURI_UPDATER_SIGNATURE")
            .expect("KUKURI_UPDATER_SIGNATURE is required"),
    );
    let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    let config: serde_json::Value = serde_json::from_slice(
        &std::fs::read(config_path).expect("tauri.conf.json must be readable"),
    )
    .expect("tauri.conf.json must be valid JSON");
    let encoded_public_key = config["plugins"]["updater"]["pubkey"]
        .as_str()
        .expect("plugins.updater.pubkey must be configured");
    let encoded_signature = std::fs::read_to_string(signature_path)
        .expect("updater signature fixture must be readable");
    let public_key = PublicKey::decode(&decode_base64_text(encoded_public_key))
        .expect("updater public key must decode");
    let signature = Signature::decode(&decode_base64_text(&encoded_signature))
        .expect("updater signature must decode");
    let bundle = std::fs::read(bundle_path).expect("updater bundle must be readable");

    public_key
        .verify(&bundle, &signature, true)
        .expect("published updater bundle must match its signature");

    let mut tampered = bundle;
    let first = tampered
        .first_mut()
        .expect("published updater bundle must not be empty");
    *first ^= 0x01;
    assert!(
        public_key.verify(&tampered, &signature, true).is_err(),
        "a one-byte modification must invalidate the updater signature"
    );
}
