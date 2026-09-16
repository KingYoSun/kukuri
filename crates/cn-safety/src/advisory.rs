//! content advisory（ADR 0028 §8.6 / ADR 0025 §7.2）。
//!
//! nsfw / objectionable の suspected を `Allow` で index したとき、index entry に同梱して client へ
//! 配信する node-local なラベル。署名済み `content_labels` とは別欄であり、投稿の canonical でも
//! 署名対象でもない。issuer node の判定であって「network 全体が判定した」ものではない
//! （ADR 0027 §2.8 の説明可能性のため issuer / basis / confidence / signal_id を必ず伴う）。

use serde::{Deserialize, Serialize};

use crate::verdict::{Basis, SafetyCategory};

/// advisory の対象種別（post 本文 / 参照 blob）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AdvisorySubjectKind {
    PostId,
    BlobCid,
}

/// content advisory 1 件（wire 要素）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct ContentAdvisory {
    /// 発行 node（署名鍵の x-only 公開鍵 hex = manifest `node_id`）。
    pub issuer_node_id: String,
    pub subject_kind: AdvisorySubjectKind,
    /// post id または blob hash。
    pub subject_id: String,
    pub category: SafetyCategory,
    /// client 表示語彙（nsfw → `adult`、objectionable → `sensitive`）。
    pub label: String,
    /// classifier confidence（0-100）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<u8>,
    /// 対応する risk signal id（appeal の入口）。
    pub signal_id: String,
    /// 常に `classifier_score`（confirmed へ昇格しない）。
    pub basis: Basis,
}
