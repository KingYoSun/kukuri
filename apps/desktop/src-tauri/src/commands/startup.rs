use crate::state::{DesktopStartupState, DesktopStartupStatus};

#[tauri::command]
pub fn get_desktop_startup_status(
    state: tauri::State<'_, DesktopStartupState>,
) -> DesktopStartupStatus {
    state.status()
}
