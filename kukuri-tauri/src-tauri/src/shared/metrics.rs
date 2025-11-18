use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const UNSET_TS: u64 = 0;

#[derive(Debug)]
pub struct AtomicMetric {
    success: AtomicU64,
    failure: AtomicU64,
    last_success_ms: AtomicU64,
    last_failure_ms: AtomicU64,
}

#[derive(Debug, Clone, Copy)]
pub struct AtomicSnapshot {
    pub successes: u64,
    pub failures: u64,
    pub last_success_ms: Option<u64>,
    pub last_failure_ms: Option<u64>,
}

impl AtomicMetric {
    pub const fn new() -> Self {
        Self {
            success: AtomicU64::new(0),
            failure: AtomicU64::new(0),
            last_success_ms: AtomicU64::new(UNSET_TS),
            last_failure_ms: AtomicU64::new(UNSET_TS),
        }
    }

    pub fn record_success(&self) {
        self.success.fetch_add(1, Ordering::Relaxed);
        self.last_success_ms
            .store(current_unix_ms(), Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        self.failure.fetch_add(1, Ordering::Relaxed);
        self.last_failure_ms
            .store(current_unix_ms(), Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> AtomicSnapshot {
        AtomicSnapshot {
            successes: self.success.load(Ordering::Relaxed),
            failures: self.failure.load(Ordering::Relaxed),
            last_success_ms: timestamp_to_option(self.last_success_ms.load(Ordering::Relaxed)),
            last_failure_ms: timestamp_to_option(self.last_failure_ms.load(Ordering::Relaxed)),
        }
    }

    pub fn reset(&self) {
        self.success.store(0, Ordering::Relaxed);
        self.failure.store(0, Ordering::Relaxed);
        self.last_success_ms.store(UNSET_TS, Ordering::Relaxed);
        self.last_failure_ms.store(UNSET_TS, Ordering::Relaxed);
    }
}

#[inline]
pub fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(UNSET_TS)
}

#[inline]
pub fn timestamp_to_option(value: u64) -> Option<u64> {
    if value == UNSET_TS { None } else { Some(value) }
}
