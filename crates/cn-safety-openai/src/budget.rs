use std::{collections::VecDeque, sync::Mutex, time::Duration};

use async_trait::async_trait;
use kukuri_cn_safety::ScanError;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BudgetConfig {
    pub requests_per_minute: u32,
    pub requests_per_day: u32,
    pub tokens_per_minute: u32,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 400,
            requests_per_day: 8000,
            tokens_per_minute: 8000,
        }
    }
}

impl BudgetConfig {
    pub fn validate(&self) -> Result<(), ScanError> {
        if self.requests_per_minute == 0
            || self.requests_per_day == 0
            || self.tokens_per_minute == 0
            || self.requests_per_minute > 500
            || self.requests_per_day > 10_000
            || self.tokens_per_minute > 10_000
        {
            return Err(crate::config::invalid(
                "moderation budget must fit the configured Tier 1 ceiling",
            ));
        }
        Ok(())
    }
}

/// Atomic admission shared by text, images, video frames, probes and retries.
/// Zero means a reservation was consumed; nonzero means no reservation was made.
/// Production uses the node's persistent implementation, so restart cannot reset the budget.
#[async_trait]
pub trait ModerationBudget: Send + Sync {
    async fn reserve(&self, tokens: u32, limits: &BudgetConfig) -> Result<Duration, ScanError>;
}

#[derive(Default)]
pub struct MemoryModerationBudget {
    reservations: Mutex<VecDeque<(Instant, u32)>>,
}

#[async_trait]
impl ModerationBudget for MemoryModerationBudget {
    async fn reserve(&self, tokens: u32, limits: &BudgetConfig) -> Result<Duration, ScanError> {
        limits.validate()?;
        if tokens == 0 || tokens > limits.tokens_per_minute {
            return Err(crate::config::invalid(
                "moderation input exceeds token reservation budget",
            ));
        }
        let now = Instant::now();
        let mut rows = self.reservations.lock().expect("moderation budget mutex");
        while rows
            .front()
            .is_some_and(|(at, _)| now.duration_since(*at) >= Duration::from_secs(86400))
        {
            rows.pop_front();
        }
        if rows.len() >= limits.requests_per_day as usize {
            return Ok(Duration::from_secs(86400).saturating_sub(now.duration_since(rows[0].0)));
        }
        let recent: Vec<_> = rows
            .iter()
            .filter(|(at, _)| now.duration_since(*at) < Duration::from_secs(60))
            .collect();
        let used: u64 = recent.iter().map(|(_, cost)| u64::from(*cost)).sum();
        if recent.len() >= limits.requests_per_minute as usize
            || used + u64::from(tokens) > u64::from(limits.tokens_per_minute)
        {
            let oldest = recent.first().expect("exhausted minute has reservations").0;
            return Ok(Duration::from_secs(60).saturating_sub(now.duration_since(oldest)));
        }
        rows.push_back((now, tokens));
        Ok(Duration::ZERO)
    }
}
