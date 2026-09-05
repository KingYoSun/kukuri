use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use kukuri_cli::{
    dispatcher::{DispatchReply, Dispatcher},
    protocol::{
        CommandEffect, CommandMetadata, PROTOCOL_VERSION, ProtocolError, RequestEnvelope,
        SecretInput,
    },
    registry::{
        CommandHandler, CommandOutput, CommandRegistration, CommandRegistry, HandlerContext,
    },
};
use serde_json::{Value, json};

struct CountMutation(Arc<AtomicUsize>);

#[async_trait]
impl CommandHandler for CountMutation {
    async fn execute(
        &self,
        _: HandlerContext<'_>,
        _: Value,
        _: Option<&SecretInput>,
    ) -> Result<CommandOutput, ProtocolError> {
        let count = self.0.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(CommandOutput::Unary(json!({"count": count})))
    }
}

#[tokio::test]
async fn explicit_inputs_execute_once_each_without_an_operation_key_or_host() {
    let count = Arc::new(AtomicUsize::new(0));
    let dispatcher = Dispatcher::new(CommandRegistry::new(vec![CommandRegistration::new(
        CommandMetadata {
            name: "fixture.write".into(),
            effect: CommandEffect::Write,
            secret_input: false,
            secret_output: false,
            streaming: false,
            guards: vec![],
            input_schema: json!({"type": "object", "additionalProperties": false}),
            output_schema: json!({"type": "object", "required": ["count"], "properties": {"count": {"type": "integer"}}, "additionalProperties": false}),
        },
        Arc::new(CountMutation(count.clone())),
    )]).expect("registry"));
    let request = RequestEnvelope {
        protocol_version: PROTOCOL_VERSION,
        request_id: "same-correlation-id".into(),
        command: "fixture.write".into(),
        profile: "test".into(),
        payload: json!({}),
        timeout_ms: None,
        secret_bytes: None,
        accepts_secret_output: false,
    };
    for expected in 1..=2 {
        let DispatchReply::Unary(response, _) = dispatcher
            .dispatch(request.clone(), None, "test", None)
            .await
        else {
            panic!("単一応答");
        };
        assert!(
            response.ok,
            "操作キーなしで実行できること: {:?}",
            response.error
        );
        assert_eq!(response.data, Some(json!({"count": expected})));
        assert_eq!(count.load(Ordering::SeqCst), expected);
    }
    dispatcher.finish_operations().await;
    assert_eq!(count.load(Ordering::SeqCst), 2);
}
