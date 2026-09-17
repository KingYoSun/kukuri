//! Content-free moderation telemetry shared with the indexer status endpoint.
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModerationMetricsSnapshot {
    pub api_attempts: u64,
    pub scans_completed: u64,
    pub scans_failed: u64,
    pub scan_duration_ms: u64,
    pub video_decode_attempts: u64,
    pub video_duration_ms: u64,
    pub frames_extracted: u64,
    pub decode_duration_ms: u64,
}

#[derive(Debug, Default)]
pub struct ModerationMetrics {
    api_attempts: AtomicU64,
    scans_completed: AtomicU64,
    scans_failed: AtomicU64,
    scan_duration_ms: AtomicU64,
    video_decode_attempts: AtomicU64,
    video_duration_ms: AtomicU64,
    frames_extracted: AtomicU64,
    decode_duration_ms: AtomicU64,
}
impl ModerationMetrics {
    pub fn record_api_attempt(&self) {
        self.api_attempts.fetch_add(1, Ordering::Relaxed);
    }
    pub fn record_scan(&self, success: bool, millis: u64) {
        if success {
            self.scans_completed.fetch_add(1, Ordering::Relaxed);
        } else {
            self.scans_failed.fetch_add(1, Ordering::Relaxed);
        }
        self.scan_duration_ms.fetch_add(millis, Ordering::Relaxed);
    }
    pub fn record_video_decode(&self, duration_ms: u64, frames: u64, decode_ms: u64) {
        self.video_decode_attempts.fetch_add(1, Ordering::Relaxed);
        self.video_duration_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
        self.frames_extracted.fetch_add(frames, Ordering::Relaxed);
        self.decode_duration_ms
            .fetch_add(decode_ms, Ordering::Relaxed);
    }
    pub fn snapshot(&self) -> ModerationMetricsSnapshot {
        ModerationMetricsSnapshot {
            api_attempts: self.api_attempts.load(Ordering::Relaxed),
            scans_completed: self.scans_completed.load(Ordering::Relaxed),
            scans_failed: self.scans_failed.load(Ordering::Relaxed),
            scan_duration_ms: self.scan_duration_ms.load(Ordering::Relaxed),
            video_decode_attempts: self.video_decode_attempts.load(Ordering::Relaxed),
            video_duration_ms: self.video_duration_ms.load(Ordering::Relaxed),
            frames_extracted: self.frames_extracted.load(Ordering::Relaxed),
            decode_duration_ms: self.decode_duration_ms.load(Ordering::Relaxed),
        }
    }
}
