use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const UNSET_TS: u64 = 0;

#[derive(Debug)]
struct AtomicMetric {
    success: AtomicU64,
    failure: AtomicU64,
    last_success_ms: AtomicU64,
    last_failure_ms: AtomicU64,
}

impl AtomicMetric {
    const fn new() -> Self {
        Self {
            success: AtomicU64::new(0),
            failure: AtomicU64::new(0),
            last_success_ms: AtomicU64::new(UNSET_TS),
            last_failure_ms: AtomicU64::new(UNSET_TS),
        }
    }

    fn record_success(&self) {
        self.success.fetch_add(1, Ordering::Relaxed);
        self.last_success_ms
            .store(current_unix_ms(), Ordering::Relaxed);
    }

    fn record_failure(&self) {
        self.failure.fetch_add(1, Ordering::Relaxed);
        self.last_failure_ms
            .store(current_unix_ms(), Ordering::Relaxed);
    }

    fn snapshot(&self) -> GossipMetricDetails {
        GossipMetricDetails {
            total: self.success.load(Ordering::Relaxed),
            failures: self.failure.load(Ordering::Relaxed),
            last_success_ms: to_option(self.last_success_ms.load(Ordering::Relaxed)),
            last_failure_ms: to_option(self.last_failure_ms.load(Ordering::Relaxed)),
        }
    }

    fn reset(&self) {
        self.success.store(0, Ordering::Relaxed);
        self.failure.store(0, Ordering::Relaxed);
        self.last_success_ms.store(UNSET_TS, Ordering::Relaxed);
        self.last_failure_ms.store(UNSET_TS, Ordering::Relaxed);
    }
}

static JOIN_METRIC: AtomicMetric = AtomicMetric::new();
static LEAVE_METRIC: AtomicMetric = AtomicMetric::new();
static BROADCAST_METRIC: AtomicMetric = AtomicMetric::new();
static RECEIVE_METRIC: AtomicMetric = AtomicMetric::new();

#[derive(Debug, Clone, Serialize)]
pub struct GossipMetricDetails {
    pub total: u64,
    pub failures: u64,
    pub last_success_ms: Option<u64>,
    pub last_failure_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GossipMetricsSnapshot {
    pub joins: u64,
    pub leaves: u64,
    pub broadcasts_sent: u64,
    pub messages_received: u64,
    pub join_details: GossipMetricDetails,
    pub leave_details: GossipMetricDetails,
    pub broadcast_details: GossipMetricDetails,
    pub receive_details: GossipMetricDetails,
}

#[inline]
fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(UNSET_TS)
}

#[inline]
fn to_option(value: u64) -> Option<u64> {
    if value == UNSET_TS {
        None
    } else {
        Some(value)
    }
}

pub fn record_join_success() {
    JOIN_METRIC.record_success();
}

pub fn record_join_failure() {
    JOIN_METRIC.record_failure();
}

pub fn record_leave_success() {
    LEAVE_METRIC.record_success();
}

pub fn record_leave_failure() {
    LEAVE_METRIC.record_failure();
}

pub fn record_broadcast_success() {
    BROADCAST_METRIC.record_success();
}

pub fn record_broadcast_failure() {
    BROADCAST_METRIC.record_failure();
}

pub fn record_receive_success() {
    RECEIVE_METRIC.record_success();
}

pub fn record_receive_failure() {
    RECEIVE_METRIC.record_failure();
}

#[allow(dead_code)]
pub fn reset_all() {
    JOIN_METRIC.reset();
    LEAVE_METRIC.reset();
    BROADCAST_METRIC.reset();
    RECEIVE_METRIC.reset();
}

pub fn snapshot() -> GossipMetricsSnapshot {
    let join_details = JOIN_METRIC.snapshot();
    let leave_details = LEAVE_METRIC.snapshot();
    let broadcast_details = BROADCAST_METRIC.snapshot();
    let receive_details = RECEIVE_METRIC.snapshot();

    GossipMetricsSnapshot {
        joins: join_details.total,
        leaves: leave_details.total,
        broadcasts_sent: broadcast_details.total,
        messages_received: receive_details.total,
        join_details,
        leave_details,
        broadcast_details,
        receive_details,
    }
}
