use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OfflinePayload(Value);

impl OfflinePayload {
    pub fn new(value: Value) -> Result<Self, String> {
        Self::validate(&value)?;
        Ok(Self(value))
    }

    pub fn from_json_str(json: &str) -> Result<Self, String> {
        let value: Value =
            serde_json::from_str(json).map_err(|e| format!("Invalid JSON payload: {e}"))?;
        Self::new(value)
    }

    pub fn as_json(&self) -> &Value {
        &self.0
    }

    pub fn into_inner(self) -> Value {
        self.0
    }

    fn validate(value: &Value) -> Result<(), String> {
        if value.is_null() {
            return Err("Offline payload cannot be null".to_string());
        }
        Ok(())
    }
}

impl From<OfflinePayload> for Value {
    fn from(payload: OfflinePayload) -> Self {
        payload.0
    }
}
