use axum::extract::{Path, State};
use axum::{Json, response::IntoResponse};
use serde::Serialize;

use kukuri_cn_core::get_active_transmission_prevention;

use crate::errors::internal_error;
use crate::state::UserApiState;

#[derive(Serialize)]
pub(crate) struct TransmissionPreventionStatus {
    subject_kind: String,
    subject_id: String,
    active: bool,
    basis: Option<String>,
    capabilities: Vec<String>,
    decided_at: Option<String>,
    expires_at: Option<String>,
    appeal_path: String,
}

pub(crate) async fn transmission_prevention_status(
    State(state): State<UserApiState>,
    Path((subject_kind, subject_id)): Path<(String, String)>,
) -> impl IntoResponse {
    match get_active_transmission_prevention(&state.pool, &subject_kind, &subject_id).await {
        Ok(value) => {
            let active = value.is_some();
            let basis = value.as_ref().map(|item| item.basis.as_str().to_string());
            let capabilities = value
                .as_ref()
                .map(|item| {
                    item.capabilities
                        .iter()
                        .map(|capability| capability.as_str().to_string())
                        .collect()
                })
                .unwrap_or_default();
            let decided_at = value.as_ref().map(|item| item.decided_at.to_rfc3339());
            let expires_at = value
                .and_then(|item| item.expires_at)
                .map(|item| item.to_rfc3339());
            Json(TransmissionPreventionStatus {
                subject_kind,
                subject_id,
                active,
                basis,
                capabilities,
                decided_at,
                expires_at,
                appeal_path: "/v1/report".to_string(),
            })
            .into_response()
        }
        Err(error) => internal_error(error).into_response(),
    }
}
