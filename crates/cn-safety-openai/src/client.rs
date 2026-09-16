use std::{sync::Arc, time::Duration};

use base64::Engine;
use kukuri_cn_safety::{ScanError, ScanInputKind};
use tokio::{
    sync::Semaphore,
    time::{Instant, timeout_at},
};

use crate::config::invalid;
use crate::{ModerationAssessment, ModerationBudget, ModerationConfig, ModerationCredentials};

pub enum ModerationInput<'a> {
    Text(&'a str),
    Image(&'a [u8]),
}

pub struct ModerationClient {
    config: ModerationConfig,
    credentials: ModerationCredentials,
    http: reqwest::Client,
    budget: Arc<dyn ModerationBudget>,
    queue: Semaphore,
    in_flight: Semaphore,
}

impl std::fmt::Debug for ModerationClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModerationClient")
            .field("model", &self.config.model)
            .finish_non_exhaustive()
    }
}

impl ModerationClient {
    pub fn new(
        config: ModerationConfig,
        credentials: ModerationCredentials,
        budget: Arc<dyn ModerationBudget>,
    ) -> Result<Self, ScanError> {
        config.validate()?;
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(config.http_timeout)
            .build()
            .map_err(|_| invalid("cannot construct moderation HTTP client"))?;
        Ok(Self {
            queue: Semaphore::new(config.queue_capacity),
            in_flight: Semaphore::new(1),
            config,
            credentials,
            http,
            budget,
        })
    }

    pub fn config(&self) -> &ModerationConfig {
        &self.config
    }

    pub async fn moderate(
        &self,
        input: ModerationInput<'_>,
        deadline: Instant,
    ) -> Result<ModerationAssessment, ScanError> {
        let _queue = self
            .queue
            .try_acquire()
            .map_err(|_| ScanError::Unavailable("moderation queue is full".into()))?;
        timeout_at(deadline, self.moderate_inner(input, deadline))
            .await
            .map_err(|_| ScanError::Timeout("moderation scan deadline exceeded".into()))?
    }

    async fn moderate_inner(
        &self,
        input: ModerationInput<'_>,
        deadline: Instant,
    ) -> Result<ModerationAssessment, ScanError> {
        let (part, kind, tokens) = match input {
            ModerationInput::Text(text) => {
                if text.len() > 40_000 || text.chars().count() > 10_000 {
                    return Err(invalid("moderation text exceeds input limit"));
                }
                // Conservative UTF-8 byte reservation, never a zero-cost request.
                let tokens = u32::try_from(text.len().max(1))
                    .map_err(|_| invalid("text token reservation overflow"))?;
                (
                    serde_json::json!({"type":"text", "text":text}),
                    ScanInputKind::Text,
                    tokens,
                )
            }
            ModerationInput::Image(bytes) => {
                if bytes.len() > 256 * 1024 || !bytes.starts_with(&[0xff, 0xd8, 0xff]) {
                    return Err(invalid(
                        "moderation image must be a bounded normalized JPEG",
                    ));
                }
                let data = base64::engine::general_purpose::STANDARD.encode(bytes);
                (
                    serde_json::json!({"type":"image_url", "image_url":{"url":format!("data:image/jpeg;base64,{data}")}}),
                    ScanInputKind::Image,
                    self.config.image_token_reservation,
                )
            }
        };
        let payload =
            serde_json::to_vec(&serde_json::json!({"model": self.config.model,"input":[part]}))
                .map_err(|_| invalid("cannot encode moderation input"))?;
        if payload.len() > self.config.request_max_bytes {
            return Err(invalid("moderation request is too large"));
        }
        let _in_flight = timeout_at(
            deadline.min(Instant::now() + Duration::from_secs(60)),
            self.in_flight.acquire(),
        )
        .await
        .map_err(|_| ScanError::Timeout("moderation queue wait exceeded".into()))?
        .map_err(|_| ScanError::Unavailable("moderation client is stopping".into()))?;
        for attempt in 0..3u32 {
            loop {
                let delay = self.budget.reserve(tokens, &self.config.budget).await?;
                if delay.is_zero() {
                    break;
                }
                if Instant::now() + delay >= deadline {
                    return Err(ScanError::Unavailable(
                        "moderation budget is exhausted".into(),
                    ));
                }
                tokio::time::sleep(delay).await;
            }
            let response = self
                .http
                .post(format!(
                    "{}/moderations",
                    self.config.api_base_url.trim_end_matches('/')
                ))
                .bearer_auth(self.credentials.expose())
                .header("content-type", "application/json")
                .body(payload.clone())
                .send()
                .await;
            let mut response = match response {
                Ok(response) => response,
                Err(_) if attempt < 2 => {
                    self.backoff(attempt, None, deadline).await?;
                    continue;
                }
                Err(_) => return Err(ScanError::Unavailable("moderation transport failed".into())),
            };
            let status = response.status();
            if status.as_u16() == 429 || status.is_server_error() {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok());
                if attempt < 2 {
                    self.backoff(attempt, retry_after, deadline).await?;
                    continue;
                }
                return Err(ScanError::Unavailable(format!(
                    "moderation HTTP {} after bounded retries",
                    status.as_u16()
                )));
            }
            if !status.is_success() {
                // Never read/format the raw error body; it may echo content or credentials.
                return Err(invalid(&format!("moderation HTTP {}", status.as_u16())));
            }
            if response
                .content_length()
                .is_some_and(|n| n > self.config.response_max_bytes as u64)
            {
                return Err(invalid("moderation response is too large"));
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| invalid("moderation response read failed"))?
            {
                if bytes.len().saturating_add(chunk.len()) > self.config.response_max_bytes {
                    return Err(invalid("moderation response is too large"));
                }
                bytes.extend_from_slice(&chunk);
            }
            return ModerationAssessment::parse(&bytes, kind);
        }
        Err(ScanError::Unavailable(
            "moderation retries exhausted".into(),
        ))
    }

    async fn backoff(
        &self,
        attempt: u32,
        retry_after: Option<u64>,
        deadline: Instant,
    ) -> Result<(), ScanError> {
        // Non-secret jitter avoids synchronized workers; all attempts still reserve the shared budget.
        let jitter = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_millis()
            % 251;
        let delay = Duration::from_secs(retry_after.unwrap_or(1 << attempt))
            .saturating_add(Duration::from_millis(u64::from(jitter)));
        if Instant::now()
            .checked_add(delay)
            .is_none_or(|at| at >= deadline)
        {
            return Err(ScanError::Timeout(
                "moderation retry exceeds scan deadline".into(),
            ));
        }
        tokio::time::sleep(delay).await;
        Ok(())
    }
}
