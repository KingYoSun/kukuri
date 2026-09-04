use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const PROTOCOL_VERSION: u32 = 1;
pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;
pub const MAX_TIMEOUT_MS: u64 = 300_000;
pub const DEFAULT_PAGE_LIMIT: u32 = 100;
pub const MAX_PAGE_LIMIT: u32 = 500;

pub mod error_code {
    pub const INVALID_REQUEST: &str = "invalid_request";
    pub const VALIDATION_FAILED: &str = "validation_failed";
    pub const CONSENT_REQUIRED: &str = "consent_required";
    pub const ACTION_REQUIRED: &str = "action_required";
    pub const DAEMON_UNAVAILABLE: &str = "daemon_unavailable";
    pub const PROTOCOL_MISMATCH: &str = "protocol_mismatch";
    pub const PROFILE_IN_USE: &str = "profile_in_use";
    pub const AUTHORIZATION_FAILED: &str = "authorization_failed";
    pub const NOT_FOUND: &str = "not_found";
    pub const CONFLICT: &str = "conflict";
    pub const IDEMPOTENCY_CONFLICT: &str = "idempotency_conflict";
    pub const IDEMPOTENCY_EXPIRED: &str = "idempotency_expired";
    pub const OPERATION_OUTCOME_UNKNOWN: &str = "operation_outcome_unknown";
    pub const NETWORK_UNAVAILABLE: &str = "network_unavailable";
    pub const TIMEOUT: &str = "timeout";
    pub const INTERRUPTED: &str = "interrupted";
    pub const INTERNAL_ERROR: &str = "internal_error";
    pub const BACKPRESSURE: &str = "backpressure";
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    pub protocol_version: u32,
    pub request_id: String,
    pub command: String,
    pub profile: String,
    #[serde(default = "empty_object")]
    pub payload: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_bytes: Option<u64>,
    #[serde(default)]
    pub accepts_secret_output: bool,
}

impl RequestEnvelope {
    pub fn timeout_ms(&self) -> Result<u64, ProtocolError> {
        let timeout = self.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
        if timeout == 0 || timeout > MAX_TIMEOUT_MS {
            return Err(ProtocolError::new(
                error_code::VALIDATION_FAILED,
                format!("timeout_ms must be between 1 and {MAX_TIMEOUT_MS}"),
            ));
        }
        Ok(timeout)
    }
}

fn empty_object() -> Value {
    json!({})
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope {
    pub schema_version: u32,
    pub ok: bool,
    pub command: String,
    pub request_id: String,
    pub profile: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub more: bool,
}

impl ResponseEnvelope {
    pub fn success(request: &RequestEnvelope, data: Value) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            ok: true,
            command: request.command.clone(),
            request_id: request.request_id.clone(),
            profile: request.profile.clone(),
            data: Some(data),
            error: None,
            secret_bytes: None,
            more: false,
        }
    }

    pub fn failure(request: &RequestEnvelope, error: ProtocolError) -> Self {
        Self::failure_parts(
            request.command.clone(),
            request.request_id.clone(),
            request.profile.clone(),
            error,
        )
    }

    pub fn failure_parts(
        command: impl Into<String>,
        request_id: impl Into<String>,
        profile: impl Into<String>,
        error: ProtocolError,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            ok: false,
            command: command.into(),
            request_id: request_id.into(),
            profile: profile.into(),
            data: None,
            error: Some(ErrorBody {
                code: error.code,
                message: error.message,
                details: error.details,
            }),
            secret_bytes: None,
            more: false,
        }
    }

    pub fn error_code(&self) -> Option<&str> {
        self.error.as_ref().map(|error| error.code.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolError {
    pub code: String,
    pub message: String,
    pub details: Option<Value>,
}

impl ProtocolError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ProtocolError {}

pub struct SecretInput(Vec<u8>);

impl SecretInput {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn expose(&self) -> &[u8] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for SecretInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SecretInput")
            .field("bytes", &"<redacted>")
            .finish()
    }
}

pub struct SecretOutput(Vec<u8>);

impl SecretOutput {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for SecretOutput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SecretOutput")
            .field("bytes", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandEffect {
    Read,
    Write,
    Destructive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardRequirement {
    HostReady,
    Account,
    Consent,
    Audience,
    Credential,
    DomainAuthorization,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandMetadata {
    pub name: String,
    pub effect: CommandEffect,
    pub secret_input: bool,
    pub secret_output: bool,
    pub idempotency_required: bool,
    pub streaming: bool,
    pub guards: Vec<GuardRequirement>,
    pub input_schema: Value,
    pub output_schema: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandPage {
    pub items: Vec<CommandMetadata>,
    pub next_cursor: Option<String>,
}

pub fn exit_code_for(error_code: &str) -> i32 {
    match error_code {
        error_code::INVALID_REQUEST | error_code::VALIDATION_FAILED => 2,
        error_code::DAEMON_UNAVAILABLE
        | error_code::PROTOCOL_MISMATCH
        | error_code::PROFILE_IN_USE => 3,
        error_code::CONSENT_REQUIRED
        | error_code::ACTION_REQUIRED
        | error_code::AUTHORIZATION_FAILED => 4,
        error_code::NOT_FOUND
        | error_code::CONFLICT
        | error_code::IDEMPOTENCY_CONFLICT
        | error_code::IDEMPOTENCY_EXPIRED
        | error_code::OPERATION_OUTCOME_UNKNOWN => 5,
        error_code::NETWORK_UNAVAILABLE | error_code::TIMEOUT | error_code::BACKPRESSURE => 6,
        error_code::INTERRUPTED => 130,
        _ => 1,
    }
}

pub fn protocol_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://kukuri.app/schema/cli-protocol-v1.json",
        "title": "Kukuri CLI protocol v1",
        "oneOf": [
            {"$ref": "#/$defs/request"},
            {"$ref": "#/$defs/response"}
        ],
        "$defs": {
            "request": {
                "type": "object",
                "required": ["protocol_version", "request_id", "command", "profile", "payload"],
                "properties": {
                    "protocol_version": {"const": PROTOCOL_VERSION},
                    "request_id": {"type": "string", "minLength": 1},
                    "command": {"type": "string", "minLength": 1},
                    "profile": {"type": "string", "minLength": 1},
                    "payload": {"type": "object"},
                    "idempotency_key": {"type": "string", "format": "uuid"},
                    "timeout_ms": {"type": "integer", "minimum": 1, "maximum": MAX_TIMEOUT_MS},
                    "secret_bytes": {"type": "integer", "minimum": 0, "maximum": MAX_FRAME_BYTES}
                    ,"accepts_secret_output": {"type": "boolean"}
                },
                "additionalProperties": false
            },
            "response": {
                "type": "object",
                "required": ["schema_version", "ok", "command", "request_id", "profile"],
                "properties": {
                    "schema_version": {"const": SCHEMA_VERSION},
                    "ok": {"type": "boolean"},
                    "command": {"type": "string"},
                    "request_id": {"type": "string"},
                    "profile": {"type": "string"},
                    "data": {},
                    "error": {"$ref": "#/$defs/error"},
                    "secret_bytes": {"type": "integer", "minimum": 0, "maximum": MAX_FRAME_BYTES}
                    ,"more": {"type": "boolean"}
                },
                "additionalProperties": false
            },
            "error": {
                "type": "object",
                "required": ["code", "message"],
                "properties": {
                    "code": {"type": "string"},
                    "message": {"type": "string"},
                    "details": {}
                },
                "additionalProperties": false
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_and_error_envelopes_have_stable_shape() {
        let request = RequestEnvelope {
            protocol_version: PROTOCOL_VERSION,
            request_id: "req-1".to_string(),
            command: "client.status".to_string(),
            profile: "default".to_string(),
            payload: json!({}),
            idempotency_key: None,
            timeout_ms: None,
            secret_bytes: None,
            accepts_secret_output: false,
        };
        assert_eq!(
            serde_json::to_value(ResponseEnvelope::success(&request, json!({"ready": true})))
                .expect("success json"),
            json!({
                "schema_version": 1,
                "ok": true,
                "command": "client.status",
                "request_id": "req-1",
                "profile": "default",
                "data": {"ready": true}
            })
        );
        assert_eq!(
            serde_json::to_value(ResponseEnvelope::failure(
                &request,
                ProtocolError::new(error_code::CONSENT_REQUIRED, "consent is required")
            ))
            .expect("error json"),
            json!({
                "schema_version": 1,
                "ok": false,
                "command": "client.status",
                "request_id": "req-1",
                "profile": "default",
                "error": {"code": "consent_required", "message": "consent is required"}
            })
        );
    }

    #[test]
    fn secret_debug_is_always_redacted() {
        let sentinel = "do-not-print-this-secret";
        let input = SecretInput::new(sentinel.as_bytes().to_vec());
        let output = SecretOutput::new(sentinel.as_bytes().to_vec());
        assert!(!format!("{input:?}").contains(sentinel));
        assert!(!format!("{output:?}").contains(sentinel));
    }

    #[test]
    fn exit_codes_are_stable_by_error_class() {
        assert_eq!(exit_code_for(error_code::VALIDATION_FAILED), 2);
        assert_eq!(exit_code_for(error_code::DAEMON_UNAVAILABLE), 3);
        assert_eq!(exit_code_for(error_code::CONSENT_REQUIRED), 4);
        assert_eq!(exit_code_for(error_code::IDEMPOTENCY_CONFLICT), 5);
        assert_eq!(exit_code_for(error_code::TIMEOUT), 6);
        assert_eq!(exit_code_for(error_code::INTERRUPTED), 130);
        assert_eq!(exit_code_for(error_code::INTERNAL_ERROR), 1);
    }

    #[test]
    fn protocol_schema_has_a_golden_digest() {
        let encoded = serde_json::to_vec(&protocol_schema()).expect("schema JSON");
        assert_eq!(
            blake3::hash(&encoded).to_hex().as_str(),
            "65554f6e72dd166cfdb2b8a0a0acd86233efdfff94ccfac89a1c67dac90cec0a",
            "protocol v1 schema changed without an explicit version decision"
        );
    }
}
