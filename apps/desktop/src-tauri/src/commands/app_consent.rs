use serde::Serialize;
use tauri::Manager;

use crate::state::{
    AppConsentRecord, DesktopStartupState, DesktopStartupStatus, DesktopState, LEGAL_BUNDLE_VERSION,
    build_desktop_state, consent_satisfied, current_unix_seconds, failed_status, load_app_consent,
    resolve_db_path, save_app_consent,
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConsentStatus {
    pub current_bundle_version: i32,
    pub accepted_bundle_version: Option<i32>,
    pub accepted_at: Option<i64>,
    pub satisfied: bool,
}

#[tauri::command]
pub fn get_app_consent_status(app_handle: tauri::AppHandle) -> Result<AppConsentStatus, String> {
    let db_path = resolve_db_path(&app_handle)?;
    let record = load_app_consent(&db_path);
    let accepted_bundle_version = record
        .as_ref()
        .map(|record| record.accepted_bundle_version);
    Ok(AppConsentStatus {
        current_bundle_version: LEGAL_BUNDLE_VERSION,
        accepted_bundle_version,
        accepted_at: record.map(|record| record.accepted_at),
        satisfied: consent_satisfied(accepted_bundle_version),
    })
}

#[tauri::command]
pub fn accept_app_consents(
    app_handle: tauri::AppHandle,
    bundle_version: i32,
) -> Result<DesktopStartupStatus, String> {
    if bundle_version < LEGAL_BUNDLE_VERSION {
        return Err(format!(
            "consent bundle version {bundle_version} is older than the current version {LEGAL_BUNDLE_VERSION}"
        ));
    }

    let db_path = resolve_db_path(&app_handle)?;
    save_app_consent(
        &db_path,
        &AppConsentRecord {
            accepted_bundle_version: bundle_version,
            accepted_at: current_unix_seconds(),
        },
    )?;

    let startup_state = app_handle.state::<DesktopStartupState>();

    if app_handle.try_state::<DesktopState>().is_some() {
        startup_state.set_status(DesktopStartupStatus::Ready);
        return Ok(DesktopStartupStatus::Ready);
    }

    match build_desktop_state(&app_handle) {
        Ok(state) => {
            app_handle.manage(state);
            startup_state.set_status(DesktopStartupStatus::Ready);
            Ok(DesktopStartupStatus::Ready)
        }
        Err(error) => {
            let db_path = resolve_db_path(&app_handle).ok();
            let status = failed_status(error, db_path);
            startup_state.set_status(status.clone());
            Ok(status)
        }
    }
}
