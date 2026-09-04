use std::sync::Arc;

mod validation;

use async_trait::async_trait;
use kukuri_desktop_runtime::{ClientHost, IdempotencyClaim, IdempotencyLedger, IdempotencyScope};
use serde_json::Value;

use crate::{
    protocol::{
        GuardRequirement, PROTOCOL_VERSION, ProtocolError, RequestEnvelope, ResponseEnvelope,
        SecretInput, error_code,
    },
    registry::{CommandOutput, CommandRegistry, HandlerContext},
};
use validation::{redact_protocol_error, validate_command_output, validate_payload};

pub enum DispatchReply {
    Unary(ResponseEnvelope, Option<crate::protocol::SecretOutput>),
    Events {
        request: RequestEnvelope,
        receiver: kukuri_desktop_runtime::ClientEventReceiver,
    },
}

pub struct Dispatcher {
    registry: CommandRegistry,
    mutation_guard: tokio::sync::Mutex<()>,
    guard_evaluator: Arc<dyn GuardEvaluator>,
}

pub struct GuardContext<'a> {
    pub request: &'a RequestEnvelope,
    pub host: Option<&'a Arc<ClientHost>>,
}

#[async_trait]
pub trait GuardEvaluator: Send + Sync {
    async fn evaluate(
        &self,
        requirement: GuardRequirement,
        context: &GuardContext<'_>,
    ) -> Result<(), ProtocolError>;
}

struct HostGuardEvaluator;

#[async_trait]
impl GuardEvaluator for HostGuardEvaluator {
    async fn evaluate(
        &self,
        requirement: GuardRequirement,
        context: &GuardContext<'_>,
    ) -> Result<(), ProtocolError> {
        match requirement {
            GuardRequirement::HostReady if context.host.is_none() => Err(ProtocolError::new(
                error_code::CONSENT_REQUIRED,
                "application consent is required",
            )),
            GuardRequirement::HostReady => Ok(()),
            _ => Err(ProtocolError::new(
                error_code::ACTION_REQUIRED,
                "the command guard is not available in this protocol version",
            )),
        }
    }
}

impl Dispatcher {
    pub fn builtin() -> Self {
        Self {
            registry: CommandRegistry::builtin(),
            mutation_guard: tokio::sync::Mutex::new(()),
            guard_evaluator: Arc::new(HostGuardEvaluator),
        }
    }

    pub fn new(registry: CommandRegistry) -> Self {
        Self {
            registry,
            mutation_guard: tokio::sync::Mutex::new(()),
            guard_evaluator: Arc::new(HostGuardEvaluator),
        }
    }

    pub fn with_guard_evaluator(mut self, evaluator: Arc<dyn GuardEvaluator>) -> Self {
        self.guard_evaluator = evaluator;
        self
    }

    pub fn registry(&self) -> &CommandRegistry {
        &self.registry
    }

    pub async fn preflight(
        &self,
        request: &RequestEnvelope,
        expected_profile: &str,
        host: Option<&Arc<ClientHost>>,
    ) -> Result<(), ProtocolError> {
        let registration = self.registration_for(request, expected_profile)?;
        if registration.metadata.secret_output && !request.accepts_secret_output {
            return Err(ProtocolError::new(
                error_code::ACTION_REQUIRED,
                "secret response requires an explicit output sink",
            ));
        }
        let context = GuardContext { request, host };
        let secret_bearing =
            registration.metadata.secret_input || registration.metadata.secret_output;
        for requirement in &registration.metadata.guards {
            self.guard_evaluator
                .evaluate(*requirement, &context)
                .await
                .map_err(|error| redact_protocol_error(error, secret_bearing))?;
        }
        Ok(())
    }

    pub async fn dispatch(
        &self,
        request: RequestEnvelope,
        secret: Option<SecretInput>,
        expected_profile: &str,
        host: Option<&Arc<ClientHost>>,
    ) -> DispatchReply {
        match self
            .dispatch_inner(&request, secret, expected_profile, host)
            .await
        {
            Ok(CommandOutput::Unary(data)) => {
                DispatchReply::Unary(ResponseEnvelope::success(&request, data), None)
            }
            Ok(CommandOutput::Secret { data, secret }) => {
                let mut response = ResponseEnvelope::success(&request, data);
                response.secret_bytes = Some(secret.expose().len() as u64);
                DispatchReply::Unary(response, Some(secret))
            }
            Ok(CommandOutput::Events(receiver)) => DispatchReply::Events { request, receiver },
            Err(error) => DispatchReply::Unary(ResponseEnvelope::failure(&request, error), None),
        }
    }

    async fn dispatch_inner(
        &self,
        request: &RequestEnvelope,
        secret: Option<SecretInput>,
        expected_profile: &str,
        host: Option<&Arc<ClientHost>>,
    ) -> Result<CommandOutput, ProtocolError> {
        self.preflight(request, expected_profile, host).await?;
        let registration = self
            .registry
            .get(&request.command)
            .expect("preflight resolved the command");
        if request.secret_bytes.is_some() != secret.is_some() {
            return Err(ProtocolError::new(
                error_code::INVALID_REQUEST,
                "secret frame is missing or unexpected",
            ));
        }
        if !registration.metadata.idempotency_required {
            let result = registration
                .handler
                .execute(
                    HandlerContext {
                        registry: &self.registry,
                        host,
                        profile: expected_profile,
                    },
                    request.payload.clone(),
                    secret.as_ref(),
                )
                .await
                .map_err(|error| {
                    redact_protocol_error(
                        error,
                        registration.metadata.secret_input || registration.metadata.secret_output,
                    )
                })?;
            validate_command_output(&registration.metadata, &result, secret.as_ref())?;
            return Ok(result);
        }

        let _mutation_guard = self.mutation_guard.lock().await;
        let host = host.ok_or_else(|| {
            ProtocolError::new(
                error_code::INTERNAL_ERROR,
                "mutating command requires a ready client host",
            )
        })?;
        let runtime = host.runtime();
        let db_path = runtime.db_path();
        let account = db_path
            .parent()
            .and_then(|path| path.file_name())
            .and_then(|value| value.to_str())
            .unwrap_or("active-account")
            .to_string();
        let ledger = IdempotencyLedger::open(db_path)
            .await
            .map_err(internal_ledger_error)?;
        let canonical_payload = canonical_json(&request.payload)?;
        let payload_hash =
            ledger.digest_payload(&canonical_payload, secret.as_ref().map(SecretInput::expose));
        let key = request
            .idempotency_key
            .as_deref()
            .expect("idempotency requirement checked above");
        let scope = IdempotencyScope {
            profile: expected_profile,
            account: &account,
            command: &request.command,
        };
        match ledger
            .claim(&scope, key, &payload_hash, unix_millis())
            .await
            .map_err(map_claim_error)?
        {
            IdempotencyClaim::Replay(value) => return Ok(CommandOutput::Unary(value)),
            IdempotencyClaim::Conflict => {
                return Err(ProtocolError::new(
                    error_code::IDEMPOTENCY_CONFLICT,
                    "idempotency key was already used with a different payload",
                ));
            }
            IdempotencyClaim::OutcomeUnknown => {
                return Err(ProtocolError::new(
                    error_code::OPERATION_OUTCOME_UNKNOWN,
                    "the previous operation outcome cannot be determined",
                ));
            }
            IdempotencyClaim::Expired => {
                return Err(ProtocolError::new(
                    error_code::IDEMPOTENCY_EXPIRED,
                    "the idempotency key is outside the retained history",
                ));
            }
            IdempotencyClaim::Execute => {}
        }
        let result = registration
            .handler
            .execute(
                HandlerContext {
                    registry: &self.registry,
                    host: Some(host),
                    profile: expected_profile,
                },
                request.payload.clone(),
                secret.as_ref(),
            )
            .await
            .map_err(|error| {
                redact_protocol_error(
                    error,
                    registration.metadata.secret_input || registration.metadata.secret_output,
                )
            });
        if let Ok(output) = &result
            && let Err(error) =
                validate_command_output(&registration.metadata, output, secret.as_ref())
        {
            ledger
                .mark_unknown(&scope, key, unix_millis())
                .await
                .map_err(internal_ledger_error)?;
            return Err(error);
        }
        match result {
            Ok(CommandOutput::Unary(data)) => {
                ledger
                    .complete(&scope, key, &data, unix_millis())
                    .await
                    .map_err(internal_ledger_error)?;
                Ok(CommandOutput::Unary(data))
            }
            Ok(CommandOutput::Secret { .. }) => {
                unreachable!("registry rejects mutating secret output")
            }
            Ok(CommandOutput::Events(_)) => unreachable!("registry rejects streaming mutation"),
            Err(error) => {
                ledger
                    .mark_unknown(&scope, key, unix_millis())
                    .await
                    .map_err(internal_ledger_error)?;
                Err(error)
            }
        }
    }

    fn registration_for<'a>(
        &'a self,
        request: &RequestEnvelope,
        expected_profile: &str,
    ) -> Result<&'a crate::registry::CommandRegistration, ProtocolError> {
        if request.protocol_version != PROTOCOL_VERSION {
            return Err(ProtocolError::new(
                error_code::PROTOCOL_MISMATCH,
                format!(
                    "unsupported protocol version {}; expected {PROTOCOL_VERSION}",
                    request.protocol_version
                ),
            ));
        }
        if request.request_id.trim().is_empty() || request.command.trim().is_empty() {
            return Err(ProtocolError::new(
                error_code::VALIDATION_FAILED,
                "request_id and command must not be empty",
            ));
        }
        if request.profile != expected_profile {
            return Err(ProtocolError::new(
                error_code::AUTHORIZATION_FAILED,
                "request profile does not match the daemon profile",
            ));
        }
        if !request.payload.is_object() {
            return Err(ProtocolError::new(
                error_code::VALIDATION_FAILED,
                "payload must be a JSON object",
            ));
        }
        let registration = self
            .registry
            .get(&request.command)
            .ok_or_else(|| ProtocolError::new(error_code::NOT_FOUND, "unknown command"))?;
        validate_payload(&registration.metadata.input_schema, &request.payload).map_err(
            |error| {
                redact_protocol_error(
                    error,
                    registration.metadata.secret_input || registration.metadata.secret_output,
                )
            },
        )?;
        if request.secret_bytes.is_some() != registration.metadata.secret_input {
            return Err(ProtocolError::new(
                error_code::VALIDATION_FAILED,
                "secret input does not match command metadata",
            ));
        }
        if registration.metadata.idempotency_required && request.idempotency_key.is_none() {
            return Err(ProtocolError::new(
                error_code::VALIDATION_FAILED,
                "this command requires an idempotency_key",
            ));
        }
        Ok(registration)
    }
}

fn canonical_json(value: &Value) -> Result<Vec<u8>, ProtocolError> {
    fn canonicalize(value: &Value) -> Value {
        match value {
            Value::Array(items) => Value::Array(items.iter().map(canonicalize).collect()),
            Value::Object(map) => {
                let mut keys = map.keys().collect::<Vec<_>>();
                keys.sort();
                let mut output = serde_json::Map::new();
                for key in keys {
                    output.insert(key.clone(), canonicalize(&map[key]));
                }
                Value::Object(output)
            }
            scalar => scalar.clone(),
        }
    }
    serde_json::to_vec(&canonicalize(value)).map_err(|_| {
        ProtocolError::new(
            error_code::INVALID_REQUEST,
            "payload could not be canonicalized",
        )
    })
}

fn unix_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX)
}

fn internal_ledger_error(error: anyhow::Error) -> ProtocolError {
    ProtocolError::new(
        error_code::INTERNAL_ERROR,
        format!("idempotency ledger failure: {error:#}"),
    )
}

fn map_claim_error(error: anyhow::Error) -> ProtocolError {
    let message = error.to_string();
    if message.contains("UUIDv7")
        || message.contains("must be a UUID")
        || message.contains("too far in the future")
    {
        ProtocolError::new(error_code::VALIDATION_FAILED, message)
    } else if message.contains("capacity is exhausted") {
        ProtocolError::new(error_code::ACTION_REQUIRED, message)
    } else {
        internal_ledger_error(error)
    }
}

pub fn decode_request(bytes: &[u8]) -> Result<RequestEnvelope, ProtocolError> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| ProtocolError::new(error_code::INVALID_REQUEST, "invalid JSON request"))?;
    serde_json::from_value(value)
        .map_err(|error| ProtocolError::new(error_code::INVALID_REQUEST, error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use kukuri_desktop_runtime::DesktopRuntime;
    use serde_json::json;
    use uuid::Uuid;

    use super::*;
    use crate::{
        protocol::{CommandEffect, CommandMetadata, GuardRequirement},
        registry::{CommandHandler, CommandRegistration},
    };

    fn request(command: &str) -> RequestEnvelope {
        RequestEnvelope {
            protocol_version: PROTOCOL_VERSION,
            request_id: "req-1".to_string(),
            command: command.to_string(),
            profile: "test".to_string(),
            payload: json!({}),
            idempotency_key: None,
            timeout_ms: None,
            secret_bytes: None,
            accepts_secret_output: false,
        }
    }

    #[tokio::test]
    async fn mismatch_and_unknown_command_are_typed_errors() {
        let dispatcher = Dispatcher::builtin();
        let mut mismatch = request("client.status");
        mismatch.protocol_version = 99;
        let DispatchReply::Unary(response, _) =
            dispatcher.dispatch(mismatch, None, "test", None).await
        else {
            panic!("expected unary response")
        };
        assert_eq!(response.error_code(), Some(error_code::PROTOCOL_MISMATCH));

        let DispatchReply::Unary(response, _) = dispatcher
            .dispatch(request("missing.command"), None, "test", None)
            .await
        else {
            panic!("expected unary response")
        };
        assert_eq!(response.error_code(), Some(error_code::NOT_FOUND));
    }

    #[tokio::test]
    async fn status_is_available_before_consent_but_events_are_guarded() {
        let dispatcher = Dispatcher::builtin();
        let DispatchReply::Unary(status, _) = dispatcher
            .dispatch(request("client.status"), None, "test", None)
            .await
        else {
            panic!("expected status")
        };
        assert!(status.ok);
        assert_eq!(status.data.expect("data")["ready"], false);

        let DispatchReply::Unary(events, _) = dispatcher
            .dispatch(request("events.watch"), None, "test", None)
            .await
        else {
            panic!("expected guarded response")
        };
        assert_eq!(events.error_code(), Some(error_code::CONSENT_REQUIRED));
    }

    struct CountingWriteHandler(Arc<AtomicUsize>);

    #[async_trait]
    impl CommandHandler for CountingWriteHandler {
        async fn execute(
            &self,
            _context: crate::registry::HandlerContext<'_>,
            payload: Value,
            _secret: Option<&SecretInput>,
        ) -> Result<CommandOutput, ProtocolError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(CommandOutput::Unary(json!({"saved": payload})))
        }
    }

    struct SecretEchoHandler;

    #[async_trait]
    impl CommandHandler for SecretEchoHandler {
        async fn execute(
            &self,
            _context: crate::registry::HandlerContext<'_>,
            _payload: Value,
            secret: Option<&SecretInput>,
        ) -> Result<CommandOutput, ProtocolError> {
            let bytes = secret.expect("secret input").expose().to_vec();
            Ok(CommandOutput::Secret {
                data: json!({"written": true}),
                secret: crate::protocol::SecretOutput::new(bytes),
            })
        }
    }

    struct SecretErrorHandler;

    #[async_trait]
    impl CommandHandler for SecretErrorHandler {
        async fn execute(
            &self,
            _context: crate::registry::HandlerContext<'_>,
            _payload: Value,
            secret: Option<&SecretInput>,
        ) -> Result<CommandOutput, ProtocolError> {
            let secret =
                std::str::from_utf8(secret.expect("secret input").expose()).expect("fixture UTF-8");
            Err(
                ProtocolError::new(error_code::VALIDATION_FAILED, format!("rejected {secret}"))
                    .with_details(json!({"reason": secret})),
            )
        }
    }

    struct SecretOutputLeakHandler;

    #[async_trait]
    impl CommandHandler for SecretOutputLeakHandler {
        async fn execute(
            &self,
            _context: crate::registry::HandlerContext<'_>,
            _payload: Value,
            _secret: Option<&SecretInput>,
        ) -> Result<CommandOutput, ProtocolError> {
            Ok(CommandOutput::Secret {
                data: json!({"leak": "secret-output-\"-\\-sentinel"}),
                secret: crate::protocol::SecretOutput::new(
                    b"secret-output-\"-\\-sentinel".to_vec(),
                ),
            })
        }
    }

    struct SecretOutputErrorHandler;

    #[async_trait]
    impl CommandHandler for SecretOutputErrorHandler {
        async fn execute(
            &self,
            _context: crate::registry::HandlerContext<'_>,
            _payload: Value,
            _secret: Option<&SecretInput>,
        ) -> Result<CommandOutput, ProtocolError> {
            Err(
                ProtocolError::new(error_code::INTERNAL_ERROR, "secret-output-error-sentinel")
                    .with_details(json!({"leak": "secret-output-error-sentinel"})),
            )
        }
    }

    struct AllowAllGuards;

    #[async_trait]
    impl GuardEvaluator for AllowAllGuards {
        async fn evaluate(
            &self,
            _requirement: GuardRequirement,
            _context: &GuardContext<'_>,
        ) -> Result<(), ProtocolError> {
            Ok(())
        }
    }

    struct LeakingGuard;

    #[async_trait]
    impl GuardEvaluator for LeakingGuard {
        async fn evaluate(
            &self,
            _requirement: GuardRequirement,
            _context: &GuardContext<'_>,
        ) -> Result<(), ProtocolError> {
            Err(
                ProtocolError::new(error_code::AUTHORIZATION_FAILED, "guard-secret-sentinel")
                    .with_details(json!({"credential": "guard-secret-sentinel"})),
            )
        }
    }

    fn write_dispatcher(counter: Arc<AtomicUsize>) -> Dispatcher {
        let metadata = CommandMetadata {
            name: "fixture.write".to_string(),
            effect: CommandEffect::Write,
            secret_input: false,
            secret_output: false,
            idempotency_required: true,
            streaming: false,
            guards: vec![GuardRequirement::HostReady],
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
        };
        Dispatcher::new(
            CommandRegistry::new(vec![CommandRegistration::new(
                metadata,
                Arc::new(CountingWriteHandler(counter)),
            )])
            .expect("registry"),
        )
    }

    #[tokio::test]
    async fn guard_runs_before_ledger_and_mutation_sink() {
        let counter = Arc::new(AtomicUsize::new(0));
        let dispatcher = write_dispatcher(counter.clone());
        let mut request = request("fixture.write");
        request.idempotency_key = Some(Uuid::now_v7().to_string());
        let DispatchReply::Unary(response, _) =
            dispatcher.dispatch(request, None, "test", None).await
        else {
            panic!("expected guarded response")
        };
        assert_eq!(response.error_code(), Some(error_code::CONSENT_REQUIRED));
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn successful_mutation_replays_and_payload_mismatch_conflicts() {
        let root = tempfile::tempdir().expect("tempdir");
        let runtime = Arc::new(
            DesktopRuntime::new(root.path().join("kukuri.db"))
                .await
                .expect("runtime"),
        );
        let host = ClientHost::from_runtime(root.path().to_path_buf(), runtime)
            .await
            .expect("host");
        let counter = Arc::new(AtomicUsize::new(0));
        let dispatcher = write_dispatcher(counter.clone());
        let mut first = request("fixture.write");
        first.idempotency_key = Some(Uuid::now_v7().to_string());
        first.payload = json!({"value": 1});
        let DispatchReply::Unary(response, _) = dispatcher
            .dispatch(first.clone(), None, "test", Some(&host))
            .await
        else {
            panic!("expected first response")
        };
        assert!(response.ok);
        let DispatchReply::Unary(replay, _) = dispatcher
            .dispatch(first.clone(), None, "test", Some(&host))
            .await
        else {
            panic!("expected replay")
        };
        assert!(replay.ok);
        assert_eq!(replay.data, response.data);
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        first.payload = json!({"value": 2});
        let DispatchReply::Unary(conflict, _) =
            dispatcher.dispatch(first, None, "test", Some(&host)).await
        else {
            panic!("expected conflict")
        };
        assert_eq!(
            conflict.error_code(),
            Some(error_code::IDEMPOTENCY_CONFLICT)
        );
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        host.shutdown().await;
    }

    #[tokio::test]
    async fn secret_is_kept_out_of_the_json_response() {
        let metadata = CommandMetadata {
            name: "fixture.secret".to_string(),
            effect: CommandEffect::Read,
            secret_input: true,
            secret_output: true,
            idempotency_required: false,
            streaming: false,
            guards: Vec::new(),
            input_schema: json!({"type": "object", "additionalProperties": false}),
            output_schema: json!({"type": "object"}),
        };
        let dispatcher = Dispatcher::new(
            CommandRegistry::new(vec![CommandRegistration::new(
                metadata,
                Arc::new(SecretEchoHandler),
            )])
            .expect("registry"),
        );
        let sentinel = b"secret-sentinel";
        let mut request = request("fixture.secret");
        request.secret_bytes = Some(sentinel.len() as u64);
        request.accepts_secret_output = true;
        let DispatchReply::Unary(response, secret) = dispatcher
            .dispatch(
                request,
                Some(SecretInput::new(sentinel.to_vec())),
                "test",
                None,
            )
            .await
        else {
            panic!("expected secret response")
        };
        let encoded = serde_json::to_string(&response).expect("response json");
        assert!(!encoded.contains("secret-sentinel"));
        assert_eq!(secret.expect("secret frame").expose(), sentinel);
    }

    #[tokio::test]
    async fn preflight_rejects_guarded_secret_before_a_secret_frame_is_needed() {
        let metadata = CommandMetadata {
            name: "fixture.guarded-secret".to_string(),
            effect: CommandEffect::Read,
            secret_input: true,
            secret_output: false,
            idempotency_required: false,
            streaming: false,
            guards: vec![GuardRequirement::HostReady],
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
        };
        let dispatcher = Dispatcher::new(
            CommandRegistry::new(vec![CommandRegistration::new(
                metadata,
                Arc::new(SecretEchoHandler),
            )])
            .expect("registry"),
        );
        let mut request = request("fixture.guarded-secret");
        request.secret_bytes = Some(64);
        let error = dispatcher
            .preflight(&request, "test", None)
            .await
            .expect_err("guard blocks before secret read");
        assert_eq!(error.code, error_code::CONSENT_REQUIRED);
    }

    #[tokio::test]
    async fn domain_guard_has_an_injectable_success_path() {
        let metadata = CommandMetadata {
            name: "fixture.domain-read".to_string(),
            effect: CommandEffect::Read,
            secret_input: false,
            secret_output: false,
            idempotency_required: false,
            streaming: false,
            guards: vec![GuardRequirement::DomainAuthorization],
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
        };
        let dispatcher = Dispatcher::new(
            CommandRegistry::new(vec![CommandRegistration::new(
                metadata,
                Arc::new(CountingWriteHandler(Arc::new(AtomicUsize::new(0)))),
            )])
            .expect("registry"),
        )
        .with_guard_evaluator(Arc::new(AllowAllGuards));
        let DispatchReply::Unary(response, _) = dispatcher
            .dispatch(request("fixture.domain-read"), None, "test", None)
            .await
        else {
            panic!("expected unary response")
        };
        assert!(response.ok);
    }

    #[tokio::test]
    async fn secret_is_redacted_from_handler_errors_and_details() {
        let metadata = CommandMetadata {
            name: "fixture.secret-error".to_string(),
            effect: CommandEffect::Read,
            secret_input: true,
            secret_output: false,
            idempotency_required: false,
            streaming: false,
            guards: Vec::new(),
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
        };
        let dispatcher = Dispatcher::new(
            CommandRegistry::new(vec![CommandRegistration::new(
                metadata,
                Arc::new(SecretErrorHandler),
            )])
            .expect("registry"),
        );
        let sentinel = b"secret-error-sentinel";
        let mut request = request("fixture.secret-error");
        request.secret_bytes = Some(sentinel.len() as u64);
        let DispatchReply::Unary(response, _) = dispatcher
            .dispatch(
                request,
                Some(SecretInput::new(sentinel.to_vec())),
                "test",
                None,
            )
            .await
        else {
            panic!("expected error response")
        };
        let encoded = serde_json::to_string(&response).expect("response JSON");
        assert!(!encoded.contains("secret-error-sentinel"));
        assert!(encoded.contains("secret-bearing command failed"));
    }

    #[tokio::test]
    async fn handler_cannot_return_an_unregistered_secret_output() {
        let metadata = CommandMetadata {
            name: "fixture.secret-leak".to_string(),
            effect: CommandEffect::Read,
            secret_input: true,
            secret_output: false,
            idempotency_required: false,
            streaming: false,
            guards: Vec::new(),
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
        };
        let dispatcher = Dispatcher::new(
            CommandRegistry::new(vec![CommandRegistration::new(
                metadata,
                Arc::new(SecretEchoHandler),
            )])
            .expect("registry"),
        );
        let sentinel = b"unregistered-secret";
        let mut request = request("fixture.secret-leak");
        request.secret_bytes = Some(sentinel.len() as u64);
        let DispatchReply::Unary(response, secret) = dispatcher
            .dispatch(
                request,
                Some(SecretInput::new(sentinel.to_vec())),
                "test",
                None,
            )
            .await
        else {
            panic!("expected guarded response")
        };
        assert_eq!(response.error_code(), Some(error_code::INTERNAL_ERROR));
        assert!(secret.is_none());
        assert!(
            !serde_json::to_string(&response)
                .expect("response JSON")
                .contains("unregistered-secret")
        );
    }

    #[test]
    fn nested_schema_constraints_are_enforced() {
        let schema = json!({
            "type": "object",
            "required": ["mode", "items"],
            "properties": {
                "mode": {"type": "string", "enum": ["safe"]},
                "items": {
                    "type": "array",
                    "minItems": 1,
                    "items": {
                        "type": "object",
                        "required": ["name"],
                        "properties": {"name": {"type": "string", "minLength": 1}},
                        "additionalProperties": false
                    }
                }
            },
            "additionalProperties": false
        });
        assert!(
            validate_payload(&schema, &json!({"mode": "safe", "items": [{"name": "a"}]})).is_ok()
        );
        assert!(validate_payload(&schema, &json!({"mode": "unsafe", "items": []})).is_err());
        assert!(
            validate_payload(&schema, &json!({"mode": "safe", "items": [{"name": ""}]})).is_err()
        );
    }

    #[tokio::test]
    async fn secret_output_cannot_also_appear_in_json() {
        let metadata = CommandMetadata {
            name: "fixture.secret-output-leak".to_string(),
            effect: CommandEffect::Read,
            secret_input: false,
            secret_output: true,
            idempotency_required: false,
            streaming: false,
            guards: Vec::new(),
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
        };
        let dispatcher = Dispatcher::new(
            CommandRegistry::new(vec![CommandRegistration::new(
                metadata,
                Arc::new(SecretOutputLeakHandler),
            )])
            .expect("registry"),
        );
        let mut request = request("fixture.secret-output-leak");
        request.accepts_secret_output = true;
        let DispatchReply::Unary(response, secret) =
            dispatcher.dispatch(request, None, "test", None).await
        else {
            panic!("expected error response")
        };
        assert_eq!(response.error_code(), Some(error_code::INTERNAL_ERROR));
        assert!(secret.is_none());
        assert!(
            !serde_json::to_string(&response)
                .expect("response JSON")
                .contains("secret-output-\\\"-\\\\-sentinel")
        );
    }

    #[tokio::test]
    async fn secret_output_only_errors_are_redacted() {
        let metadata = CommandMetadata {
            name: "fixture.secret-output-error".to_string(),
            effect: CommandEffect::Read,
            secret_input: false,
            secret_output: true,
            idempotency_required: false,
            streaming: false,
            guards: Vec::new(),
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
        };
        let dispatcher = Dispatcher::new(
            CommandRegistry::new(vec![CommandRegistration::new(
                metadata,
                Arc::new(SecretOutputErrorHandler),
            )])
            .expect("registry"),
        );
        let mut request = request("fixture.secret-output-error");
        request.accepts_secret_output = true;
        let DispatchReply::Unary(response, _) =
            dispatcher.dispatch(request, None, "test", None).await
        else {
            panic!("expected error response")
        };
        let encoded = serde_json::to_string(&response).expect("response JSON");
        assert!(!encoded.contains("secret-output-error-sentinel"));
        assert!(encoded.contains("secret-bearing command failed"));
    }

    #[tokio::test]
    async fn secret_bearing_guard_errors_are_redacted() {
        let metadata = CommandMetadata {
            name: "fixture.secret-guard".to_string(),
            effect: CommandEffect::Read,
            secret_input: true,
            secret_output: false,
            idempotency_required: false,
            streaming: false,
            guards: vec![GuardRequirement::Credential],
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
        };
        let dispatcher = Dispatcher::new(
            CommandRegistry::new(vec![CommandRegistration::new(
                metadata,
                Arc::new(SecretEchoHandler),
            )])
            .expect("registry"),
        )
        .with_guard_evaluator(Arc::new(LeakingGuard));
        let mut request = request("fixture.secret-guard");
        request.secret_bytes = Some(1);
        let error = dispatcher
            .preflight(&request, "test", None)
            .await
            .expect_err("guard rejection");
        assert_eq!(error.code, error_code::AUTHORIZATION_FAILED);
        assert_eq!(error.message, "secret-bearing command failed");
        assert!(error.details.is_none());
    }
}
