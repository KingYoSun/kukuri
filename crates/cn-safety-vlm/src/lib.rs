//! OpenAI-compatible VLM moderation provider（#420）。
//!
//! `kukuri-cn-safety` の `SafetyProvider` trait の非決定論的（VLM-assisted）実装。
//! OpenAI-compatible API（chat / vision）を叩き、classifier 検知と descriptive 検索タグを
//! domain verdict の入力（`ProviderScanResult`）に写像する。self-host / 外部 API を問わない。
//!
//! 設計の真実源:
//! - `docs/adr/0028-nondeterministic-moderation-vlm.md`（#411 / #420）
//! - `docs/adr/0027-deterministic-moderation-critical-safety.md`（共通枠組み）
//! - 先例: `kukuri-cn-safety-arachnid`（#391、operator-owned credentials 方式）
//!
//! この crate が守る不変条件:
//! - **operator-owned endpoint / credentials**: endpoint・model・API key は各 Community Node
//!   operator が env（`COMMUNITY_NODE_VLM_API_BASE_URL` / `COMMUNITY_NODE_VLM_MODEL` /
//!   `COMMUNITY_NODE_VLM_API_KEY`）で与える。kukuri 本体は credentials を同梱・共有しない。
//!   API key の値を `Debug` / `Display` / error message / log に出さない
//!   （self-host の無認証 endpoint では API key 自体を省略できる）。
//! - **basis は常に `ClassifierScore`**: `known_hash_match` を設定する経路が構造的に存在せず、
//!   confirmed（`csam_confirmed`）へ昇格する写像が無い。確信度は score（0-100）として返し、
//!   suspected 判定は policy router の `suspected_threshold` が行う。
//! - **fail-closed**: HTTP エラー・timeout・解析不能応答はすべて `ScanError` で返し、呼び出し側
//!   （orchestrator）が `Failed` / `Unavailable` に写像して index を止める。`allow` に落ちる
//!   写像は存在しない。
//! - **no permanent blob storage**: media は `media_hint` から scan のために一時 fetch する
//!   のみで、provider は bytes を保持しない。
//! - **タグの遮断**: critical（CSAM / CSE / grooming）を検知した scan では `derived_tags` を
//!   空にする（収集側 `derived_tags_for_index` のガードと合わせた二重防御）。

pub mod client;
pub mod config;
pub mod provider;

pub use client::{VlmClient, VlmDetection, VlmError, VlmModeration, VlmScanInput};
pub use config::{
    API_BASE_URL_ENV, API_TIMEOUT_SECS_ENV, DEFAULT_API_KEY_ENV, MODEL_ENV, RESPONSE_FORMAT_ENV,
    VlmConfigError, VlmCredentials, VlmProviderConfig, VlmResponseFormat,
};
pub use provider::{CapabilityProfile, PROVIDER_NAME, VlmModerationProvider};
