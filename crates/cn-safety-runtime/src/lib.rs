//! kukuri community node 向け safety runtime adapter / scan orchestration（#353 段階3b）。
//!
//! `cn-safety` の pure domain（`SafetyProvider` / `SafetyPolicy` / `route()`）を実際に駆動する
//! 境界。登録された provider を逐次実行し、provider 失敗を fail-closed な `ScanOutcome` に写像し、
//! `route()` で `SafetyVerdict` を得て、未署名 moderation artifact（`ModerationEventBody` /
//! `SafetyRiskSignal`）を生成する。
//!
//! この crate は DB / network / production credentials に依存しない。時刻と event id は
//! [`ScanClock`] / [`EventIdGenerator`] として注入される。本番実装として時刻は
//! [`SystemScanClock`]（system clock, UTC RFC3339）、event id は
//! [`UuidEventIdGenerator`]（UUID v4）を提供する。
//!
//! スコープ境界（本 crate に含まないもの）:
//! - 本番 provider 接続（#391 Project Arachnid Shield 等）
//! - moderation event の実鍵署名（secp256k1）と永続化、risk signal の永続化
//! - fail-closed indexing 本体（search / discovery / recommendation 除外の DB 制約）
//! - blob の一時 fetch / moderation server 本体（HTTP）
//!
//! 設計の真実源:
//! - `docs/safety/community-node-critical-safety.md`（§5 component boundary, §6 data flow,
//!   §8 fail-closed invariants, §9 signed events / risk signals）
//! - `docs/architecture/moderation-event-trust-semantics.md`

mod artifacts;
pub mod clock;
pub mod error;
pub mod id;
pub mod orchestrator;

pub use clock::{ScanClock, SystemScanClock};
pub use error::SafetyRuntimeError;
pub use id::{EventIdGenerator, UuidEventIdGenerator};
pub use orchestrator::{
    SafetyOrchestrator, SafetyOrchestratorBuilder, SafetyScanReport, map_scan_error,
};
