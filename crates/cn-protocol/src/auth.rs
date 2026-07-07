//! 認証封筒(client 側)の構築。検証・トークン発行はサーバ側(cn-core)に残る。

use anyhow::{Context, Result};
use serde_json::Value;

use kukuri_core::{KukuriAuthEnvelopeContentV1, KukuriKeys, sign_envelope_json};

use crate::normalize::normalize_http_url;

/// 認証封筒の kind(cn endpoint contract の一部)。
pub const AUTH_ENVELOPE_KIND: &str = "auth";

pub fn build_auth_envelope_json(
    keys: &KukuriKeys,
    challenge: &str,
    public_base_url: &str,
) -> Result<Value> {
    let signed = sign_envelope_json(
        keys,
        AUTH_ENVELOPE_KIND,
        vec![
            vec!["challenge".into(), challenge.to_string()],
            vec![
                "capability_url".into(),
                normalize_http_url(public_base_url)?,
            ],
        ],
        &KukuriAuthEnvelopeContentV1 {
            scope: "community-node-auth".into(),
        },
    )?;
    serde_json::to_value(signed).context("failed to encode auth envelope json")
}
