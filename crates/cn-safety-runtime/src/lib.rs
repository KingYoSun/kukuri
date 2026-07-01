//! kukuri community node 向け safety runtime adapter / scan orchestration（#353 段階3b）。
//!
//! `cn-safety` の pure domain（`SafetyProvider` / `SafetyPolicy` / `route()`）を実際に駆動する
//! 境界。登録された provider を逐次実行し、provider 失敗を fail-closed な `ScanOutcome` に写像し、
//! `route()` で `SafetyVerdict` を得て、未署名 moderation artifact（`ModerationEventBody` /
//! `SafetyRiskSignal`）を生成する。
//!
//! この crate は DB / network に依存しない。時刻と event id は
//! [`ScanClock`] / [`EventIdGenerator`] として注入される。本番実装として時刻は
//! [`SystemScanClock`]（system clock, UTC RFC3339）、event id は
//! [`UuidEventIdGenerator`]（UUID v4）を提供する。
//!
//! moderation event の実鍵署名は [`Secp256k1ModerationEventSigner`]（secp256k1 schnorr）として
//! 提供し、`cn-safety` の mock signer を置き換える。署名対象は `sha256(canonical_bytes)`、
//! issuer_node_id は署名鍵の x-only 公開鍵 hex。[`verify_signed_event`] で検証する。署名鍵の
//! 値は呼び出し側（runtime / Secret Manager 注入 env）が供給し、本 crate は鍵 store を持たない。
//!
//! スコープ境界（本 crate に含まないもの）:
//! - 本番 provider 接続（#391 Project Arachnid Shield 等）
//! - signed moderation event / risk signal の永続化（`cn-core` が所有）
//! - fail-closed indexing 本体（search / discovery / recommendation 除外の DB 制約）
//! - blob の一時 fetch / moderation server 本体（HTTP）
//!
//! 設計の真実源:
//! - `docs/adr/0027-deterministic-moderation-critical-safety.md`（§2 component boundary / data flow /
//!   fail-closed invariants / signed events / risk signals、§2.1 advisory ≠ command）

mod artifacts;
pub mod clock;
pub mod error;
pub mod id;
pub mod orchestrator;
pub mod signer;

pub use clock::{ScanClock, SystemScanClock};
pub use error::SafetyRuntimeError;
pub use id::{EventIdGenerator, UuidEventIdGenerator};
pub use orchestrator::{
    SafetyOrchestrator, SafetyOrchestratorBuilder, SafetyScanReport, map_scan_error,
};
pub use signer::{
    SAFETY_SIGNING_KEY_ENV, Secp256k1ModerationEventSigner, SignatureError, SignerKeyError,
    verify_signed_event,
};
