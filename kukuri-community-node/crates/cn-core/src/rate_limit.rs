use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Default)]
pub struct RateLimiter {
    inner: Mutex<HashMap<String, RateState>>,
}

#[derive(Clone, Copy)]
pub struct RateLimitOutcome {
    pub allowed: bool,
    pub remaining: u64,
    pub retry_after: Option<Duration>,
}

struct RateState {
    window_start: Instant,
    count: u64,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn check(&self, key: &str, limit: u64, window: Duration) -> RateLimitOutcome {
        if limit == 0 {
            return RateLimitOutcome {
                allowed: false,
                remaining: 0,
                retry_after: Some(window),
            };
        }

        let mut guard = self.inner.lock().await;
        let entry = guard.entry(key.to_string()).or_insert_with(|| RateState {
            window_start: Instant::now(),
            count: 0,
        });

        let elapsed = entry.window_start.elapsed();
        if elapsed >= window {
            entry.window_start = Instant::now();
            entry.count = 0;
        }

        entry.count += 1;
        if entry.count > limit {
            let retry_after = window.saturating_sub(entry.window_start.elapsed());
            return RateLimitOutcome {
                allowed: false,
                remaining: 0,
                retry_after: Some(retry_after),
            };
        }

        RateLimitOutcome {
            allowed: true,
            remaining: limit.saturating_sub(entry.count),
            retry_after: None,
        }
    }
}
