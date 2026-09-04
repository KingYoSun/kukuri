use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    AGE_ATTESTATION_VERSION, APP_LEGAL_DOCUMENTS, AgeAttestationRecord, AgeAttestationStatus,
    AppConsentDocumentRecord, AppConsentDocumentStatus, ClientStartupStatus,
    age_attestation_satisfied, age_attestation_status, app_consent_documents_status,
    app_consent_satisfied, current_unix_seconds, load_app_consent_store, save_app_consent_store,
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

pub fn require_consent_acceptance_state(status: &ClientStartupStatus) -> Result<(), String> {
    if matches!(status, ClientStartupStatus::ConsentRequired { .. }) {
        Ok(())
    } else {
        Err(format!(
            "app consent can only be accepted from ConsentRequired; current state is {status:?}"
        ))
    }
}

pub fn validate_app_consent_documents(
    documents: &[AcceptedAppConsentDocument],
) -> Result<(), String> {
    for (slug, current_version) in APP_LEGAL_DOCUMENTS {
        let accepted = documents
            .iter()
            .find(|document| document.slug == *slug)
            .ok_or_else(|| format!("consent for document `{slug}` is missing"))?;
        if accepted.version < *current_version {
            return Err(format!(
                "consent version {} for document `{slug}` is older than the current version {current_version}",
                accepted.version
            ));
        }
    }
    for document in documents {
        if !APP_LEGAL_DOCUMENTS
            .iter()
            .any(|(slug, _)| *slug == document.slug)
        {
            return Err(format!("unknown consent document `{}`", document.slug));
        }
    }

    Ok(())
}

pub fn record_app_consents(
    db_path: &Path,
    documents: &[AcceptedAppConsentDocument],
    language: &str,
    age_attested: bool,
    app_version: &str,
) -> Result<(), String> {
    let mut store = load_app_consent_store(db_path);

    // #858: 18歳以上の自己申告は文書同意とは別の必須行為。今回のリクエストで
    // 申告されたか、過去に現行版で申告済みのどちらかが必要(fail-closed)。
    if !age_attested && !age_attestation_satisfied(&store) {
        return Err("age attestation is required to use kukuri".to_string());
    }

    let accepted_at = current_unix_seconds();
    if age_attested {
        // 同一版の申告は日時等を更新し、それ以外は履歴として残す。
        if let Some(existing) = store
            .age_attestations
            .iter_mut()
            .find(|record| record.version == AGE_ATTESTATION_VERSION)
        {
            existing.attested_at = accepted_at;
            existing.language = language.to_string();
            existing.app_version = app_version.to_string();
        } else {
            store.age_attestations.push(AgeAttestationRecord {
                version: AGE_ATTESTATION_VERSION,
                attested_at: accepted_at,
                language: language.to_string(),
                app_version: app_version.to_string(),
            });
        }
    }
    for document in documents {
        // 同一 slug+version の記録は日時等を更新し、それ以外は履歴として残す。
        if let Some(existing) = store
            .records
            .iter_mut()
            .find(|record| record.slug == document.slug && record.version == document.version)
        {
            existing.accepted_at = accepted_at;
            existing.language = language.to_string();
            existing.app_version = app_version.to_string();
        } else {
            store.records.push(AppConsentDocumentRecord {
                slug: document.slug.clone(),
                version: document.version,
                accepted_at,
                language: language.to_string(),
                app_version: app_version.to_string(),
            });
        }
    }
    save_app_consent_store(db_path, &store)?;

    Ok(())
}

pub fn app_consent_status(db_path: &Path) -> AppConsentStatus {
    let store = load_app_consent_store(db_path);
    AppConsentStatus {
        satisfied: app_consent_satisfied(&store),
        age_attestation: age_attestation_status(&store),
        documents: app_consent_documents_status(&store),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{
        ClientStartupError, consent_required_status, failed_startup_status as failed_status,
    };

    #[test]
    fn consent_acceptance_is_only_allowed_from_consent_required() {
        assert!(
            require_consent_acceptance_state(&consent_required_status(&Default::default())).is_ok()
        );
        for status in [
            ClientStartupStatus::Initializing,
            ClientStartupStatus::Ready,
            failed_status(ClientStartupError::unknown("failed".to_string()), None),
        ] {
            assert!(require_consent_acceptance_state(&status).is_err());
        }
    }
}
