//! #978: 開発者モード向けのアプリ内ログ閲覧。
//!
//! backend は開発者モードを永続化せず、frontend(localStorage)が mount 時と変更時に
//! `set_developer_mode_enabled` でミラーする。`read_desktop_logs` はミラーが false なら
//! buffer に触れず typed error で拒否する。どちらも invoke gate を通るため Ready 以外は
//! そこで拒否される(INVAR-3)。ログ本文は in-memory ring buffer(`crate::tracing`)からの
//! 読み取りだけで、file・network・DB への sink は持たない。

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::{
    state::CommandError,
    tracing::{DesktopLogBuffer, DesktopLogSnapshot},
};

pub(crate) const DEVELOPER_MODE_DISABLED_CODE: &str = "developer_mode_disabled";

pub(crate) struct DeveloperLogState {
    developer_mode: AtomicBool,
    buffer: Arc<DesktopLogBuffer>,
}

impl DeveloperLogState {
    pub(crate) fn new(buffer: Arc<DesktopLogBuffer>) -> Self {
        Self {
            developer_mode: AtomicBool::new(false),
            buffer,
        }
    }

    pub(crate) fn developer_mode_enabled(&self) -> bool {
        self.developer_mode.load(Ordering::SeqCst)
    }

    fn set_developer_mode(&self, enabled: bool) {
        self.developer_mode.store(enabled, Ordering::SeqCst);
    }

    fn read(
        &self,
        after_seq: Option<u64>,
        limit: Option<u32>,
    ) -> Result<DesktopLogSnapshot, CommandError> {
        if !self.developer_mode_enabled() {
            return Err(CommandError {
                code: DEVELOPER_MODE_DISABLED_CODE.to_string(),
                message: "desktop logs are only readable while developer mode is enabled"
                    .to_string(),
                status: None,
                retry_after_seconds: None,
            });
        }
        let limit = limit
            .map(|value| value as usize)
            .filter(|value| *value > 0)
            .unwrap_or_else(|| self.buffer.max_entries());
        Ok(self.buffer.snapshot(after_seq, limit))
    }
}

/// frontend の開発者モード(localStorage)を backend へ写す。永続化しない。
#[tauri::command]
pub fn set_developer_mode_enabled(state: tauri::State<'_, DeveloperLogState>, enabled: bool) {
    state.set_developer_mode(enabled);
}

/// 開発者モードON時だけ、in-memory ring buffer の直近ログを返す。
#[tauri::command]
pub fn read_desktop_logs(
    state: tauri::State<'_, DeveloperLogState>,
    after_seq: Option<u64>,
    limit: Option<u32>,
) -> Result<DesktopLogSnapshot, CommandError> {
    state.read(after_seq, limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{DesktopStartupState, DesktopStartupStatus};
    use tauri::Manager;
    use tracing::Level;

    fn fixture_state() -> DeveloperLogState {
        let buffer = Arc::new(DesktopLogBuffer::with_limits(5, usize::MAX, usize::MAX));
        buffer.push(&Level::INFO, "kukuri", "startup".to_owned());
        buffer.push(&Level::WARN, "kukuri", "peer lost".to_owned());
        DeveloperLogState::new(buffer)
    }

    #[test]
    fn read_is_rejected_until_developer_mode_is_mirrored_and_limit_is_clamped() {
        let state = fixture_state();
        let rejected = state
            .read(None, None)
            .expect_err("developer mode is off by default");
        assert_eq!(rejected.code, DEVELOPER_MODE_DISABLED_CODE);
        assert!(!rejected.message.contains("startup"));

        state.set_developer_mode(true);
        let snapshot = state.read(None, None).expect("developer mode on");
        assert_eq!(snapshot.entries.len(), 2);
        assert_eq!(snapshot.max_entries, 5);
        assert_eq!(state.read(None, Some(1)).unwrap().entries.len(), 1);
        assert_eq!(
            state.read(Some(1), None).unwrap().entries[0].message,
            "peer lost"
        );
        // 0 は「既定の上限」として扱い、空応答にしない。
        assert_eq!(state.read(None, Some(0)).unwrap().entries.len(), 2);

        state.set_developer_mode(false);
        assert_eq!(
            state.read(None, None).expect_err("mirror off again").code,
            DEVELOPER_MODE_DISABLED_CODE
        );
    }

    #[test]
    fn ipc_round_trip_rejects_reads_while_mirror_is_off_and_serves_them_after_mirror() {
        let handler: fn(tauri::ipc::Invoke<tauri::test::MockRuntime>) -> bool =
            tauri::generate_handler![set_developer_mode_enabled, read_desktop_logs];
        let app = tauri::test::mock_builder()
            .manage(DesktopStartupState::initializing())
            .manage(fixture_state())
            .invoke_handler(crate::invoke_gate::with_desktop_startup_gate(handler))
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("mock app");
        let webview = tauri::WebviewWindowBuilder::new(&app, "review", Default::default())
            .build()
            .expect("mock webview");
        let request = |command: &str, body: serde_json::Value| tauri::webview::InvokeRequest {
            cmd: command.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: if cfg!(windows) {
                "http://tauri.localhost"
            } else {
                "tauri://localhost"
            }
            .parse()
            .unwrap(),
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        };

        // Ready 以外では gate が両 command を拒否し、mirror も変わらない。
        assert!(
            tauri::test::get_ipc_response(
                &webview,
                request(
                    "set_developer_mode_enabled",
                    serde_json::json!({"enabled": true})
                )
            )
            .is_err()
        );
        assert!(
            tauri::test::get_ipc_response(
                &webview,
                request("read_desktop_logs", serde_json::json!({}))
            )
            .is_err()
        );
        assert!(!app.state::<DeveloperLogState>().developer_mode_enabled());

        app.state::<DesktopStartupState>()
            .set_status(DesktopStartupStatus::Ready);
        let error = tauri::test::get_ipc_response(
            &webview,
            request("read_desktop_logs", serde_json::json!({})),
        )
        .expect_err("mirror is off");
        assert_eq!(error["code"], DEVELOPER_MODE_DISABLED_CODE);

        tauri::test::get_ipc_response(
            &webview,
            request(
                "set_developer_mode_enabled",
                serde_json::json!({"enabled": true}),
            ),
        )
        .expect("mirror on");
        let snapshot: serde_json::Value = tauri::test::get_ipc_response(
            &webview,
            request(
                "read_desktop_logs",
                serde_json::json!({"afterSeq": 1, "limit": 10}),
            ),
        )
        .expect("logs readable")
        .deserialize()
        .expect("snapshot json");
        assert_eq!(snapshot["entries"].as_array().unwrap().len(), 1);
        assert_eq!(snapshot["entries"][0]["message"], "peer lost");
        assert_eq!(snapshot["entries"][0]["level"], "WARN");
        assert_eq!(snapshot["oldest_seq"], 1);
        assert_eq!(snapshot["next_seq"], 3);

        tauri::test::get_ipc_response(
            &webview,
            request(
                "set_developer_mode_enabled",
                serde_json::json!({"enabled": false}),
            ),
        )
        .expect("mirror off");
        assert!(
            tauri::test::get_ipc_response(
                &webview,
                request("read_desktop_logs", serde_json::json!({}))
            )
            .is_err()
        );
    }
}
