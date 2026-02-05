use crate::application::ports::join_request_store::JoinRequestRecord;
use crate::application::services::JoinRequestInput;
use crate::presentation::dto::ApiResponse;
use crate::presentation::dto::Validate;
use crate::presentation::dto::access_control_dto::{
    AccessControlApproveJoinRequest, AccessControlApproveJoinResponse,
    AccessControlIssueInviteRequest, AccessControlIssueInviteResponse, AccessControlJoinRequest,
    AccessControlJoinResponse, AccessControlListJoinRequestsResponse,
    AccessControlPendingJoinRequest, AccessControlRejectJoinRequest,
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

#[tauri::command]
pub async fn access_control_list_join_requests(
    state: State<'_, AppState>,
) -> Result<ApiResponse<AccessControlListJoinRequestsResponse>, AppError> {
    let records = state
        .access_control_service
        .list_pending_join_requests()
        .await?;
    let items = records
        .into_iter()
        .map(map_pending_join_request)
        .collect::<Vec<_>>();
    Ok(ApiResponse::success(
        AccessControlListJoinRequestsResponse { items },
    ))
}

#[tauri::command]
pub async fn access_control_approve_join_request(
    state: State<'_, AppState>,
    request: AccessControlApproveJoinRequest,
) -> Result<ApiResponse<AccessControlApproveJoinResponse>, AppError> {
    request
        .validate()
        .map_err(|err| AppError::validation(crate::shared::ValidationFailureKind::Generic, err))?;

    let result = state
        .access_control_service
        .approve_join_request(&request.event_id)
        .await?;

    Ok(ApiResponse::success(AccessControlApproveJoinResponse {
        event_id: result.event_id,
        key_envelope_event_id: result.key_envelope_event_id,
    }))
}

#[tauri::command]
pub async fn access_control_reject_join_request(
    state: State<'_, AppState>,
    request: AccessControlRejectJoinRequest,
) -> Result<ApiResponse<()>, AppError> {
    request
        .validate()
        .map_err(|err| AppError::validation(crate::shared::ValidationFailureKind::Generic, err))?;
    state
        .access_control_service
        .reject_join_request(&request.event_id)
        .await?;
    Ok(ApiResponse::success(()))
}

fn map_pending_join_request(record: JoinRequestRecord) -> AccessControlPendingJoinRequest {
    AccessControlPendingJoinRequest {
        event_id: record.event.id,
        topic_id: record.topic_id,
        scope: record.scope,
        requester_pubkey: record.requester_pubkey,
        target_pubkey: record.target_pubkey,
        requested_at: record.requested_at,
        received_at: record.received_at,
        invite_event_json: record.invite_event_json,
    }
}
