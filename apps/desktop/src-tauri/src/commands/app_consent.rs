use kukuri_desktop_runtime::{DeviceRestorePhase, pending_device_restore_phase};
use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::restore_lifecycle::{
    DesktopOperationState, RestoreActivationOrchestrationFailure, activate_pending_restore,
    orchestrate_restore_activation, rollback_pending_restore_and_rebuild,
};
use crate::spawn_desktop_initialization;
use crate::state::{
    AGE_ATTESTATION_VERSION, APP_LEGAL_DOCUMENTS, AgeAttestationRecord, AgeAttestationStatus,
    AppConsentDocumentRecord, AppConsentDocumentStatus, CommandError, DesktopStartupState,
    DesktopStartupStatus, DesktopState, StartupError, age_attestation_satisfied,
    age_attestation_status, app_consent_documents_status, app_consent_satisfied,
    build_desktop_state, current_unix_seconds, failed_status, load_app_consent_store,
    resolve_app_data_dir, resolve_db_path, save_app_consent_store,
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConsentStatus {
    pub documents: Vec<AppConsentDocumentStatus>,
    pub age_attestation: AgeAttestationStatus,
    pub satisfied: bool,
}

/// 同意リクエストの文書単位エントリ(#857)。ユーザーが実際に提示された slug と
/// 版をそのまま返してもらい、サーバ側(=このコマンド)で現行版と照合する。
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedAppConsentDocument {
    pub slug: String,
    pub version: i32,
}

fn set_activation_failed(
    app_handle: &tauri::AppHandle,
    startup: &DesktopStartupState,
    message: String,
) -> CommandError {
    startup.set_status(failed_status(
        StartupError::unknown(message.clone()),
        resolve_db_path(app_handle).ok(),
    ));
    CommandError::from(message)
}

fn require_consent_acceptance_state(status: &DesktopStartupStatus) -> Result<(), String> {
    if matches!(status, DesktopStartupStatus::ConsentRequired { .. }) {
        Ok(())
    } else {
        Err(format!(
            "app consent can only be accepted from ConsentRequired; current state is {status:?}"
        ))
    }
}

#[tauri::command]
pub fn get_app_consent_status(
    app_handle: tauri::AppHandle,
) -> Result<AppConsentStatus, CommandError> {
    let db_path = resolve_db_path(&app_handle)?;
    let store = load_app_consent_store(&db_path);
    Ok(AppConsentStatus {
        satisfied: app_consent_satisfied(&store),
        age_attestation: age_attestation_status(&store),
        documents: app_consent_documents_status(&store),
    })
}

#[tauri::command]
pub async fn accept_app_consents(
    app_handle: tauri::AppHandle,
    documents: Vec<AcceptedAppConsentDocument>,
    language: String,
    age_attested: bool,
) -> Result<DesktopStartupStatus, CommandError> {
    for (slug, current_version) in APP_LEGAL_DOCUMENTS {
        let accepted = documents
            .iter()
            .find(|document| document.slug == *slug)
            .ok_or_else(|| format!("consent for document `{slug}` is missing"))?;
        if accepted.version < *current_version {
            return Err(format!(
                "consent version {} for document `{slug}` is older than the current version {current_version}",
                accepted.version
            )
            .into());
        }
    }
    for document in &documents {
        if !APP_LEGAL_DOCUMENTS
            .iter()
            .any(|(slug, _)| *slug == document.slug)
        {
            return Err(format!("unknown consent document `{}`", document.slug).into());
        }
    }

    let operation = app_handle.state::<DesktopOperationState>();
    let _guard = operation.switch_guard.lock().await;
    let startup_state = app_handle.state::<DesktopStartupState>();
    require_consent_acceptance_state(&startup_state.status()).map_err(CommandError::from)?;
    let app_data_dir = resolve_app_data_dir(&app_handle)?;
    let pending_restore = pending_device_restore_phase(&app_data_dir).map_err(|error| {
        CommandError::from(format!(
            "failed to inspect pending device restore: {error:#}"
        ))
    })?;
    if let Some(phase) = pending_restore
        && phase != DeviceRestorePhase::AwaitingConsent
    {
        return Err(set_activation_failed(
            &app_handle,
            &startup_state,
            format!("device restore cannot accept consent from phase {phase:?}"),
        ));
    }

    let db_path = resolve_db_path(&app_handle)?;
    let mut store = load_app_consent_store(&db_path);

    // #858: 18歳以上の自己申告は文書同意とは別の必須行為。今回のリクエストで
    // 申告されたか、過去に現行版で申告済みのどちらかが必要(fail-closed)。
    if !age_attested && !age_attestation_satisfied(&store) {
        return Err("age attestation is required to use kukuri"
            .to_string()
            .into());
    }

    let accepted_at = current_unix_seconds();
    let app_version = app_handle.package_info().version.to_string();
    if age_attested {
        // 同一版の申告は日時等を更新し、それ以外は履歴として残す。
        if let Some(existing) = store
            .age_attestations
            .iter_mut()
            .find(|record| record.version == AGE_ATTESTATION_VERSION)
        {
            existing.attested_at = accepted_at;
            existing.language = language.clone();
            existing.app_version = app_version.clone();
        } else {
            store.age_attestations.push(AgeAttestationRecord {
                version: AGE_ATTESTATION_VERSION,
                attested_at: accepted_at,
                language: language.clone(),
                app_version: app_version.clone(),
            });
        }
    }
    for document in &documents {
        // 同一 slug+version の記録は日時等を更新し、それ以外は履歴として残す。
        if let Some(existing) = store
            .records
            .iter_mut()
            .find(|record| record.slug == document.slug && record.version == document.version)
        {
            existing.accepted_at = accepted_at;
            existing.language = language.clone();
            existing.app_version = app_version.clone();
        } else {
            store.records.push(AppConsentDocumentRecord {
                slug: document.slug.clone(),
                version: document.version,
                accepted_at,
                language: language.clone(),
                app_version: app_version.clone(),
            });
        }
    }
    save_app_consent_store(&db_path, &store)?;

    if pending_restore == Some(DeviceRestorePhase::AwaitingConsent) {
        startup_state.set_status(DesktopStartupStatus::Initializing);
        let activation = orchestrate_restore_activation(
            || async {
                build_desktop_state(&app_handle).await.map_err(|error| {
                    format!("failed to start restored account after consent: {error}")
                })
            },
            |restored| activate_pending_restore(&app_handle, &app_data_dir, restored),
            || rollback_pending_restore_and_rebuild(&app_handle, &app_data_dir),
        )
        .await;
        match activation {
            Ok(()) => {
                startup_state.set_status(DesktopStartupStatus::Ready);
                return Ok(DesktopStartupStatus::Ready);
            }
            Err(RestoreActivationOrchestrationFailure::RolledBack(message)) => {
                startup_state.set_status(DesktopStartupStatus::Ready);
                return Err(CommandError::from(message));
            }
            Err(
                RestoreActivationOrchestrationFailure::RollbackFailed(message)
                | RestoreActivationOrchestrationFailure::FinishForward(message),
            ) => {
                return Err(set_activation_failed(&app_handle, &startup_state, message));
            }
        }
    }

    if app_handle.try_state::<DesktopState>().is_some() {
        startup_state.set_status(DesktopStartupStatus::Ready);
        return Ok(DesktopStartupStatus::Ready);
    }

    startup_state.set_status(DesktopStartupStatus::Initializing);
    spawn_desktop_initialization(app_handle.clone())
        .await
        .map_err(|error| error.to_string().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{StartupError, consent_required_status, failed_status};

    #[test]
    fn consent_acceptance_is_only_allowed_from_consent_required() {
        assert!(
            require_consent_acceptance_state(&consent_required_status(&Default::default())).is_ok()
        );
        for status in [
            DesktopStartupStatus::Initializing,
            DesktopStartupStatus::Ready,
            failed_status(StartupError::unknown("failed".to_string()), None),
        ] {
            assert!(require_consent_acceptance_state(&status).is_err());
        }
    }
}
