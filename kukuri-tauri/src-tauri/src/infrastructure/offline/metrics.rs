use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RetryOutcomeStatus {
    Success,
    Failure,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OfflineRetryMetricsSnapshot {
    pub total_success: u64,
    pub total_failure: u64,
    pub consecutive_failure: u64,
    pub last_success_ms: Option<u64>,
    pub last_failure_ms: Option<u64>,
    pub last_outcome: Option<RetryOutcomeStatus>,
    pub last_job_id: Option<String>,
    pub last_job_reason: Option<String>,
    pub last_trigger: Option<String>,
    pub last_user_pubkey: Option<String>,
    pub last_retry_count: Option<u32>,
    pub last_max_retries: Option<u32>,
    pub last_backoff_ms: Option<u64>,
    pub last_duration_ms: Option<u64>,
    pub last_success_count: Option<u32>,
    pub last_failure_count: Option<u32>,
    pub last_timestamp_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RetryOutcomeMetadata {
    pub job_id: Option<String>,
    pub job_reason: Option<String>,
    pub trigger: Option<String>,
    pub user_pubkey: Option<String>,
    pub retry_count: Option<u32>,
    pub max_retries: Option<u32>,
    pub backoff_ms: Option<u64>,
    pub duration_ms: Option<u64>,
    pub success_count: Option<u32>,
    pub failure_count: Option<u32>,
    pub timestamp_ms: Option<u64>,
}

impl RetryOutcomeMetadata {
    pub fn new() -> Self {
        Self {
            job_id: None,
            job_reason: None,
            trigger: None,
            user_pubkey: None,
            retry_count: None,
            max_retries: None,
            backoff_ms: None,
            duration_ms: None,
            success_count: None,
            failure_count: None,
            timestamp_ms: None,
        }
    }
}

impl Default for RetryOutcomeMetadata {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default, Clone)]
struct LastRetryMetadata {
    last_outcome: Option<RetryOutcomeStatus>,
    job_id: Option<String>,
    job_reason: Option<String>,
    trigger: Option<String>,
    user_pubkey: Option<String>,
    retry_count: Option<u32>,
    max_retries: Option<u32>,
    backoff_ms: Option<u64>,
    duration_ms: Option<u64>,
    success_count: Option<u32>,
    failure_count: Option<u32>,
    timestamp_ms: Option<u64>,
}

struct OfflineRetryMetrics {
    success: AtomicU64,
    failure: AtomicU64,
    consecutive_failure: AtomicU64,
    last_success_ms: AtomicU64,
    last_failure_ms: AtomicU64,
    metadata: Mutex<LastRetryMetadata>,
}

impl OfflineRetryMetrics {
    fn new() -> Self {
        Self {
            success: AtomicU64::new(0),
            failure: AtomicU64::new(0),
            consecutive_failure: AtomicU64::new(0),
            last_success_ms: AtomicU64::new(0),
            last_failure_ms: AtomicU64::new(0),
            metadata: Mutex::new(LastRetryMetadata::default()),
        }
    }

    fn record(&self, status: RetryOutcomeStatus, meta: &RetryOutcomeMetadata) {
        match status {
            RetryOutcomeStatus::Success => {
                self.success.fetch_add(1, Ordering::Relaxed);
                self.last_success_ms
                    .store(current_unix_ms(), Ordering::Relaxed);
                self.consecutive_failure.store(0, Ordering::Relaxed);
            }
            RetryOutcomeStatus::Failure => {
                self.failure.fetch_add(1, Ordering::Relaxed);
                self.last_failure_ms
                    .store(current_unix_ms(), Ordering::Relaxed);
                self.consecutive_failure.fetch_add(1, Ordering::Relaxed);
            }
        }

        if let Ok(mut guard) = self.metadata.lock() {
            guard.last_outcome = Some(status);
            guard.job_id = meta.job_id.clone();
            guard.job_reason = meta.job_reason.clone();
            guard.trigger = meta.trigger.clone();
            guard.user_pubkey = meta.user_pubkey.clone();
            guard.retry_count = meta.retry_count;
            guard.max_retries = meta.max_retries;
            guard.backoff_ms = meta.backoff_ms;
            guard.duration_ms = meta.duration_ms;
            guard.success_count = meta.success_count;
            guard.failure_count = meta.failure_count;
            guard.timestamp_ms = meta.timestamp_ms.or_else(|| Some(current_unix_ms()));
        }
    }

    fn snapshot(&self) -> OfflineRetryMetricsSnapshot {
        let metadata = self
            .metadata
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| LastRetryMetadata::default());

        OfflineRetryMetricsSnapshot {
            total_success: self.success.load(Ordering::Relaxed),
            total_failure: self.failure.load(Ordering::Relaxed),
            consecutive_failure: self.consecutive_failure.load(Ordering::Relaxed),
            last_success_ms: to_option(self.last_success_ms.load(Ordering::Relaxed)),
            last_failure_ms: to_option(self.last_failure_ms.load(Ordering::Relaxed)),
            last_outcome: metadata.last_outcome,
            last_job_id: metadata.job_id,
            last_job_reason: metadata.job_reason,
            last_trigger: metadata.trigger,
            last_user_pubkey: metadata.user_pubkey,
            last_retry_count: metadata.retry_count,
            last_max_retries: metadata.max_retries,
            last_backoff_ms: metadata.backoff_ms,
            last_duration_ms: metadata.duration_ms,
            last_success_count: metadata.success_count,
            last_failure_count: metadata.failure_count,
            last_timestamp_ms: metadata.timestamp_ms,
        }
    }
}

fn to_option(value: u64) -> Option<u64> {
    if value == 0 { None } else { Some(value) }
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

static OFFLINE_RETRY_METRICS: LazyLock<OfflineRetryMetrics> =
    LazyLock::new(OfflineRetryMetrics::new);

pub fn record_outcome(
    status: RetryOutcomeStatus,
    metadata: &RetryOutcomeMetadata,
) -> OfflineRetryMetricsSnapshot {
    OFFLINE_RETRY_METRICS.record(status, metadata);
    OFFLINE_RETRY_METRICS.snapshot()
}

pub fn snapshot() -> OfflineRetryMetricsSnapshot {
    OFFLINE_RETRY_METRICS.snapshot()
}

#[cfg(test)]
mod tests {
    use super::{RetryOutcomeMetadata, RetryOutcomeStatus, snapshot};

    #[test]
    fn record_success_and_failure() {
        let meta = RetryOutcomeMetadata {
            job_id: Some("job-1".into()),
            job_reason: Some("pending-actions".into()),
            trigger: Some("worker".into()),
            retry_count: Some(1),
            max_retries: Some(3),
            backoff_ms: Some(5_000),
            duration_ms: Some(800),
            success_count: Some(2),
            failure_count: Some(0),
            timestamp_ms: Some(1_000),
            user_pubkey: Some("npub".into()),
        };

        super::record_outcome(RetryOutcomeStatus::Success, &meta);

        let snapshot = snapshot();
        assert_eq!(snapshot.total_success, 1);
        assert_eq!(snapshot.total_failure, 0);
        assert_eq!(snapshot.last_outcome, Some(RetryOutcomeStatus::Success));
        assert_eq!(snapshot.last_job_id.as_deref(), Some("job-1"));
        assert_eq!(snapshot.last_backoff_ms, Some(5_000));

        let failure_meta = RetryOutcomeMetadata {
            job_id: Some("job-2".into()),
            failure_count: Some(1),
            ..RetryOutcomeMetadata::default()
        };

        super::record_outcome(RetryOutcomeStatus::Failure, &failure_meta);
        let snapshot = snapshot();
        assert_eq!(snapshot.total_success, 1);
        assert_eq!(snapshot.total_failure, 1);
        assert_eq!(snapshot.last_outcome, Some(RetryOutcomeStatus::Failure));
        assert_eq!(snapshot.last_job_id.as_deref(), Some("job-2"));
    }
}
