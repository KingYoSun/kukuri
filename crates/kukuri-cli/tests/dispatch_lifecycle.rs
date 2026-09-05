use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use kukuri_cli::{
    dispatcher::Dispatcher,
    protocol::{
        CommandEffect, CommandMetadata, GuardRequirement, PROTOCOL_VERSION, ProtocolError,
        RequestEnvelope, SecretInput, SecretOutput,
    },
    registry::{
        CommandHandler, CommandOutput, CommandRegistration, CommandRegistry, HandlerContext,
    },
};
use kukuri_desktop_runtime::{ClientHost, DesktopRuntime};
use serde_json::{Value, json};
use tokio::sync::Notify;

struct GatedMutation {
    secret_output: bool,
    started: Arc<AtomicUsize>,
    entered: Arc<Notify>,
    release: Arc<Notify>,
    finalized: Arc<Notify>,
}

#[tokio::test]
async fn unconfigured_domain_guard_rejects_even_when_host_is_ready() {
    let root = tempfile::tempdir().expect("tempdir");
    let runtime = Arc::new(
        DesktopRuntime::new(root.path().join("kukuri.db"))
            .await
            .expect("runtime"),
    );
    let host = ClientHost::from_runtime(root.path().to_path_buf(), runtime)
        .await
        .expect("host");
    for guard in [
        GuardRequirement::Account,
        GuardRequirement::Consent,
        GuardRequirement::Audience,
        GuardRequirement::Credential,
        GuardRequirement::DomainAuthorization,
    ] {
        let dispatcher = Dispatcher::new(
            CommandRegistry::new(vec![CommandRegistration::new(
                CommandMetadata {
                    name: "fixture.guard".into(),
                    effect: CommandEffect::Read,
                    secret_input: false,
                    secret_output: false,
                    streaming: false,
                    guards: vec![guard],
                    input_schema: json!({"type": "object"}),
                    output_schema: json!({}),
                },
                Arc::new(GatedMutation {
                    secret_output: false,
                    started: Arc::new(AtomicUsize::new(0)),
                    entered: Arc::new(Notify::new()),
                    release: Arc::new(Notify::new()),
                    finalized: Arc::new(Notify::new()),
                }),
            )])
            .expect("registry"),
        );
        let request = RequestEnvelope {
            protocol_version: PROTOCOL_VERSION,
            request_id: "guard".into(),
            command: "fixture.guard".into(),
            profile: "test".into(),
            payload: json!({}),
            timeout_ms: None,
            secret_bytes: None,
            accepts_secret_output: false,
        };
        let error = dispatcher
            .preflight(&request, "test", Some(&host))
            .await
            .expect_err("未設定guardは拒否");
        assert_eq!(
            error.code,
            kukuri_cli::protocol::error_code::ACTION_REQUIRED
        );
    }
    host.shutdown().await;
}

#[async_trait]
impl CommandHandler for GatedMutation {
    async fn execute(
        &self,
        _context: HandlerContext<'_>,
        _payload: Value,
        _secret: Option<&SecretInput>,
    ) -> Result<CommandOutput, ProtocolError> {
        self.started.fetch_add(1, Ordering::SeqCst);
        self.entered.notify_one();
        self.release.notified().await;
        self.finalized.notify_one();
        let data = json!({"object_id": "one-object"});
        if self.secret_output {
            Ok(CommandOutput::Secret {
                data,
                secret: SecretOutput::new(b"generated-secret-sentinel".to_vec()),
            })
        } else {
            Ok(CommandOutput::Unary(data))
        }
    }
}

#[tokio::test]
async fn disconnect_finishes_the_single_mutation_without_reexecution() {
    check_disconnected_mutation(false).await;
}

#[tokio::test]
async fn disconnected_secret_output_mutation_finishes_without_reexecution() {
    check_disconnected_mutation(true).await;
}

async fn check_disconnected_mutation(secret_output: bool) {
    let root = tempfile::tempdir().expect("tempdir");
    let runtime = Arc::new(
        DesktopRuntime::new(root.path().join("kukuri.db"))
            .await
            .expect("runtime"),
    );
    let host = ClientHost::from_runtime(root.path().to_path_buf(), runtime)
        .await
        .expect("host");
    let started = Arc::new(AtomicUsize::new(0));
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let finalized = Arc::new(Notify::new());
    let registry = CommandRegistry::new(vec![CommandRegistration::new(
        CommandMetadata {
            name: "fixture.mutation".into(),
            effect: CommandEffect::Write,
            secret_input: false,
            secret_output,
            streaming: false,
            guards: vec![GuardRequirement::HostReady],
            input_schema: json!({"type": "object", "additionalProperties": false}),
            output_schema: json!({"type": "object", "required": ["object_id"],
            "properties": {"object_id": {"type": "string"}}, "additionalProperties": false}),
        },
        Arc::new(GatedMutation {
            secret_output,
            started: started.clone(),
            entered: entered.clone(),
            release: release.clone(),
            finalized: finalized.clone(),
        }),
    )])
    .expect("registry");
    let dispatcher = Arc::new(Dispatcher::new(registry));
    let request = RequestEnvelope {
        protocol_version: PROTOCOL_VERSION,
        request_id: "disconnect-fixture".into(),
        command: "fixture.mutation".into(),
        profile: "test".into(),
        payload: json!({}),
        timeout_ms: None,
        secret_bytes: None,
        accepts_secret_output: secret_output,
    };
    let client = tokio::spawn({
        let dispatcher = dispatcher.clone();
        let host = host.clone();
        let request = request.clone();
        async move {
            dispatcher
                .dispatch(request, None, "test", Some(&host))
                .await
        }
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), entered.notified())
        .await
        .expect("mutation開始");
    client.abort();
    assert!(client.await.err().expect("client終了").is_cancelled());
    release.notify_one();
    let finalized =
        tokio::time::timeout(std::time::Duration::from_secs(1), finalized.notified()).await;
    assert!(finalized.is_ok(), "切断後も変更操作の後処理を完了すること");
    dispatcher.finish_operations().await;
    assert_eq!(started.load(Ordering::SeqCst), 1);
    assert!(!root.path().join("kukuri.idempotency.sqlite3").exists());
    host.shutdown().await;
}
