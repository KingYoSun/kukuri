//! scan 時刻の供給抽象と本番実装（#353 段階3b / #398）。
//!
//! `cn-safety` の `route()` は時計を持たず、`scanned_at`（RFC3339 文字列）を呼び出し側が与える。
//! orchestrator も時計依存を持たず、この trait を注入される。これによりテストでは固定時刻 clock で
//! verdict / event を決定論的に検証できる。
//!
//! 本番では [`SystemScanClock`] を `Arc<dyn ScanClock>` として注入し、system clock（UTC）から
//! RFC3339 文字列を得る。

use chrono::{SecondsFormat, Utc};

/// scan 実行時刻を RFC3339 文字列で返す clock 抽象。
pub trait ScanClock: Send + Sync {
    /// 現在時刻を RFC3339（例: `2026-06-29T09:00:00Z`）で返す。
    fn now_rfc3339(&self) -> String;
}

/// system clock（UTC）ベースの本番 [`ScanClock`] 実装。
///
/// `now_rfc3339()` は UTC・秒精度・`Z` suffix に正規化した RFC3339 文字列を返す
/// （例: `2026-06-29T09:00:00Z`）。scan / moderation event の監査時刻として秒未満は不要のため、
/// fractional seconds は含めない。
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemScanClock;

impl SystemScanClock {
    /// 新しい `SystemScanClock` を作る。
    pub fn new() -> Self {
        Self
    }
}

impl ScanClock for SystemScanClock {
    fn now_rfc3339(&self) -> String {
        Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
    }
}
