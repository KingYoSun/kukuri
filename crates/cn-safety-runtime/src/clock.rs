//! scan 時刻の供給抽象（#353 段階3b）。
//!
//! `cn-safety` の `route()` は時計を持たず、`scanned_at`（RFC3339 文字列）を呼び出し側が与える。
//! orchestrator も時計依存を持たず、この trait を注入される。これにより本 crate は pure に保たれ、
//! テストでは固定時刻 clock で verdict / event を決定論的に検証できる。
//!
//! 本番の system clock 実装（`std::time` / `chrono` ベース）は本 crate のスコープ外であり、
//! runtime 組み込み段階で別途追加する（別 Issue 化済み）。

/// scan 実行時刻を RFC3339 文字列で返す clock 抽象。
pub trait ScanClock: Send + Sync {
    /// 現在時刻を RFC3339（例: `2026-06-29T09:00:00Z`）で返す。
    fn now_rfc3339(&self) -> String;
}
