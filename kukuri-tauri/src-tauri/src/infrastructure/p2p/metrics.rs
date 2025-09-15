use std::sync::atomic::{AtomicU64, Ordering};
use serde::Serialize;

static JOIN_COUNT: AtomicU64 = AtomicU64::new(0);
static LEAVE_COUNT: AtomicU64 = AtomicU64::new(0);
static BROADCAST_SENT: AtomicU64 = AtomicU64::new(0);
static RECEIVED_COUNT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize)]
pub struct GossipMetricsSnapshot {
    pub joins: u64,
    pub leaves: u64,
    pub broadcasts_sent: u64,
    pub messages_received: u64,
}

pub fn inc_join() {
    JOIN_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_leave() {
    LEAVE_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_broadcast() {
    BROADCAST_SENT.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_received() {
    RECEIVED_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn snapshot() -> GossipMetricsSnapshot {
    GossipMetricsSnapshot {
        joins: JOIN_COUNT.load(Ordering::Relaxed),
        leaves: LEAVE_COUNT.load(Ordering::Relaxed),
        broadcasts_sent: BROADCAST_SENT.load(Ordering::Relaxed),
        messages_received: RECEIVED_COUNT.load(Ordering::Relaxed),
    }
}

