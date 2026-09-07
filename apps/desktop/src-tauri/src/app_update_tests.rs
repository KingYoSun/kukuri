use super::*;
use std::io::{Read, Write};

fn app() -> tauri::App<tauri::test::MockRuntime> {
    let mut context = tauri::test::mock_context(tauri::test::noop_assets());
    context.config_mut().plugins.0.insert(
        "updater".into(),
        serde_json::json!({
            "pubkey": "test-only", "dangerousInsecureTransportProtocol": true
        }),
    );
    tauri::test::mock_builder()
        .manage(AppUpdateState::default())
        .manage(crate::restore_lifecycle::DesktopOperationState::default())
        .manage(crate::desktop_lifecycle::DesktopLifecycle::default())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
        .unwrap()
}

fn pending(app: &tauri::App<tauri::test::MockRuntime>) -> Pending {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/manifest", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut byte = [0];
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }
        let body = serde_json::json!({"version":"99.0.0", "url":"https://example.invalid/bundle", "signature":"test-only"}).to_string();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    });
    let updater = app
        .updater_builder()
        .endpoints(vec![endpoint.parse().unwrap()])
        .unwrap()
        .no_proxy()
        .build()
        .unwrap();
    let update = tauri::async_runtime::block_on(updater.check())
        .unwrap()
        .unwrap();
    server.join().unwrap();
    Pending {
        id: 1,
        update,
        verified: None,
    }
}

#[test]
fn direct_commands_reject_stale_unverified_and_busy_sessions_without_restart() {
    let app = app();
    let state = app.state::<AppUpdateState>();
    assert_eq!(
        require_installed(app.handle()).unwrap_err(),
        "update_not_installed"
    );
    assert_eq!(
        tauri::async_runtime::block_on(install_app_update(app.handle().clone(), 1)).unwrap_err(),
        "update_stale_session"
    );
    state.0.blocking_lock().pending = Some(pending(&app));
    assert_eq!(
        tauri::async_runtime::block_on(install_app_update(app.handle().clone(), 1)).unwrap_err(),
        "update_not_verified"
    );
    assert_eq!(
        tauri::async_runtime::block_on(download_app_update(
            app.handle().clone(),
            2,
            Channel::new(|_| Ok(()))
        ))
        .unwrap_err(),
        "update_stale_session"
    );
    let guard = state.0.blocking_lock();
    assert_eq!(
        tauri::async_runtime::block_on(install_app_update(app.handle().clone(), 1)).unwrap_err(),
        "update_busy"
    );
    drop(guard);
    assert_eq!(
        require_installed(app.handle()).unwrap_err(),
        "update_not_installed"
    );
}

#[test]
fn completed_install_cannot_be_overwritten_by_a_new_check() {
    let app = app();
    app.state::<AppUpdateState>().0.blocking_lock().installed = true;
    assert_eq!(
        tauri::async_runtime::block_on(check_app_update(app.handle().clone()))
            .err()
            .unwrap(),
        "update_already_downloaded"
    );
    assert!(require_installed(app.handle()).is_ok());
}

#[test]
fn webview_cannot_reach_upstream_install_or_download_commands() {
    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(tauri::generate_context!())
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    for command in ["check", "download", "install", "download_and_install"] {
        let result = tauri::test::get_ipc_response(
            &webview,
            tauri::webview::InvokeRequest {
                cmd: format!("plugin:updater|{command}"),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: if cfg!(windows) {
                    "http://tauri.localhost"
                } else {
                    "tauri://localhost"
                }
                .parse()
                .unwrap(),
                body: tauri::ipc::InvokeBody::Json(serde_json::json!({"rid": 1, "bytesRid": 2})),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.into(),
            },
        );
        let error = result
            .err()
            .expect("upstream command must be denied")
            .to_string();
        assert!(
            error.contains("not allowed") || error.contains("not permitted"),
            "{command}: {error}"
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn failed_deb_attempt_consumes_payload_and_keeps_restart_forbidden() {
    let app = app();
    let mut candidate = pending(&app);
    // Private test state only: exercise the real format refusal before OS authentication.
    candidate.verified = Some(b"\x7fELFwrong-format".to_vec());
    app.state::<AppUpdateState>().0.blocking_lock().pending = Some(candidate);
    assert_eq!(
        tauri::async_runtime::block_on(install_checked(app.handle().clone(), 1, true)).unwrap_err(),
        "deb_update_format_mismatch"
    );
    assert_eq!(
        tauri::async_runtime::block_on(install_checked(app.handle().clone(), 1, true)).unwrap_err(),
        "update_not_verified"
    );
    assert_eq!(
        require_installed(app.handle()).unwrap_err(),
        "update_not_installed"
    );
}
