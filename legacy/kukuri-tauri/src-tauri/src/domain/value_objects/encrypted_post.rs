use serde::{Deserialize, Serialize};

const ENCRYPTED_POST_SCHEMA: &str = "kukuri-post-cipher-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedPostPayload {
    pub schema: String,
    pub topic: String,
    pub scope: String,
    pub epoch: i64,
    pub payload_b64: String,
}

impl EncryptedPostPayload {
    pub fn new(topic: String, scope: String, epoch: i64, payload_b64: String) -> Self {
        Self {
            schema: ENCRYPTED_POST_SCHEMA.to_string(),
            topic,
            scope,
            epoch,
            payload_b64,
        }
    }

    pub fn schema() -> &'static str {
        ENCRYPTED_POST_SCHEMA
    }

    pub fn try_parse(content: &str) -> Option<Self> {
        let value: Self = serde_json::from_str(content).ok()?;
        if value.schema == ENCRYPTED_POST_SCHEMA {
            Some(value)
        } else {
            None
        }
    }
}
