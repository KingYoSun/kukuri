use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::spawn_desktop_initialization;
use crate::state::{
    AGE_ATTESTATION_VERSION, APP_LEGAL_DOCUMENTS, AgeAttestationRecord, AgeAttestationStatus,
    AppConsentDocumentRecord, AppConsentDocumentStatus, CommandError, DesktopStartupState,
    DesktopStartupStatus, DesktopState, age_attestation_satisfied, age_attestation_status,
    app_consent_documents_status, app_consent_satisfied, current_unix_seconds,
    load_app_consent_store, resolve_db_path, save_app_consent_store,
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

    let startup_state = app_handle.state::<DesktopStartupState>();

    if app_handle.try_state::<DesktopState>().is_some() {
        startup_state.set_status(DesktopStartupStatus::Ready);
        return Ok(DesktopStartupStatus::Ready);
    }

    startup_state.set_status(DesktopStartupStatus::Initializing);
    spawn_desktop_initialization(app_handle)
        .await
        .map_err(|error| error.to_string().into())
}
