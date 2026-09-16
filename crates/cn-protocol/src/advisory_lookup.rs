//! タイムライン向け content advisory 一括照会の wire contract(#1056 / ADR 0046 §6.3 / ADR 0028 §8.12)。
//!
//! client は可視の post id と添付 blob hash だけを送り、node は **自 node が発行した**
//! nsfw / objectionable の advisory だけを返す。本文・viewer の social graph・閲覧履歴の集約は
//! 送受信しない。応答は相対 trust / relation を含まない。

use serde::{Deserialize, Serialize};

pub use kukuri_cn_safety::ContentAdvisory;

/// 1 request で送れる subject(post id + blob hash の合計)の上限。
pub const ADVISORY_LOOKUP_MAX_SUBJECTS: usize = 200;

/// 要求形式が不正(空・上限超過・識別子の形式不正)なときの安定コード(400)。
pub const INVALID_ADVISORY_LOOKUP_CODE: &str = "INVALID_ADVISORY_LOOKUP";

/// `POST /v1/advisories/lookup` の要求本文。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AdvisoryLookupRequest {
    /// 可視の投稿 id(object id)。
    #[serde(default)]
    pub post_ids: Vec<String>,
    /// 可視の添付 blob hash(64 桁 hex)。
    #[serde(default)]
    pub blob_hashes: Vec<String>,
}

impl AdvisoryLookupRequest {
    pub fn subject_count(&self) -> usize {
        self.post_ids.len() + self.blob_hashes.len()
    }
}

/// `POST /v1/advisories/lookup` の応答本文。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AdvisoryLookupResponse {
    pub advisories: Vec<ContentAdvisory>,
}

/// blob hash として受け付ける形式(64 桁の 16 進)。
pub fn is_advisory_blob_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
