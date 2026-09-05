use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use kukuri_cli::{
    client::daemon_socket_path,
    dispatcher::{DispatchReply, Dispatcher, decode_request},
    framing::{read_json_frame, read_secret_frame, write_json_frame, write_secret_frame},
    protocol::{DEFAULT_TIMEOUT_MS, ProtocolError, ResponseEnvelope, SecretInput, error_code},
    session::ClientSession,
};
use kukuri_desktop_runtime::{ClientHost, ClientProfile, ProfileLease};
use tokio::{
    io::BufReader,
    net::{UnixListener, UnixStream},
    process::Command,
    sync::Semaphore,
    task::JoinSet,
    time::Instant,
};

use crate::{CliError, DaemonCommand};

const MAX_CONNECTIONS: usize = 64;

pub(crate) async fn run(profile: ClientProfile, command: DaemonCommand) -> Result<(), CliError> {
    match command {
        DaemonCommand::Run => run_foreground(profile).await,
        DaemonCommand::Start => systemctl(&profile.name, "start").await,
        DaemonCommand::Stop => systemctl(&profile.name, "stop").await,
        DaemonCommand::Status => systemctl(&profile.name, "status").await,
    }
}

async fn run_foreground(profile: ClientProfile) -> Result<(), CliError> {
    std::panic::set_hook(Box::new(|_| eprintln!("daemon task panicked")));
    let lease = ProfileLease::acquire(profile).map_err(CliError::from_profile)?;
    let socket_path = daemon_socket_path(lease.profile()).map_err(CliError::from_protocol)?;
    let runtime_directory = socket_path.parent().ok_or_else(|| {
        CliError::new(
            "runtime_dir_unavailable",
            "daemon socket has no runtime directory",
            1,
        )
    })?;
    ensure_private_runtime_directory(runtime_directory)?;
    let listener = bind_listener(&socket_path)?;
    let _socket_cleanup = SocketCleanup(socket_path);

    let session = ClientSession::start(lease.profile().app_data_dir.clone())
        .await
        .map_err(CliError::from_protocol)?;
    // readinessを通知する前にsignal handlerを登録し、直後の停止要求を取りこぼさない。
    let interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
        .map_err(|error| CliError::new("signal_setup_failed", error.to_string(), 1))?;
    let terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(|error| CliError::new("signal_setup_failed", error.to_string(), 1))?;
    eprintln!(
        "{}",
        if session.host().is_some() {
            "ready"
        } else {
            "consent_required"
        }
    );

    let result = wait_for_shutdown(
        listener,
        lease.profile().name.clone(),
        None,
        Arc::new(Dispatcher::for_session(session.clone())),
        interrupt,
        terminate,
    )
    .await;
    session.shutdown().await;
    drop(lease);
    result
}

async fn wait_for_shutdown(
    listener: UnixListener,
    profile: String,
    host: Option<Arc<ClientHost>>,
    dispatcher: Arc<Dispatcher>,
    mut interrupt: tokio::signal::unix::Signal,
    mut terminate: tokio::signal::unix::Signal,
) -> Result<(), CliError> {
    let mut connections = JoinSet::new();
    let connection_permits = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    let result = loop {
        tokio::select! {
            _ = interrupt.recv() => {
                break Ok(());
            }
            _ = terminate.recv() => {
                break Ok(());
            },
            result = listener.accept() => {
                let (stream, _) = match result {
                    Ok(connection) => connection,
                    Err(error) => break Err(CliError::new(
                        "socket_accept_failed", error.to_string(), 1,
                    )),
                };
                match authorize_peer(&stream) {
                    Ok(()) => {
                        let Ok(permit) = connection_permits.clone().try_acquire_owned() else {
                            reject_over_capacity(stream, &profile).await;
                            continue;
                        };
                        let dispatcher = dispatcher.clone();
                        let host = host.clone();
                        let profile = profile.clone();
                        connections.spawn(async move {
                            let _permit = permit;
                            handle_connection(stream, dispatcher, profile, host).await
                        });
                    }
                    Err(error) => eprintln!("{}: {}", error.code, error.message),
                }
            }
            Some(result) = connections.join_next(), if !connections.is_empty() => {
                if let Ok(Err(error)) = result {
                    eprintln!("{}: {}", error.code, error.message);
                }
            }
        }
    };
    drop(listener);
    let drain = async { while connections.join_next().await.is_some() {} };
    if tokio::time::timeout(Duration::from_secs(5), drain)
        .await
        .is_err()
    {
        connections.abort_all();
        while connections.join_next().await.is_some() {}
    }
    dispatcher.finish_operations().await;
    result
}

async fn reject_over_capacity(stream: UnixStream, profile: &str) {
    let (read, mut write) = tokio::io::split(stream);
    let mut reader = BufReader::new(read);
    let request = tokio::time::timeout(Duration::from_millis(100), read_json_frame(&mut reader))
        .await
        .ok()
        .and_then(Result::ok)
        .flatten()
        .and_then(|bytes| decode_request(&bytes).ok());
    let error = ProtocolError::new(error_code::BACKPRESSURE, "daemon connection limit reached");
    let response = match request {
        Some(request) => ResponseEnvelope::failure(&request, error),
        None => ResponseEnvelope::failure_parts("", "", profile, error),
    };
    let _ = tokio::time::timeout(
        Duration::from_millis(100),
        write_json_frame(&mut write, &response),
    )
    .await;
}

async fn handle_connection(
    stream: UnixStream,
    dispatcher: Arc<Dispatcher>,
    profile: String,
    host: Option<Arc<ClientHost>>,
) -> Result<(), ProtocolError> {
    let (read, mut write) = tokio::io::split(stream);
    let mut reader = BufReader::new(read);
    let bytes = match tokio::time::timeout(
        Duration::from_millis(DEFAULT_TIMEOUT_MS),
        read_json_frame(&mut reader),
    )
    .await
    {
        Err(_) => {
            let response = ResponseEnvelope::failure_parts(
                "",
                "",
                &profile,
                ProtocolError::new(error_code::TIMEOUT, "request frame timed out"),
            );
            let _ = tokio::time::timeout(
                Duration::from_millis(100),
                write_json_frame(&mut write, &response),
            )
            .await;
            return Ok(());
        }
        Ok(result) => match result {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Ok(()),
            Err(error) => {
                let response = ResponseEnvelope::failure_parts("", "", &profile, error);
                write_json_frame(&mut write, &response).await?;
                return Ok(());
            }
        },
    };
    let request = match decode_request(&bytes) {
        Ok(request) => request,
        Err(error) => {
            let response = ResponseEnvelope::failure_parts("", "", &profile, error);
            write_json_frame(&mut write, &response).await?;
            return Ok(());
        }
    };
    let timeout_ms = match request.timeout_ms() {
        Ok(timeout) => timeout,
        Err(error) => {
            let response = ResponseEnvelope::failure(&request, error);
            write_json_frame(&mut write, &response).await?;
            return Ok(());
        }
    };
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    if let Err(error) = dispatcher
        .preflight(&request, &profile, host.as_ref())
        .await
    {
        let response = ResponseEnvelope::failure(&request, error);
        write_json_before(deadline, &mut write, &response).await?;
        return Ok(());
    }
    let secret = match request.secret_bytes {
        Some(length) => {
            let mut ready =
                ResponseEnvelope::success(&request, serde_json::json!({"ready_for_secret": true}));
            ready.more = true;
            write_json_before(deadline, &mut write, &ready).await?;
            let bytes =
                match tokio::time::timeout_at(deadline, read_secret_frame(&mut reader, length))
                    .await
                {
                    Ok(Ok(bytes)) => bytes,
                    Ok(Err(error)) => {
                        let response = ResponseEnvelope::failure(&request, error);
                        let _ = tokio::time::timeout(
                            Duration::from_millis(100),
                            write_json_frame(&mut write, &response),
                        )
                        .await;
                        return Ok(());
                    }
                    Err(_) => {
                        let response = ResponseEnvelope::failure(
                            &request,
                            ProtocolError::new(error_code::TIMEOUT, "secret input timed out"),
                        );
                        let _ = tokio::time::timeout(
                            Duration::from_millis(100),
                            write_json_frame(&mut write, &response),
                        )
                        .await;
                        return Ok(());
                    }
                };
            Some(SecretInput::new(bytes))
        }
        None => None,
    };
    let reply = match tokio::time::timeout_at(
        deadline,
        dispatcher.dispatch(request.clone(), secret, &profile, host.as_ref()),
    )
    .await
    {
        Ok(reply) => reply,
        Err(_) => DispatchReply::Unary(
            ResponseEnvelope::failure(
                &request,
                ProtocolError::new(error_code::TIMEOUT, "command timed out"),
            ),
            None,
        ),
    };
    match reply {
        DispatchReply::Unary(response, secret) => {
            write_json_before(deadline, &mut write, &response).await?;
            if let Some(secret) = secret {
                tokio::time::timeout_at(deadline, write_secret_frame(&mut write, secret.expose()))
                    .await
                    .map_err(|_| {
                        ProtocolError::new(error_code::TIMEOUT, "secret output timed out")
                    })??;
            }
        }
        DispatchReply::Events {
            request,
            mut receiver,
        } => {
            let mut started =
                ResponseEnvelope::success(&request, serde_json::json!({"subscribed": true}));
            started.more = true;
            write_json_before(deadline, &mut write, &started).await?;
            loop {
                let received = tokio::time::timeout_at(deadline, receiver.recv()).await;
                match received {
                    Err(_) => {
                        let response = ResponseEnvelope::failure(
                            &request,
                            ProtocolError::new(error_code::TIMEOUT, "command timed out"),
                        );
                        let _ = tokio::time::timeout(
                            Duration::from_millis(100),
                            write_json_frame(&mut write, &response),
                        )
                        .await;
                        break;
                    }
                    Ok(Ok(event)) => {
                        let data = serde_json::to_value(event).map_err(|_| {
                            ProtocolError::new(
                                error_code::INTERNAL_ERROR,
                                "failed to encode runtime event",
                            )
                        })?;
                        let mut response = ResponseEnvelope::success(&request, data);
                        response.more = true;
                        write_json_before(deadline, &mut write, &response).await?;
                    }
                    Ok(Err(error)) => {
                        if let Some(response) = stream_receive_failure(&request, error) {
                            write_json_before(deadline, &mut write, &response).await?;
                        }
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

fn stream_receive_failure(
    request: &kukuri_cli::protocol::RequestEnvelope,
    error: tokio::sync::broadcast::error::RecvError,
) -> Option<ResponseEnvelope> {
    match error {
        tokio::sync::broadcast::error::RecvError::Lagged(_) => Some(ResponseEnvelope::failure(
            request,
            ProtocolError::new(error_code::BACKPRESSURE, "event consumer fell behind"),
        )),
        tokio::sync::broadcast::error::RecvError::Closed => None,
    }
}

async fn write_json_before<W: tokio::io::AsyncWrite + Unpin, T: serde::Serialize>(
    deadline: Instant,
    writer: &mut W,
    value: &T,
) -> Result<(), ProtocolError> {
    tokio::time::timeout_at(deadline, write_json_frame(writer, value))
        .await
        .map_err(|_| ProtocolError::new(error_code::TIMEOUT, "response output timed out"))?
}

fn ensure_private_runtime_directory(path: &Path) -> Result<(), CliError> {
    use std::os::unix::fs::PermissionsExt;

    if let Ok(metadata) = std::fs::symlink_metadata(path)
        && metadata.file_type().is_symlink()
    {
        return Err(CliError::new(
            "runtime_dir_invalid",
            format!(
                "runtime directory `{}` must not be a symlink",
                path.display()
            ),
            1,
        ));
    }
    std::fs::create_dir_all(path).map_err(|error| {
        CliError::new(
            "runtime_dir_unavailable",
            format!("failed to create `{}`: {error}", path.display()),
            1,
        )
    })?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(|error| {
        CliError::new(
            "runtime_dir_permission_failed",
            format!("failed to protect `{}`: {error}", path.display()),
            1,
        )
    })
}

fn bind_listener(path: &Path) -> Result<UnixListener, CliError> {
    use std::os::unix::fs::{FileTypeExt, PermissionsExt};

    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        if !metadata.file_type().is_socket() {
            return Err(CliError::new(
                "socket_path_invalid",
                format!("socket path `{}` is not a socket", path.display()),
                1,
            ));
        }
        std::fs::remove_file(path).map_err(|error| {
            CliError::new(
                "socket_cleanup_failed",
                format!(
                    "failed to remove stale socket `{}`: {error}",
                    path.display()
                ),
                1,
            )
        })?;
    }
    let listener = UnixListener::bind(path).map_err(|error| {
        CliError::new(
            "socket_bind_failed",
            format!("failed to bind `{}`: {error}", path.display()),
            1,
        )
    })?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|error| {
        CliError::new(
            "socket_permission_failed",
            format!("failed to protect `{}`: {error}", path.display()),
            1,
        )
    })?;
    Ok(listener)
}

fn authorize_peer(stream: &UnixStream) -> Result<(), CliError> {
    let credential = stream
        .peer_cred()
        .map_err(|error| CliError::new("peer_credential_failed", error.to_string(), 1))?;
    authorize_peer_uid(unsafe { libc::geteuid() }, credential.uid())
}

fn authorize_peer_uid(expected_uid: u32, actual_uid: u32) -> Result<(), CliError> {
    if expected_uid == actual_uid {
        Ok(())
    } else {
        Err(CliError::new(
            "peer_uid_mismatch",
            format!("peer uid {actual_uid} does not match daemon uid {expected_uid}"),
            1,
        ))
    }
}

async fn systemctl(profile: &str, verb: &str) -> Result<(), CliError> {
    let unit = format!("kukuri@{profile}.service");
    let output = Command::new("systemctl")
        .args(["--user", verb, unit.as_str()])
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|error| CliError::new("systemd_unavailable", error.to_string(), 1))?;
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if output.status.success() {
        Ok(())
    } else {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(CliError::new(
            "systemd_command_failed",
            if message.is_empty() {
                format!("systemctl --user {verb} {unit} failed")
            } else {
                message
            },
            output.status.code().unwrap_or(1),
        ))
    }
}

struct SocketCleanup(PathBuf);

impl Drop for SocketCleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use kukuri_cli::{
        protocol::{
            CommandEffect, CommandMetadata, PROTOCOL_VERSION, RequestEnvelope, SecretOutput,
        },
        registry::{
            CommandHandler, CommandOutput, CommandRegistration, CommandRegistry, HandlerContext,
        },
    };
    use kukuri_desktop_runtime::{ClientProfileKind, resolve_cli_profile};
    use serde_json::{Value, json};
    use tokio::io::{AsyncWriteExt, BufReader};

    use super::*;

    #[tokio::test]
    async fn runtime_directory_and_socket_are_private() {
        let root = tempfile::tempdir().expect("tempdir");
        let runtime_dir = root.path().join("runtime");
        ensure_private_runtime_directory(&runtime_dir).expect("runtime dir");
        let socket = runtime_dir.join("test.sock");
        let listener = bind_listener(&socket).expect("listener");
        assert_eq!(
            std::fs::metadata(&runtime_dir)
                .expect("dir metadata")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(&socket)
                .expect("socket metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        drop(listener);
    }

    #[test]
    fn peer_uid_mismatch_is_rejected() {
        let error = authorize_peer_uid(1000, 1001).expect_err("different uid");
        assert_eq!(error.code, "peer_uid_mismatch");
    }

    #[test]
    fn profile_kind_for_daemon_is_cli() {
        let profile =
            resolve_cli_profile(Some("alpha"), None, None, Some(Path::new("/data")), None)
                .expect("profile");
        assert_eq!(profile.kind, ClientProfileKind::Cli);
    }

    #[tokio::test]
    async fn lagged_event_queue_maps_to_typed_backpressure() {
        let (sender, mut receiver) = tokio::sync::broadcast::channel(1);
        sender.send(1).expect("first event");
        sender.send(2).expect("second event");
        let error = receiver.recv().await.expect_err("receiver lagged");
        let request = kukuri_cli::protocol::RequestEnvelope {
            protocol_version: kukuri_cli::protocol::PROTOCOL_VERSION,
            request_id: "req-1".to_string(),
            command: "events.watch".to_string(),
            profile: "test".to_string(),
            payload: serde_json::json!({}),
            timeout_ms: None,
            secret_bytes: None,
            accepts_secret_output: false,
        };
        let response = stream_receive_failure(&request, error).expect("terminal response");
        assert_eq!(response.error_code(), Some(error_code::BACKPRESSURE));
    }

    struct SecretRoundtripHandler;

    #[async_trait]
    impl CommandHandler for SecretRoundtripHandler {
        async fn execute(
            &self,
            _context: HandlerContext<'_>,
            _payload: Value,
            secret: Option<&SecretInput>,
        ) -> Result<CommandOutput, ProtocolError> {
            Ok(CommandOutput::Secret {
                data: json!({"accepted": true}),
                secret: SecretOutput::new(secret.expect("secret input").expose().to_vec()),
            })
        }
    }

    fn secret_dispatcher() -> Arc<Dispatcher> {
        let metadata = CommandMetadata {
            name: "fixture.secret-roundtrip".to_string(),
            effect: CommandEffect::Read,
            secret_input: true,
            secret_output: true,
            streaming: false,
            guards: Vec::new(),
            input_schema: json!({"type": "object", "additionalProperties": false}),
            output_schema: json!({"type": "object"}),
        };
        Arc::new(Dispatcher::new(
            CommandRegistry::new(vec![CommandRegistration::new(
                metadata,
                Arc::new(SecretRoundtripHandler),
            )])
            .expect("registry"),
        ))
    }

    fn secret_request(length: usize) -> RequestEnvelope {
        RequestEnvelope {
            protocol_version: PROTOCOL_VERSION,
            request_id: "secret-request".to_string(),
            command: "fixture.secret-roundtrip".to_string(),
            profile: "test".to_string(),
            payload: json!({}),
            timeout_ms: Some(2_000),
            secret_bytes: Some(length as u64),
            accepts_secret_output: true,
        }
    }

    struct CountingMutation {
        count: Arc<AtomicUsize>,
        gate: Option<Arc<tokio::sync::Notify>>,
    }

    #[async_trait]
    impl CommandHandler for CountingMutation {
        async fn execute(
            &self,
            _: HandlerContext<'_>,
            _: Value,
            _: Option<&SecretInput>,
        ) -> Result<CommandOutput, ProtocolError> {
            let count = self.count.fetch_add(1, Ordering::SeqCst) + 1;
            if let Some(gate) = &self.gate {
                gate.notified().await;
            }
            Ok(CommandOutput::Unary(json!({"count": count})))
        }
    }

    fn counting_dispatcher(
        count: Arc<AtomicUsize>,
        gate: Option<Arc<tokio::sync::Notify>>,
    ) -> Arc<Dispatcher> {
        Arc::new(Dispatcher::new(CommandRegistry::new(vec![CommandRegistration::new(
            CommandMetadata {
                name: "fixture.write".into(), effect: CommandEffect::Write,
                secret_input: false, secret_output: false, streaming: false, guards: vec![],
                input_schema: json!({"type": "object", "additionalProperties": false}),
                output_schema: json!({"type": "object", "required": ["count"], "properties": {"count": {"type": "integer"}}, "additionalProperties": false}),
            },
            Arc::new(CountingMutation { count, gate }),
        )]).expect("registry")))
    }

    #[tokio::test]
    async fn accept_failure_waits_for_the_owned_mutation_before_shutdown() {
        use std::os::fd::{FromRawFd, IntoRawFd};
        use tokio::signal::unix::{SignalKind, signal};

        // 非listen socketの所有権を移し、実accept syscallの失敗を発生させる。
        let (socket, sender) = std::os::unix::net::UnixDatagram::pair().expect("socket");
        sender.send(b"ready").expect("readable socket");
        socket.set_nonblocking(true).expect("nonblocking");
        let listener =
            unsafe { std::os::unix::net::UnixListener::from_raw_fd(socket.into_raw_fd()) };
        let listener = UnixListener::from_std(listener).expect("listener");
        let count = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new(tokio::sync::Notify::new());
        let dispatcher = counting_dispatcher(count.clone(), Some(gate.clone()));
        let request = RequestEnvelope {
            protocol_version: PROTOCOL_VERSION,
            request_id: "accept-failure".into(),
            command: "fixture.write".into(),
            profile: "test".into(),
            payload: json!({}),
            timeout_ms: None,
            secret_bytes: None,
            accepts_secret_output: false,
        };
        let pending = tokio::spawn({
            let dispatcher = dispatcher.clone();
            async move { dispatcher.dispatch(request, None, "test", None).await }
        });
        tokio::time::timeout(Duration::from_secs(2), async {
            while count.load(Ordering::SeqCst) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("handler開始");
        pending.abort();
        let _ = pending.await;
        let mut shutdown = tokio::spawn(wait_for_shutdown(
            listener,
            "test".into(),
            None,
            dispatcher.clone(),
            signal(SignalKind::interrupt()).expect("interrupt"),
            signal(SignalKind::terminate()).expect("terminate"),
        ));
        let exited_early = tokio::time::timeout(Duration::from_millis(100), &mut shutdown).await;
        gate.notify_one();
        // 失敗時もtest自身のtaskを残さない。
        dispatcher.finish_operations().await;
        assert!(
            exited_early.is_err(),
            "accept失敗でも実行中のmutationの完了を待つ"
        );
        let error = tokio::time::timeout(Duration::from_secs(2), shutdown)
            .await
            .expect("shutdown期限")
            .expect("shutdown task")
            .expect_err("accept失敗");
        assert_eq!(error.code, "socket_accept_failed");
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn socket_inputs_execute_once_each_and_timeout_never_reexecutes() {
        let count = Arc::new(AtomicUsize::new(0));
        let dispatcher = counting_dispatcher(count.clone(), None);
        let request = RequestEnvelope {
            protocol_version: PROTOCOL_VERSION,
            request_id: "same-request-id".into(),
            command: "fixture.write".into(),
            profile: "test".into(),
            payload: json!({}),
            timeout_ms: Some(2_000),
            secret_bytes: None,
            accepts_secret_output: false,
        };
        for expected in 1..=2 {
            let (client, server) = UnixStream::pair().expect("socket pair");
            let task = tokio::spawn(handle_connection(
                server,
                dispatcher.clone(),
                "test".into(),
                None,
            ));
            let (read, mut write) = tokio::io::split(client);
            let mut reader = BufReader::new(read);
            write_json_frame(&mut write, &request)
                .await
                .expect("request");
            write.shutdown().await.expect("finish request");
            let bytes = read_json_frame(&mut reader)
                .await
                .expect("frame")
                .expect("response");
            let response: ResponseEnvelope = serde_json::from_slice(&bytes).expect("JSON");
            assert!(response.ok, "{:?}", response.error);
            assert_eq!(response.data, Some(json!({"count": expected})));
            task.await.expect("server task").expect("server result");
            assert_eq!(count.load(Ordering::SeqCst), expected);
        }
        dispatcher.finish_operations().await;

        let gate = Arc::new(tokio::sync::Notify::new());
        let dispatcher = counting_dispatcher(count.clone(), Some(gate.clone()));
        let (client, server) = UnixStream::pair().expect("timeout socket");
        let task = tokio::spawn(handle_connection(
            server,
            dispatcher.clone(),
            "test".into(),
            None,
        ));
        let (read, mut write) = tokio::io::split(client);
        let mut reader = BufReader::new(read);
        let mut request = request;
        request.timeout_ms = Some(100);
        write_json_frame(&mut write, &request)
            .await
            .expect("timeout request");
        write.shutdown().await.expect("finish request");
        let bytes = read_json_frame(&mut reader)
            .await
            .expect("timeout frame")
            .expect("timeout response");
        let response: ResponseEnvelope = serde_json::from_slice(&bytes).expect("timeout JSON");
        assert_eq!(response.error_code(), Some(error_code::TIMEOUT));
        task.await
            .expect("timeout server task")
            .expect("timeout result");
        gate.notify_one();
        dispatcher.finish_operations().await;
        assert_eq!(
            count.load(Ordering::SeqCst),
            3,
            "timeoutを理由にhandlerを再起動しない"
        );
    }

    #[tokio::test]
    async fn secret_handshake_roundtrips_and_partial_body_returns_typed_error() {
        let sentinel = b"socket-secret-\"-\\-sentinel";
        let (client, server) = UnixStream::pair().expect("socket pair");
        let task = tokio::spawn(handle_connection(
            server,
            secret_dispatcher(),
            "test".to_string(),
            None,
        ));
        let (read, mut write) = tokio::io::split(client);
        let mut reader = BufReader::new(read);
        let request = secret_request(sentinel.len());
        write_json_frame(&mut write, &request)
            .await
            .expect("header");
        let ready = read_json_frame(&mut reader)
            .await
            .expect("ready frame")
            .expect("ready response");
        let ready: ResponseEnvelope = serde_json::from_slice(&ready).expect("ready JSON");
        assert!(ready.ok && ready.more);
        write_secret_frame(&mut write, sentinel)
            .await
            .expect("secret body");
        write.shutdown().await.expect("finish request");
        let response = read_json_frame(&mut reader)
            .await
            .expect("response frame")
            .expect("response");
        let response: ResponseEnvelope = serde_json::from_slice(&response).expect("response JSON");
        assert!(response.ok);
        let output = read_secret_frame(
            &mut reader,
            response.secret_bytes.expect("secret response length"),
        )
        .await
        .expect("secret response");
        assert_eq!(output, sentinel);
        task.await.expect("server task").expect("server result");

        let (client, server) = UnixStream::pair().expect("partial socket pair");
        let task = tokio::spawn(handle_connection(
            server,
            secret_dispatcher(),
            "test".to_string(),
            None,
        ));
        let (read, mut write) = tokio::io::split(client);
        let mut reader = BufReader::new(read);
        let request = secret_request(sentinel.len());
        write_json_frame(&mut write, &request)
            .await
            .expect("header");
        let _ready = read_json_frame(&mut reader)
            .await
            .expect("ready frame")
            .expect("ready response");
        write.write_all(&sentinel[..3]).await.expect("partial body");
        write.shutdown().await.expect("finish partial request");
        let response = read_json_frame(&mut reader)
            .await
            .expect("error frame")
            .expect("error response");
        let response: ResponseEnvelope = serde_json::from_slice(&response).expect("error JSON");
        assert_eq!(response.request_id, request.request_id);
        assert_eq!(response.error_code(), Some(error_code::INVALID_REQUEST));
        task.await.expect("server task").expect("server result");
    }
}
