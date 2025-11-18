use crate::shared::error::AppError;
use crate::shared::metrics::{AtomicMetric, AtomicSnapshot};
use serde::Serialize;
#[cfg(test)]
use std::cell::Cell;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

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

fn snapshot_from_atomic(metric: &AtomicMetric) -> GatewayMetricSnapshot {
    let snapshot = metric.snapshot();
    GatewayMetricSnapshot {
        total: snapshot.successes,
        failures: snapshot.failures,
        last_success_ms: snapshot.last_success_ms,
        last_failure_ms: snapshot.last_failure_ms,
    }
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
        incoming: snapshot_from_atomic(&INCOMING_EVENTS),
        publish_text_note: snapshot_from_atomic(&PUBLISH_TEXT),
        publish_topic_post: snapshot_from_atomic(&PUBLISH_TOPIC),
        reactions: snapshot_from_atomic(&SEND_REACTION),
        metadata_updates: snapshot_from_atomic(&UPDATE_METADATA),
        deletions: snapshot_from_atomic(&DELETE_EVENTS),
        disconnects: snapshot_from_atomic(&DISCONNECT),
        reposts: snapshot_from_atomic(&REPOST_EVENTS),
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
