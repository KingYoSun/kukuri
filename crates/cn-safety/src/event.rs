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
    /// 同一の論理内容に対して常に同じバイト列を返す。フィールド順序が固定された struct を
    /// `serde_json` で文字列化する（`serde_json` は struct を宣言順でシリアライズし、map を
    /// 使わないため出力は決定論的）。crate 内テストで安定性を固定する。
    ///
    /// 注意: 本実装は同一バージョンでの決定性を担保する。クロス実装で厳密に一致させる
    /// canonical 形式（フィールドのソート / 正規化）が必要になった段階で、実鍵署名導入
    /// （後続）と合わせて強化する。
    pub fn canonical_bytes(&self) -> Vec<u8> {
        self.canonical_json().into_bytes()
    }

    /// 署名対象の決定論的 JSON 文字列。
    pub fn canonical_json(&self) -> String {
        serde_json::to_string(self).expect("moderation event body serializes to JSON")
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
