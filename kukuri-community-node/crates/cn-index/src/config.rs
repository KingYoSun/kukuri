use serde::Deserialize;
use serde_json::Value;

#[derive(Clone)]
pub struct IndexRuntimeConfig {
    pub enabled: bool,
    pub consumer_batch_size: i64,
    pub consumer_poll_seconds: u64,
    pub reindex_poll_seconds: u64,
    pub expiration_sweep_seconds: u64,
}

#[derive(Deserialize)]
struct ConsumerSection {
    batch_size: Option<i64>,
    poll_interval_seconds: Option<u64>,
}

#[derive(Deserialize)]
struct ReindexSection {
    poll_interval_seconds: Option<u64>,
}

#[derive(Deserialize)]
struct ExpirationSection {
    sweep_interval_seconds: Option<u64>,
}

impl IndexRuntimeConfig {
    pub fn from_json(value: &Value) -> Self {
        let enabled = value.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
        let consumer = value
            .get("consumer")
            .and_then(|v| serde_json::from_value::<ConsumerSection>(v.clone()).ok());
        let reindex = value
            .get("reindex")
            .and_then(|v| serde_json::from_value::<ReindexSection>(v.clone()).ok());
        let expiration = value
            .get("expiration")
            .and_then(|v| serde_json::from_value::<ExpirationSection>(v.clone()).ok());
        Self {
            enabled,
            consumer_batch_size: consumer
                .as_ref()
                .and_then(|c| c.batch_size)
                .unwrap_or(200),
            consumer_poll_seconds: consumer
                .as_ref()
                .and_then(|c| c.poll_interval_seconds)
                .unwrap_or(5),
            reindex_poll_seconds: reindex
                .as_ref()
                .and_then(|r| r.poll_interval_seconds)
                .unwrap_or(30),
            expiration_sweep_seconds: expiration
                .as_ref()
                .and_then(|e| e.sweep_interval_seconds)
                .unwrap_or(300),
        }
    }
}
