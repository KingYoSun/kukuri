//! テスターフィードバック(#802 / ADR 0039)の共有通信契約。

use serde::{Deserialize, Serialize};

/// 各自由記述項目の最大文字数(Unicode コードポイント数。ADR 0039)。
/// client 側の入力検証と server 側の受付検証が同じ値を参照する。
pub const TESTER_FEEDBACK_MAX_CHARS: usize = 2000;

/// `POST /v1/tester-feedback` の要求本文。
///
/// 3 つの自由記述はユーザー入力、`client_version` / `os` は desktop-runtime が
/// 自動付与する(UI に入力させない。ADR 0039)。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CommunityNodeTesterFeedbackRequest {
    /// やろうとしたこと。
    #[serde(default)]
    pub what_attempted: String,
    /// 何が起きたか。
    #[serde(default)]
    pub what_happened: String,
    /// 何が変だと思ったか。
    #[serde(default)]
    pub what_seemed_wrong: String,
    /// 送信元 client のバージョン(自動付与)。
    #[serde(default)]
    pub client_version: String,
    /// 送信元 client の OS(自動付与)。
    #[serde(default)]
    pub os: String,
}

/// `POST /v1/tester-feedback` の成功応答。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct CommunityNodeTesterFeedbackResponse {
    /// 保存されたレポートの ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<String>,
}
