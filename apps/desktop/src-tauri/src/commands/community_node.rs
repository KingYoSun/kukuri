use kukuri_desktop_runtime::{
    AcceptCommunityNodeConsentsRequest, CommunityNodeConfig, CommunityNodeNodeStatus,
    CommunityNodeTargetRequest, CreatePrivateChannelRequest, DiscoveryConfig,
    ExportChannelAccessTokenRequest, ExportFriendOnlyGrantRequest, ExportFriendPlusShareRequest,
    ExportPrivateChannelInviteRequest, FreezePrivateChannelRequest,
    ImportChannelAccessTokenRequest, ImportFriendOnlyGrantRequest, ImportFriendPlusShareRequest,
    ImportPeerTicketRequest, ImportPrivateChannelInviteRequest, ListJoinedPrivateChannelsRequest,
    RotatePrivateChannelRequest, SetCommunityNodeConfigRequest, SetDiscoverySeedsRequest,
    UnsubscribeTopicRequest,
};

use crate::state::{DesktopState, map_error};

#[tauri::command]
pub async fn create_private_channel(
    state: tauri::State<'_, DesktopState>,
    request: CreatePrivateChannelRequest,
) -> Result<kukuri_app_api::JoinedPrivateChannelView, String> {
    state
        .runtime
        .create_private_channel(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn export_private_channel_invite(
    state: tauri::State<'_, DesktopState>,
    request: ExportPrivateChannelInviteRequest,
) -> Result<String, String> {
    state
        .runtime
        .export_private_channel_invite(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn import_private_channel_invite(
    state: tauri::State<'_, DesktopState>,
    request: ImportPrivateChannelInviteRequest,
) -> Result<kukuri_core::PrivateChannelInvitePreview, String> {
    state
        .runtime
        .import_private_channel_invite(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn export_channel_access_token(
    state: tauri::State<'_, DesktopState>,
    request: ExportChannelAccessTokenRequest,
) -> Result<kukuri_app_api::ChannelAccessTokenExport, String> {
    state
        .runtime
        .export_channel_access_token(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn import_channel_access_token(
    state: tauri::State<'_, DesktopState>,
    request: ImportChannelAccessTokenRequest,
) -> Result<kukuri_app_api::ChannelAccessTokenPreview, String> {
    state
        .runtime
        .import_channel_access_token(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn export_friend_only_grant(
    state: tauri::State<'_, DesktopState>,
    request: ExportFriendOnlyGrantRequest,
) -> Result<String, String> {
    state
        .runtime
        .export_friend_only_grant(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn import_friend_only_grant(
    state: tauri::State<'_, DesktopState>,
    request: ImportFriendOnlyGrantRequest,
) -> Result<kukuri_core::FriendOnlyGrantPreview, String> {
    state
        .runtime
        .import_friend_only_grant(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn export_friend_plus_share(
    state: tauri::State<'_, DesktopState>,
    request: ExportFriendPlusShareRequest,
) -> Result<String, String> {
    state
        .runtime
        .export_friend_plus_share(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn import_friend_plus_share(
    state: tauri::State<'_, DesktopState>,
    request: ImportFriendPlusShareRequest,
) -> Result<kukuri_core::FriendPlusSharePreview, String> {
    state
        .runtime
        .import_friend_plus_share(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn freeze_private_channel(
    state: tauri::State<'_, DesktopState>,
    request: FreezePrivateChannelRequest,
) -> Result<kukuri_app_api::JoinedPrivateChannelView, String> {
    state
        .runtime
        .freeze_private_channel(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn rotate_private_channel(
    state: tauri::State<'_, DesktopState>,
    request: RotatePrivateChannelRequest,
) -> Result<kukuri_app_api::JoinedPrivateChannelView, String> {
    state
        .runtime
        .rotate_private_channel(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn list_joined_private_channels(
    state: tauri::State<'_, DesktopState>,
    request: ListJoinedPrivateChannelsRequest,
) -> Result<Vec<kukuri_app_api::JoinedPrivateChannelView>, String> {
    state
        .runtime
        .list_joined_private_channels(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn get_sync_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<kukuri_app_api::SyncStatus, String> {
    state.runtime.get_sync_status().await.map_err(map_error)
}

#[tauri::command]
pub async fn get_discovery_config(
    state: tauri::State<'_, DesktopState>,
) -> Result<DiscoveryConfig, String> {
    state.runtime.get_discovery_config().await.map_err(map_error)
}

#[tauri::command]
pub async fn import_peer_ticket(
    state: tauri::State<'_, DesktopState>,
    request: ImportPeerTicketRequest,
) -> Result<(), String> {
    state.runtime.import_peer_ticket(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn set_discovery_seeds(
    state: tauri::State<'_, DesktopState>,
    request: SetDiscoverySeedsRequest,
) -> Result<DiscoveryConfig, String> {
    state.runtime.set_discovery_seeds(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn unsubscribe_topic(
    state: tauri::State<'_, DesktopState>,
    request: UnsubscribeTopicRequest,
) -> Result<(), String> {
    state.runtime.unsubscribe_topic(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn get_local_peer_ticket(
    state: tauri::State<'_, DesktopState>,
) -> Result<Option<String>, String> {
    state.runtime.local_peer_ticket().await.map_err(map_error)
}

#[tauri::command]
pub async fn get_community_node_config(
    state: tauri::State<'_, DesktopState>,
) -> Result<CommunityNodeConfig, String> {
    state
        .runtime
        .get_community_node_config()
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn get_community_node_statuses(
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<CommunityNodeNodeStatus>, String> {
    state
        .runtime
        .get_community_node_statuses()
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn set_community_node_config(
    state: tauri::State<'_, DesktopState>,
    request: SetCommunityNodeConfigRequest,
) -> Result<CommunityNodeConfig, String> {
    state
        .runtime
        .set_community_node_config(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn clear_community_node_config(
    state: tauri::State<'_, DesktopState>,
) -> Result<(), String> {
    state
        .runtime
        .clear_community_node_config()
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn authenticate_community_node(
    state: tauri::State<'_, DesktopState>,
    request: CommunityNodeTargetRequest,
) -> Result<CommunityNodeNodeStatus, String> {
    state
        .runtime
        .authenticate_community_node(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn clear_community_node_token(
    state: tauri::State<'_, DesktopState>,
    request: CommunityNodeTargetRequest,
) -> Result<CommunityNodeNodeStatus, String> {
    state
        .runtime
        .clear_community_node_token(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn get_community_node_consent_status(
    state: tauri::State<'_, DesktopState>,
    request: CommunityNodeTargetRequest,
) -> Result<CommunityNodeNodeStatus, String> {
    state
        .runtime
        .get_community_node_consent_status(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn accept_community_node_consents(
    state: tauri::State<'_, DesktopState>,
    request: AcceptCommunityNodeConsentsRequest,
) -> Result<CommunityNodeNodeStatus, String> {
    state
        .runtime
        .accept_community_node_consents(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn refresh_community_node_metadata(
    state: tauri::State<'_, DesktopState>,
    request: CommunityNodeTargetRequest,
) -> Result<CommunityNodeNodeStatus, String> {
    state
        .runtime
        .refresh_community_node_metadata(request)
        .await
        .map_err(map_error)
}
