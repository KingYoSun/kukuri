use serde_json::Value;

#[derive(Clone)]
pub struct LlmRuntimeConfig {
    pub enabled: bool,
    pub provider: String,
    pub external_send_enabled: bool,
    pub truncate_chars: usize,
    pub mask_pii: bool,
    pub max_requests_per_day: i64,
    pub max_cost_per_day: f64,
    pub max_concurrency: usize,
}

impl LlmRuntimeConfig {
    pub fn from_json(value: Option<&Value>) -> Self {
        let value = value.and_then(|v| v.as_object());
        let provider = value
            .and_then(|map| map.get("provider"))
            .and_then(|v| v.as_str())
            .unwrap_or("disabled")
            .to_string();
        Self {
            enabled: value
                .and_then(|map| map.get("enabled"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            provider,
            external_send_enabled: value
                .and_then(|map| map.get("external_send_enabled"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            truncate_chars: value
                .and_then(|map| map.get("truncate_chars"))
                .and_then(|v| v.as_u64())
                .unwrap_or(2000) as usize,
            mask_pii: value
                .and_then(|map| map.get("mask_pii"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            max_requests_per_day: value
                .and_then(|map| map.get("max_requests_per_day"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            max_cost_per_day: value
                .and_then(|map| map.get("max_cost_per_day"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            max_concurrency: value
                .and_then(|map| map.get("max_concurrency"))
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as usize,
        }
    }
}

#[derive(Clone)]
pub struct ModerationRuntimeConfig {
    pub enabled: bool,
    pub consumer_batch_size: i64,
    pub consumer_poll_seconds: u64,
    pub queue_max_attempts: i32,
    pub queue_retry_delay_seconds: i64,
    pub rules_max_labels_per_event: usize,
    pub llm: LlmRuntimeConfig,
}

impl ModerationRuntimeConfig {
    pub fn from_json(value: &Value) -> Self {
        let consumer = value.get("consumer").and_then(|v| v.as_object());
        let queue = value.get("queue").and_then(|v| v.as_object());
        let rules = value.get("rules").and_then(|v| v.as_object());
        Self {
            enabled: value
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            consumer_batch_size: consumer
                .and_then(|map| map.get("batch_size"))
                .and_then(|v| v.as_i64())
                .unwrap_or(200),
            consumer_poll_seconds: consumer
                .and_then(|map| map.get("poll_interval_seconds"))
                .and_then(|v| v.as_u64())
                .unwrap_or(5),
            queue_max_attempts: queue
                .and_then(|map| map.get("max_attempts"))
                .and_then(|v| v.as_i64())
                .unwrap_or(3)
                .clamp(1, 10) as i32,
            queue_retry_delay_seconds: queue
                .and_then(|map| map.get("retry_delay_seconds"))
                .and_then(|v| v.as_i64())
                .unwrap_or(30)
                .max(1),
            rules_max_labels_per_event: rules
                .and_then(|map| map.get("max_labels_per_event"))
                .and_then(|v| v.as_u64())
                .unwrap_or(5)
                .clamp(1, 100) as usize,
            llm: LlmRuntimeConfig::from_json(value.get("llm")),
        }
    }
}
