use tauri::{Runtime, ipc::Invoke};

use crate::state::{CommandError, DesktopStartupState, DesktopStartupStatus};

const NON_READY_COMMAND_ALLOWLIST: &[&str] = &[
    "get_desktop_startup_status",
    "get_app_consent_status",
    "accept_app_consents",
    "cancel_device_backup",
];

fn command_allowed(command: &str, status: &DesktopStartupStatus) -> bool {
    matches!(status, DesktopStartupStatus::Ready) || NON_READY_COMMAND_ALLOWLIST.contains(&command)
}

/// `generate_handler!`へ登録した全app commandを、引数解析やcommand本体より前に
/// 同じstartup gateへ通す。
pub(crate) fn with_desktop_startup_gate<R, F>(
    handler: F,
) -> impl Fn(Invoke<R>) -> bool + Send + Sync + 'static
where
    R: Runtime,
    F: Fn(Invoke<R>) -> bool + Send + Sync + 'static,
{
    move |invoke| {
        let command = invoke.message.command().to_string();
        let status = invoke
            .message
            .state_ref()
            .try_get::<DesktopStartupState>()
            .map(|startup| startup.status());
        let allowed = status
            .as_ref()
            .is_some_and(|status| command_allowed(&command, status));
        if allowed {
            return handler(invoke);
        }

        let detail = status
            .map(|status| format!("current state is {status:?}"))
            .unwrap_or_else(|| "startup state is unavailable".to_string());
        invoke.resolver.reject(CommandError::from(format!(
            "desktop command `{command}` requires Ready startup state; {detail}"
        )));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::consent_required_status;

    #[test]
    fn consent_required_rejects_network_sink_and_ready_allows_it() {
        let consent_required = consent_required_status(&Default::default());

        assert!(!command_allowed(
            "fetch_community_node_policies",
            &consent_required
        ));
        assert!(command_allowed(
            "fetch_community_node_policies",
            &DesktopStartupStatus::Ready
        ));
    }

    #[test]
    fn non_ready_allowlist_is_minimal_and_restore_frontend_state_remains_gated() {
        let status = DesktopStartupStatus::Initializing;
        assert_eq!(
            NON_READY_COMMAND_ALLOWLIST,
            [
                "get_desktop_startup_status",
                "get_app_consent_status",
                "accept_app_consents",
                "cancel_device_backup",
            ]
        );
        for command in NON_READY_COMMAND_ALLOWLIST {
            assert!(command_allowed(command, &status), "{command}");
        }
        for command in [
            "get_pending_device_restore_frontend_state",
            "acknowledge_pending_device_restore_frontend_state",
            "preview_device_backup_command",
            "list_accounts",
        ] {
            assert!(!command_allowed(command, &status), "{command}");
        }
    }
}
