//! signed moderation event（#353 / moderation-event-trust-semantics）。
//!
//! 危険判定・index 除外・quarantine は signed moderation event として記録する。
//! event は issuer node によって署名され、その効果は issuer node の authority scope に限定される。
//! network-wide command ではない（`docs/architecture/moderation-event-trust-semantics.md`）。
//!
//! 署名対象（canonical body）と署名を分離する。
//! - [`ModerationEventBody`] — 未署名の canonical 本体。`canonical_bytes()` で決定論的な
//!   バイト列を得る。
//! - [`SignedModerationEvent`] — body と signature の組。
//! - [`ModerationEventSigner`] — 署名抽象。実鍵署名（secp256k1）は後続段階で実装する。
//!
//! 本 crate には mock signer のみを置き、production credentials に依存しない。

use serde::{Deserialize, Serialize};

use crate::provider::SubjectKind;
use crate::verdict::{Basis, ModerationAction, ReasonCode, SafetyLabel, Severity, Visibility};

/// 未署名の moderation event 本体（署名対象の canonical 表現）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModerationEventBody {
    /// event ID（呼び出し側が採番する）。
    pub id: String,
    /// 発行・署名した node の ID。
    pub issuer_node_id: String,
    pub target_type: SubjectKind,
    pub target_id: String,
    pub action: ModerationAction,
    #[serde(default)]
    pub labels: Vec<SafetyLabel>,
    pub reason_code: ReasonCode,
    pub severity: Severity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<u8>,
    pub basis: Basis,
    pub visibility: Visibility,
    pub policy_version: String,
    /// 作成時刻（RFC3339）。この crate は時計を持たず、呼び出し側が与える。
    pub created_at: String,
}

impl ModerationEventBody {
    /// 署名対象の決定論的バイト列を返す。
    ///
    /// 同一の論理内容に対して常に同じバイト列を返す。[`canonical_json`](Self::canonical_json) の
    /// UTF-8 バイト列であり、object のキーを再帰的に辞書順ソートしてからシリアライズするため、
    /// struct のフィールド宣言順や `serde_json` の map 表現（`preserve_order` feature の有無）に
    /// 依存しない。実鍵署名（secp256k1）の署名対象として、クロス実装・クロスバージョンで安定する。
    pub fn canonical_bytes(&self) -> Vec<u8> {
        self.canonical_json().into_bytes()
    }

    /// 署名対象の決定論的 JSON 文字列。
    ///
    /// object のキーを再帰的に辞書順へ正規化する。`skip_serializing_if` で省略される
    /// `Option::None`（`confidence` 等）は canonical 表現にも現れない。
    pub fn canonical_json(&self) -> String {
        let value = serde_json::to_value(self).expect("moderation event body serializes to JSON");
        let canonical = canonicalize_json_value(value);
        serde_json::to_string(&canonical)
            .expect("canonical moderation event body serializes to JSON")
    }
}

/// JSON object のキーを再帰的に辞書順へ正規化する。
///
/// `serde_json::Map` の既定（`preserve_order` 無効時は BTreeMap）はキーをソートするが、
/// feature unification で `preserve_order` が有効化されても安定するよう、明示的にソート順で
/// 再構築する。array の要素順は意味を持つため保持する。
fn canonicalize_json_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut entries: Vec<(String, serde_json::Value)> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let mut sorted = serde_json::Map::new();
            for (key, child) in entries {
                sorted.insert(key, canonicalize_json_value(child));
            }
            serde_json::Value::Object(sorted)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(canonicalize_json_value).collect())
        }
        other => other,
    }
}

/// 署名済み moderation event。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SignedModerationEvent {
    pub body: ModerationEventBody,
    /// issuer node による署名（表現は signer 実装に依存する文字列）。
    pub signature: String,
}

/// moderation event の署名抽象。
///
/// 実鍵署名（secp256k1）は後続段階で実装する。本 crate は決定論的な mock signer のみ提供する。
pub trait ModerationEventSigner {
    /// 署名者（issuer）の node ID。
    fn issuer_node_id(&self) -> &str;

    /// canonical body に対する署名文字列を返す。
    fn sign(&self, body: &ModerationEventBody) -> String;
}

/// body に署名して [`SignedModerationEvent`] を作る。
pub fn issue_signed_event(
    body: ModerationEventBody,
    signer: &dyn ModerationEventSigner,
) -> SignedModerationEvent {
    let signature = signer.sign(&body);
    SignedModerationEvent { body, signature }
}
