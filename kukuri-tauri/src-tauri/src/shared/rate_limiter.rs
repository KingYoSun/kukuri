use crate::shared::AppError;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub struct RateLimiter {
    requests: Mutex<HashMap<String, Vec<Instant>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
            max_requests,
            window,
        }
    }

    pub async fn check_and_record(&self, key: &str, message: &str) -> Result<(), AppError> {
        let mut guard = self.requests.lock().await;
        let now = Instant::now();
        let entries = guard.entry(key.to_string()).or_default();
        entries.retain(|instant| now.duration_since(*instant) < self.window);
        if entries.len() >= self.max_requests {
            let retry_after = self
                .window
                .checked_sub(now.duration_since(entries[0]))
                .unwrap_or_default();
            return Err(AppError::rate_limited(
                message,
                retry_after.as_secs().max(1),
            ));
        }
        entries.push(now);
        Ok(())
    }
}
