use crate::infrastructure::p2p::bootstrap_config::{self, BootstrapSelection};
use crate::presentation::dto::ApiResponse;
use crate::presentation::dto::community_node_dto::{
    CommunityNodeAuthRequest, CommunityNodeAuthResponse, CommunityNodeBootstrapServicesRequest,
    CommunityNodeConfigRequest, CommunityNodeConfigResponse, CommunityNodeConsentRequest,
    CommunityNodeLabelsRequest, CommunityNodeReportRequest, CommunityNodeSearchRequest,
    CommunityNodeTokenRequest, CommunityNodeTrustProviderRequest,
    CommunityNodeTrustProviderSelector, CommunityNodeTrustProviderState, CommunityNodeTrustRequest,
};
use crate::shared::AppError;
use crate::shared::config::BootstrapSource;
use crate::state::AppState;
use tauri::State;

fn has_user_bootstrap_nodes(selection: &BootstrapSelection) -> bool {
    selection.source == BootstrapSource::User && !selection.nodes.is_empty()
}

fn should_apply_runtime_bootstrap(
    previous: &BootstrapSelection,
    next: &BootstrapSelection,
) -> bool {
    if !has_user_bootstrap_nodes(next) {
        return false;
    }

    if previous.source != BootstrapSource::User {
        return true;
    }

    previous.nodes != next.nodes
}

fn should_retry_runtime_bootstrap_on_auth(next: &BootstrapSelection) -> bool {
    has_user_bootstrap_nodes(next)
}

#[tauri::command]
pub async fn set_community_node_config(
    state: State<'_, AppState>,
    request: CommunityNodeConfigRequest,
) -> Result<ApiResponse<CommunityNodeConfigResponse>, AppError> {
    let previous_selection = bootstrap_config::load_effective_bootstrap_nodes();
    let config = state.community_node_handler.set_config(request).await?;
    let next_selection = bootstrap_config::load_effective_bootstrap_nodes();

    if should_apply_runtime_bootstrap(&previous_selection, &next_selection)
        && let Err(err) = state
            .p2p_handler
            .apply_bootstrap_nodes(next_selection.nodes.clone(), next_selection.source)
            .await
    {
        tracing::warn!(
            error = %err,
            source = ?next_selection.source,
            node_count = next_selection.nodes.len(),
            "Failed to apply bootstrap nodes after community node config update"
        );
    }

    Ok(ApiResponse::success(config))
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
    let auth_response = state.community_node_handler.authenticate(request).await?;
    let next_selection = bootstrap_config::load_effective_bootstrap_nodes();

    if should_retry_runtime_bootstrap_on_auth(&next_selection)
        && let Err(err) = state
            .p2p_handler
            .apply_bootstrap_nodes(next_selection.nodes.clone(), next_selection.source)
            .await
    {
        tracing::warn!(
            error = %err,
            source = ?next_selection.source,
            node_count = next_selection.nodes.len(),
            "Failed to apply bootstrap nodes after community node authentication"
        );
    }

    Ok(ApiResponse::success(auth_response))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn selection(source: BootstrapSource, nodes: &[&str]) -> BootstrapSelection {
        BootstrapSelection {
            source,
            nodes: nodes.iter().map(|node| (*node).to_string()).collect(),
        }
    }

    #[test]
    fn should_apply_runtime_bootstrap_when_source_switches_to_user() {
        let previous = selection(BootstrapSource::None, &[]);
        let next = selection(
            BootstrapSource::User,
            &["aaaa@127.0.0.1:11223", "bbbb@127.0.0.1:11224"],
        );
        assert!(should_apply_runtime_bootstrap(&previous, &next));
    }

    #[test]
    fn should_apply_runtime_bootstrap_when_user_nodes_change() {
        let previous = selection(BootstrapSource::User, &["aaaa@127.0.0.1:11223"]);
        let next = selection(
            BootstrapSource::User,
            &["aaaa@127.0.0.1:11223", "bbbb@127.0.0.1:11224"],
        );
        assert!(should_apply_runtime_bootstrap(&previous, &next));
    }

    #[test]
    fn should_not_apply_runtime_bootstrap_when_user_nodes_unchanged() {
        let previous = selection(BootstrapSource::User, &["aaaa@127.0.0.1:11223"]);
        let next = selection(BootstrapSource::User, &["aaaa@127.0.0.1:11223"]);
        assert!(!should_apply_runtime_bootstrap(&previous, &next));
    }

    #[test]
    fn should_not_apply_runtime_bootstrap_for_non_user_source() {
        let previous = selection(BootstrapSource::None, &[]);
        let next = selection(BootstrapSource::Env, &["aaaa@127.0.0.1:11223"]);
        assert!(!should_apply_runtime_bootstrap(&previous, &next));
    }

    #[test]
    fn should_not_apply_runtime_bootstrap_when_user_nodes_empty() {
        let previous = selection(BootstrapSource::None, &[]);
        let next = selection(BootstrapSource::User, &[]);
        assert!(!should_apply_runtime_bootstrap(&previous, &next));
    }

    #[test]
    fn should_apply_runtime_bootstrap_when_source_switches_from_bundle_to_user() {
        let previous = selection(BootstrapSource::Bundle, &["n0@relay.example:11223"]);
        let next = selection(BootstrapSource::User, &["aaaa@127.0.0.1:11223"]);
        assert!(should_apply_runtime_bootstrap(&previous, &next));
    }

    #[test]
    fn should_retry_runtime_bootstrap_on_auth_when_user_nodes_exist() {
        let next = selection(BootstrapSource::User, &["aaaa@127.0.0.1:11223"]);
        assert!(should_retry_runtime_bootstrap_on_auth(&next));
    }

    #[test]
    fn should_not_retry_runtime_bootstrap_on_auth_without_user_source() {
        let next = selection(BootstrapSource::Bundle, &["n0@relay.example:11223"]);
        assert!(!should_retry_runtime_bootstrap_on_auth(&next));
    }

    #[test]
    fn should_not_retry_runtime_bootstrap_on_auth_when_user_nodes_empty() {
        let next = selection(BootstrapSource::User, &[]);
        assert!(!should_retry_runtime_bootstrap_on_auth(&next));
    }
}
