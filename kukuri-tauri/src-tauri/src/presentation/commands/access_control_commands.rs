use crate::application::services::JoinRequestInput;
use crate::presentation::dto::ApiResponse;
use crate::presentation::dto::Validate;
use crate::presentation::dto::access_control_dto::{
    AccessControlIssueInviteRequest, AccessControlIssueInviteResponse, AccessControlJoinRequest,
    AccessControlJoinResponse,
};
use crate::shared::AppError;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn access_control_issue_invite(
    state: State<'_, AppState>,
    request: AccessControlIssueInviteRequest,
) -> Result<ApiResponse<AccessControlIssueInviteResponse>, AppError> {
    request
        .validate()
        .map_err(|err| AppError::validation(crate::shared::ValidationFailureKind::Generic, err))?;

    let invite_event_json = state
        .access_control_service
        .issue_invite(
            request.topic_id.trim(),
            request.expires_in,
            request.max_uses,
            request.nonce,
        )
        .await?;

    Ok(ApiResponse::success(AccessControlIssueInviteResponse {
        invite_event_json,
    }))
}

#[tauri::command]
pub async fn access_control_request_join(
    state: State<'_, AppState>,
    request: AccessControlJoinRequest,
) -> Result<ApiResponse<AccessControlJoinResponse>, AppError> {
    request
        .validate()
        .map_err(|err| AppError::validation(crate::shared::ValidationFailureKind::Generic, err))?;

    let result = state
        .access_control_service
        .request_join(JoinRequestInput {
            topic_id: request.topic_id,
            scope: request.scope,
            invite_event_json: request.invite_event_json,
            target_pubkey: request.target_pubkey,
            broadcast_to_topic: request.broadcast_to_topic.unwrap_or(false),
        })
        .await?;

    Ok(ApiResponse::success(AccessControlJoinResponse {
        event_id: result.event_id,
        sent_topics: result.sent_topics,
    }))
}
