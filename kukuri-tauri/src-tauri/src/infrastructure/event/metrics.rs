use crate::shared::error::AppError;
use serde::Serialize;
#[cfg(test)]
use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(test)]
use std::sync::{Mutex, OnceLock};
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

    fn snapshot(&self) -> GatewayMetricSnapshot {
        GatewayMetricSnapshot {
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

static INCOMING_EVENTS: AtomicMetric = AtomicMetric::new();
static PUBLISH_TEXT: AtomicMetric = AtomicMetric::new();
static PUBLISH_TOPIC: AtomicMetric = AtomicMetric::new();
static SEND_REACTION: AtomicMetric = AtomicMetric::new();
static UPDATE_METADATA: AtomicMetric = AtomicMetric::new();
static DELETE_EVENTS: AtomicMetric = AtomicMetric::new();
static DISCONNECT: AtomicMetric = AtomicMetric::new();
static REPOST_EVENTS: AtomicMetric = AtomicMetric::new();

#[cfg(test)]
static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
#[cfg(test)]
thread_local! {
    static LOCK_DEPTH: Cell<u32> = Cell::new(0);
}

#[cfg(test)]
pub(crate) struct MetricsGuard {
    guard: Option<std::sync::MutexGuard<'static, ()>>,
}

#[cfg(test)]
impl Drop for MetricsGuard {
    fn drop(&mut self) {
        LOCK_DEPTH.with(|depth| {
            let current = depth.get();
            depth.set(current.saturating_sub(1));
        });
    }
}

#[cfg(test)]
fn lock_guard() -> MetricsGuard {
    LOCK_DEPTH.with(|depth| {
        let current = depth.get();
        depth.set(current + 1);
        if current == 0 {
            MetricsGuard {
                guard: Some(
                    TEST_LOCK
                        .get_or_init(|| Mutex::new(()))
                        .lock()
                        .expect("event metrics lock"),
                ),
            }
        } else {
            MetricsGuard { guard: None }
        }
    })
}

#[cfg(not(test))]
struct MetricsGuard;

#[cfg(not(test))]
fn lock_guard() -> MetricsGuard {
    MetricsGuard
}

#[cfg(test)]
pub(crate) fn test_guard() -> MetricsGuard {
    lock_guard()
}

#[derive(Debug, Clone, Copy)]
pub enum GatewayMetricKind {
    Incoming,
    PublishTextNote,
    PublishTopicPost,
    Reaction,
    MetadataUpdate,
    DeleteEvents,
    Disconnect,
    Repost,
}

fn metric(kind: GatewayMetricKind) -> &'static AtomicMetric {
    match kind {
        GatewayMetricKind::Incoming => &INCOMING_EVENTS,
        GatewayMetricKind::PublishTextNote => &PUBLISH_TEXT,
        GatewayMetricKind::PublishTopicPost => &PUBLISH_TOPIC,
        GatewayMetricKind::Reaction => &SEND_REACTION,
        GatewayMetricKind::MetadataUpdate => &UPDATE_METADATA,
        GatewayMetricKind::DeleteEvents => &DELETE_EVENTS,
        GatewayMetricKind::Disconnect => &DISCONNECT,
        GatewayMetricKind::Repost => &REPOST_EVENTS,
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GatewayMetricSnapshot {
    pub total: u64,
    pub failures: u64,
    pub last_success_ms: Option<u64>,
    pub last_failure_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EventGatewayMetrics {
    pub incoming: GatewayMetricSnapshot,
    pub publish_text_note: GatewayMetricSnapshot,
    pub publish_topic_post: GatewayMetricSnapshot,
    pub reactions: GatewayMetricSnapshot,
    pub metadata_updates: GatewayMetricSnapshot,
    pub deletions: GatewayMetricSnapshot,
    pub disconnects: GatewayMetricSnapshot,
    pub reposts: GatewayMetricSnapshot,
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(UNSET_TS)
}

fn to_option(value: u64) -> Option<u64> {
    if value == UNSET_TS { None } else { Some(value) }
}

pub fn record_success(kind: GatewayMetricKind) {
    let _guard = lock_guard();
    metric(kind).record_success();
}

pub fn record_failure(kind: GatewayMetricKind) {
    let _guard = lock_guard();
    metric(kind).record_failure();
}

pub fn record_outcome<T>(
    result: Result<T, AppError>,
    kind: GatewayMetricKind,
) -> Result<T, AppError> {
    match result {
        Ok(value) => {
            record_success(kind);
            Ok(value)
        }
        Err(err) => {
            record_failure(kind);
            Err(err)
        }
    }
}

pub fn snapshot() -> EventGatewayMetrics {
    let _guard = lock_guard();
    EventGatewayMetrics {
        incoming: INCOMING_EVENTS.snapshot(),
        publish_text_note: PUBLISH_TEXT.snapshot(),
        publish_topic_post: PUBLISH_TOPIC.snapshot(),
        reactions: SEND_REACTION.snapshot(),
        metadata_updates: UPDATE_METADATA.snapshot(),
        deletions: DELETE_EVENTS.snapshot(),
        disconnects: DISCONNECT.snapshot(),
        reposts: REPOST_EVENTS.snapshot(),
    }
}

#[cfg(test)]
pub fn reset() {
    let _guard = lock_guard();
    INCOMING_EVENTS.reset();
    PUBLISH_TEXT.reset();
    PUBLISH_TOPIC.reset();
    SEND_REACTION.reset();
    UPDATE_METADATA.reset();
    DELETE_EVENTS.reset();
    DISCONNECT.reset();
    REPOST_EVENTS.reset();
}
