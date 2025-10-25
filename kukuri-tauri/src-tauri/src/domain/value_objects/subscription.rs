use crate::shared::error::AppError;
use nostr_sdk::prelude::Timestamp;

pub const RESYNC_BACKOFF_SECS: i64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionTarget {
    Topic(String),
    User(String),
}

impl SubscriptionTarget {
    pub fn as_parts(&self) -> (&str, &str) {
        match self {
            SubscriptionTarget::Topic(id) => ("topic", id.as_str()),
            SubscriptionTarget::User(id) => ("user", id.as_str()),
        }
    }

    pub fn from_parts(target_type: &str, target: String) -> Result<Self, AppError> {
        match target_type {
            "topic" => Ok(SubscriptionTarget::Topic(target)),
            "user" => Ok(SubscriptionTarget::User(target)),
            other => Err(AppError::ValidationError(format!(
                "Unknown subscription target type: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionStatus {
    Pending,
    Subscribed,
    NeedsResync,
}

impl SubscriptionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubscriptionStatus::Pending => "pending",
            SubscriptionStatus::Subscribed => "subscribed",
            SubscriptionStatus::NeedsResync => "needs_resync",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(SubscriptionStatus::Pending),
            "subscribed" => Some(SubscriptionStatus::Subscribed),
            "needs_resync" => Some(SubscriptionStatus::NeedsResync),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptionRecord {
    pub target: SubscriptionTarget,
    pub status: SubscriptionStatus,
    pub last_synced_at: Option<i64>,
    pub last_attempt_at: Option<i64>,
    pub failure_count: i64,
    pub error_message: Option<String>,
}

impl SubscriptionRecord {
    pub fn new(target: SubscriptionTarget) -> Self {
        Self {
            target,
            status: SubscriptionStatus::Pending,
            last_synced_at: None,
            last_attempt_at: None,
            failure_count: 0,
            error_message: None,
        }
    }

    pub fn mark_requested(&mut self, attempt_ts: i64) {
        self.status = SubscriptionStatus::Pending;
        self.last_attempt_at = Some(attempt_ts);
        self.error_message = None;
    }

    pub fn mark_subscribed(&mut self, synced_at: i64) {
        self.status = SubscriptionStatus::Subscribed;
        self.last_synced_at = Some(synced_at);
        self.failure_count = 0;
        self.error_message = None;
    }

    pub fn mark_failure(&mut self, attempt_ts: i64, error_message: impl Into<String>) {
        self.status = SubscriptionStatus::NeedsResync;
        self.last_attempt_at = Some(attempt_ts);
        self.failure_count += 1;
        self.error_message = Some(error_message.into());
    }

    pub fn since_timestamp(&self) -> Option<Timestamp> {
        let last_synced = self.last_synced_at?;
        let adjusted = last_synced.saturating_sub(RESYNC_BACKOFF_SECS);
        Some(Timestamp::from(adjusted as u64))
    }
}
