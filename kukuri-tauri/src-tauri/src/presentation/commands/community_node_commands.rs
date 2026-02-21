use crate::presentation::dto::ApiResponse;
use crate::presentation::dto::community_node_dto::{
    CommunityNodeAuthRequest, CommunityNodeAuthResponse, CommunityNodeBootstrapServicesRequest,
    CommunityNodeConfigRequest, CommunityNodeConfigResponse, CommunityNodeConsentRequest,
    CommunityNodeLabelsRequest, CommunityNodeReportRequest, CommunityNodeSearchRequest,
    CommunityNodeTokenRequest, CommunityNodeTrustProviderRequest,
    CommunityNodeTrustProviderSelector, CommunityNodeTrustProviderState, CommunityNodeTrustRequest,
};
use crate::shared::AppError;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn set_community_node_config(
    state: State<'_, AppState>,
    request: CommunityNodeConfigRequest,
) -> Result<ApiResponse<CommunityNodeConfigResponse>, AppError> {
    let result = state.community_node_handler.set_config(request).await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn get_community_node_config(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Option<CommunityNodeConfigResponse>>, AppError> {
    let result = state.community_node_handler.get_config().await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn clear_community_node_config(
    state: State<'_, AppState>,
) -> Result<ApiResponse<()>, AppError> {
    let result = state.community_node_handler.clear_config().await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn community_node_authenticate(
    state: State<'_, AppState>,
    request: CommunityNodeAuthRequest,
) -> Result<ApiResponse<CommunityNodeAuthResponse>, AppError> {
    let result = state.community_node_handler.authenticate(request).await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn community_node_clear_token(
    state: State<'_, AppState>,
    request: CommunityNodeTokenRequest,
) -> Result<ApiResponse<()>, AppError> {
    let result = state.community_node_handler.clear_token(request).await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn community_node_get_trust_provider(
    state: State<'_, AppState>,
    request: Option<CommunityNodeTrustProviderSelector>,
) -> Result<ApiResponse<Option<CommunityNodeTrustProviderState>>, AppError> {
    let result = state
        .community_node_handler
        .get_trust_provider(request)
        .await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn community_node_set_trust_provider(
    state: State<'_, AppState>,
    request: CommunityNodeTrustProviderRequest,
) -> Result<ApiResponse<CommunityNodeTrustProviderState>, AppError> {
    let result = state
        .community_node_handler
        .set_trust_provider(request)
        .await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn community_node_clear_trust_provider(
    state: State<'_, AppState>,
    request: Option<CommunityNodeTrustProviderSelector>,
) -> Result<ApiResponse<()>, AppError> {
    let result = state
        .community_node_handler
        .clear_trust_provider(request)
        .await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn community_node_list_group_keys(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<crate::application::ports::group_key_store::GroupKeyEntry>>, AppError> {
    let result = state.community_node_handler.list_group_keys().await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn community_node_list_labels(
    state: State<'_, AppState>,
    request: CommunityNodeLabelsRequest,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let result = state.community_node_handler.list_labels(request).await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn community_node_submit_report(
    state: State<'_, AppState>,
    request: CommunityNodeReportRequest,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let result = state.community_node_handler.submit_report(request).await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn community_node_trust_report_based(
    state: State<'_, AppState>,
    request: CommunityNodeTrustRequest,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let result = state
        .community_node_handler
        .trust_report_based(request)
        .await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn community_node_trust_communication_density(
    state: State<'_, AppState>,
    request: CommunityNodeTrustRequest,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let result = state
        .community_node_handler
        .trust_communication_density(request)
        .await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn community_node_search(
    state: State<'_, AppState>,
    request: CommunityNodeSearchRequest,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let result = state.community_node_handler.search(request).await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn community_node_list_bootstrap_nodes(
    state: State<'_, AppState>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let result = state.community_node_handler.list_bootstrap_nodes().await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn community_node_list_bootstrap_services(
    state: State<'_, AppState>,
    request: CommunityNodeBootstrapServicesRequest,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let result = state
        .community_node_handler
        .list_bootstrap_services(request)
        .await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn community_node_get_consent_status(
    state: State<'_, AppState>,
    request: CommunityNodeTokenRequest,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let result = state
        .community_node_handler
        .get_consent_status(request)
        .await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn community_node_accept_consents(
    state: State<'_, AppState>,
    request: CommunityNodeConsentRequest,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let result = state.community_node_handler.accept_consents(request).await;
    Ok(ApiResponse::from_result(result))
}
