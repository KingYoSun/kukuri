//! デスクトップが送る request body の型(cn endpoint contract の一部)。
//!
//! かつては cn-user-api がハンドラ内に Deserialize 専用で持ち、desktop は
//! `serde_json::json!` で同じ形を手書きしていた(二重持ち)。共有型にすることで
//! フィールド変更が両側にコンパイルエラーとして届く(WP-H3 PR2)。
//!
//! Option フィールドは `skip_serializing_if` で欠落として送る(サーバ側は
//! `#[serde(default)]` で null / 欠落の双方を受理するため互換)。

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthChallengeRequest {
    pub pubkey: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthVerifyRequest {
    pub auth_envelope_json: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addr_hint: Option<String>,
    /// invite mode の community node に参加するための招待コード(#383)。
    /// open / whitelist mode や既存 subscriber では不要。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invite_code: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AcceptConsentsRequest {
    #[serde(default)]
    pub policy_slugs: Vec<String>,
    /// 利用者へ提示した catalog revision。current と不一致なら受諾しない。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_snapshot_revision: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BootstrapHeartbeatRequest {
    pub endpoint_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addr_hint: Option<String>,
}
