use crate::shared::config::BootstrapSource;
use crate::shared::validation::ValidationFailureKind;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
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
static RECEIVE_FAILURES_BY_REASON: Lazy<Mutex<HashMap<ValidationFailureKind, u64>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static MAINLINE_CONNECTION_METRIC: AtomicMetric = AtomicMetric::new();
static MAINLINE_ROUTING_METRIC: AtomicMetric = AtomicMetric::new();
static MAINLINE_RECONNECT_METRIC: AtomicMetric = AtomicMetric::new();
static MAINLINE_CONNECTED_PEERS: AtomicU64 = AtomicU64::new(0);
static BOOTSTRAP_ENV_COUNT: AtomicU64 = AtomicU64::new(0);
static BOOTSTRAP_USER_COUNT: AtomicU64 = AtomicU64::new(0);
static BOOTSTRAP_BUNDLE_COUNT: AtomicU64 = AtomicU64::new(0);
static BOOTSTRAP_FALLBACK_COUNT: AtomicU64 = AtomicU64::new(0);
static BOOTSTRAP_LAST_SOURCE: AtomicU64 = AtomicU64::new(BootstrapSource::None as u8 as u64);
static BOOTSTRAP_LAST_MS: AtomicU64 = AtomicU64::new(UNSET_TS);

#[derive(Debug, Clone, Serialize)]
pub struct GossipMetricDetails {
    pub total: u64,
    pub failures: u64,
    pub last_success_ms: Option<u64>,
    pub last_failure_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReceiveFailureBreakdown {
    pub reason: String,
    pub count: u64,
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
    pub receive_failures_by_reason: Vec<ReceiveFailureBreakdown>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MainlineMetricsSnapshot {
    pub connected_peers: u64,
    pub connection_attempts: u64,
    pub connection_successes: u64,
    pub connection_failures: u64,
    pub connection_last_success_ms: Option<u64>,
    pub connection_last_failure_ms: Option<u64>,
    pub routing_attempts: u64,
    pub routing_successes: u64,
    pub routing_failures: u64,
    pub routing_success_rate: f64,
    pub routing_last_success_ms: Option<u64>,
    pub routing_last_failure_ms: Option<u64>,
    pub reconnect_attempts: u64,
    pub reconnect_successes: u64,
    pub reconnect_failures: u64,
    pub last_reconnect_success_ms: Option<u64>,
    pub last_reconnect_failure_ms: Option<u64>,
    pub bootstrap: BootstrapMetricsSnapshot,
}

#[derive(Debug, Clone, Serialize)]
pub struct P2PMetricsSnapshot {
    pub gossip: GossipMetricsSnapshot,
    pub mainline: MainlineMetricsSnapshot,
}

#[derive(Debug, Clone, Serialize)]
pub struct BootstrapMetricsSnapshot {
    pub env_uses: u64,
    pub user_uses: u64,
    pub bundle_uses: u64,
    pub fallback_uses: u64,
    pub last_source: Option<String>,
    pub last_applied_ms: Option<u64>,
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
    if value == UNSET_TS { None } else { Some(value) }
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
    record_receive_failure_with_reason(ValidationFailureKind::Generic);
}

pub fn record_receive_failure_with_reason(kind: ValidationFailureKind) {
    RECEIVE_METRIC.record_failure();
    if let Ok(mut map) = RECEIVE_FAILURES_BY_REASON.lock() {
        *map.entry(kind).or_insert(0) += 1;
    }
}

pub fn record_mainline_connection_success() {
    MAINLINE_CONNECTION_METRIC.record_success();
}

pub fn record_mainline_connection_failure() {
    MAINLINE_CONNECTION_METRIC.record_failure();
}

pub fn set_mainline_connected_peers(count: u64) {
    MAINLINE_CONNECTED_PEERS.store(count, Ordering::Relaxed);
}

pub fn record_mainline_route_success() {
    MAINLINE_ROUTING_METRIC.record_success();
}

pub fn record_mainline_route_failure() {
    MAINLINE_ROUTING_METRIC.record_failure();
}

pub fn record_mainline_reconnect_success() {
    MAINLINE_RECONNECT_METRIC.record_success();
}

pub fn record_mainline_reconnect_failure() {
    MAINLINE_RECONNECT_METRIC.record_failure();
}

pub fn record_bootstrap_source(source: BootstrapSource) {
    match source {
        BootstrapSource::Env => {
            BOOTSTRAP_ENV_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        BootstrapSource::User => {
            BOOTSTRAP_USER_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        BootstrapSource::Bundle => {
            BOOTSTRAP_BUNDLE_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        BootstrapSource::Fallback => {
            BOOTSTRAP_FALLBACK_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        BootstrapSource::None => {}
    }
    BOOTSTRAP_LAST_SOURCE.store(source as u8 as u64, Ordering::Relaxed);
    BOOTSTRAP_LAST_MS.store(current_unix_ms(), Ordering::Relaxed);
}

pub fn reset_all() {
    JOIN_METRIC.reset();
    LEAVE_METRIC.reset();
    BROADCAST_METRIC.reset();
    RECEIVE_METRIC.reset();
    if let Ok(mut map) = RECEIVE_FAILURES_BY_REASON.lock() {
        map.clear();
    }
    MAINLINE_CONNECTION_METRIC.reset();
    MAINLINE_ROUTING_METRIC.reset();
    MAINLINE_RECONNECT_METRIC.reset();
    MAINLINE_CONNECTED_PEERS.store(0, Ordering::Relaxed);
    BOOTSTRAP_ENV_COUNT.store(0, Ordering::Relaxed);
    BOOTSTRAP_USER_COUNT.store(0, Ordering::Relaxed);
    BOOTSTRAP_BUNDLE_COUNT.store(0, Ordering::Relaxed);
    BOOTSTRAP_FALLBACK_COUNT.store(0, Ordering::Relaxed);
    BOOTSTRAP_LAST_SOURCE.store(BootstrapSource::None as u8 as u64, Ordering::Relaxed);
    BOOTSTRAP_LAST_MS.store(UNSET_TS, Ordering::Relaxed);
}

pub fn snapshot() -> GossipMetricsSnapshot {
    let join_details = JOIN_METRIC.snapshot();
    let leave_details = LEAVE_METRIC.snapshot();
    let broadcast_details = BROADCAST_METRIC.snapshot();
    let receive_details = RECEIVE_METRIC.snapshot();
    let receive_failures_by_reason = if let Ok(map) = RECEIVE_FAILURES_BY_REASON.lock() {
        map.iter()
            .map(|(kind, count)| ReceiveFailureBreakdown {
                reason: kind.as_str().to_string(),
                count: *count,
            })
            .collect()
    } else {
        Vec::new()
    };

    GossipMetricsSnapshot {
        joins: join_details.total,
        leaves: leave_details.total,
        broadcasts_sent: broadcast_details.total,
        messages_received: receive_details.total,
        join_details,
        leave_details,
        broadcast_details,
        receive_details,
        receive_failures_by_reason,
    }
}

pub fn mainline_snapshot() -> MainlineMetricsSnapshot {
    let connection_details = MAINLINE_CONNECTION_METRIC.snapshot();
    let routing_details = MAINLINE_ROUTING_METRIC.snapshot();
    let reconnect_details = MAINLINE_RECONNECT_METRIC.snapshot();
    let connected_peers = MAINLINE_CONNECTED_PEERS.load(Ordering::Relaxed);
    let bootstrap = bootstrap_snapshot();

    let connection_attempts = connection_details.total + connection_details.failures;
    let routing_attempts = routing_details.total + routing_details.failures;
    let routing_success_rate = if routing_attempts == 0 {
        0.0
    } else {
        routing_details.total as f64 / routing_attempts as f64
    };
    let reconnect_attempts = reconnect_details.total + reconnect_details.failures;

    MainlineMetricsSnapshot {
        connected_peers,
        connection_attempts,
        connection_successes: connection_details.total,
        connection_failures: connection_details.failures,
        connection_last_success_ms: connection_details.last_success_ms,
        connection_last_failure_ms: connection_details.last_failure_ms,
        routing_attempts,
        routing_successes: routing_details.total,
        routing_failures: routing_details.failures,
        routing_success_rate,
        routing_last_success_ms: routing_details.last_success_ms,
        routing_last_failure_ms: routing_details.last_failure_ms,
        reconnect_attempts,
        reconnect_successes: reconnect_details.total,
        reconnect_failures: reconnect_details.failures,
        last_reconnect_success_ms: reconnect_details.last_success_ms,
        last_reconnect_failure_ms: reconnect_details.last_failure_ms,
        bootstrap,
    }
}

pub fn snapshot_full() -> P2PMetricsSnapshot {
    P2PMetricsSnapshot {
        gossip: snapshot(),
        mainline: mainline_snapshot(),
    }
}

fn bootstrap_snapshot() -> BootstrapMetricsSnapshot {
    let last_source_code = BOOTSTRAP_LAST_SOURCE.load(Ordering::Relaxed);
    let last_source = match last_source_code as u8 {
        x if x == BootstrapSource::Env as u8 => Some("env".to_string()),
        x if x == BootstrapSource::User as u8 => Some("user".to_string()),
        x if x == BootstrapSource::Bundle as u8 => Some("bundle".to_string()),
        x if x == BootstrapSource::Fallback as u8 => Some("fallback".to_string()),
        _ => None,
    };

    BootstrapMetricsSnapshot {
        env_uses: BOOTSTRAP_ENV_COUNT.load(Ordering::Relaxed),
        user_uses: BOOTSTRAP_USER_COUNT.load(Ordering::Relaxed),
        bundle_uses: BOOTSTRAP_BUNDLE_COUNT.load(Ordering::Relaxed),
        fallback_uses: BOOTSTRAP_FALLBACK_COUNT.load(Ordering::Relaxed),
        last_source,
        last_applied_ms: to_option(BOOTSTRAP_LAST_MS.load(Ordering::Relaxed)),
    }
}
