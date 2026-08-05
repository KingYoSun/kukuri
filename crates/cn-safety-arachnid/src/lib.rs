//! Project Arachnid Shield provider（#391）。
//!
//! `kukuri-cn-safety` の `SafetyProvider` trait の本番実装。Project Arachnid Shield API
//! （<https://shield.projectarachnid.com>）を叩き、既知 CSAM / harmful-abusive material への
//! Exact / Near Match を domain verdict の入力（`ProviderScanResult`）に写像する。
//!
//! 設計の真実源:
//! - Issue #391（operator-owned credentials / Match Data handling / No Known Match semantics）
//! - `docs/adr/0027-deterministic-moderation-critical-safety.md`
//!
//! この crate が守る不変条件:
//! - **operator-owned credentials**: API credentials は各 Community Node operator が自分で取得し
//!   env（既定: `PROJECT_ARACHNID_API_USERNAME` / `PROJECT_ARACHNID_API_PASSWORD`）で与える。
//!   credentials の値を `Debug` / `Display` / error message / log に出さない。
//! - **Match Data 非保持**: Shield 応答のうち scan 判定に使うのは `classification` /
//!   `match_type` のみ。`near_match_details`・一致先 sha1/sha256・timestamp は応答型に
//!   持たない（deserialize されず、保存・転送・log 出力の経路が構造的に存在しない）。
//! - **fail-closed**: HTTP エラー・timeout・想定外応答はすべて `ScanError` で返し、呼び出し側
//!   （orchestrator）が `Failed` / `Unavailable` に写像して index を止める。`allow` に落ちる
//!   写像は存在しない。
//! - **No Known Match ≠ safe**: `no-known-match` は `ScanOutcome::NoKnownMatch`（safe の証明では
//!   ない）にのみ写像する。「安全確認済み」相当の命名・写像を持たない。

pub mod client;
pub mod config;
pub mod provider;

pub use client::{
    ShieldClassification, ShieldClient, ShieldError, ShieldMatchType, ShieldScanResult,
};
pub use config::{
    API_BASE_URL_ENV, API_TIMEOUT_SECS_ENV, DEFAULT_API_BASE_URL, DEFAULT_API_PASSWORD_ENV,
    DEFAULT_API_USERNAME_ENV, ShieldConfigError, ShieldCredentials, ShieldProviderConfig,
};
pub use provider::{PROVIDER_NAME, ProjectArachnidShieldProvider};

/// 疎通確認に使う合成 PDQ hash（Shield API の期待形式 = 32 bytes の base64）。
///
/// 実画像から計算したものではない固定値（2026-07-02 に実 API で形式を確認済み）。
/// `cn-operator safety test-provider` と `cn-cli readiness` の疎通確認が共有する。
pub const SYNTHETIC_PROBE_PDQ_HASH: &str = "AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA=";
