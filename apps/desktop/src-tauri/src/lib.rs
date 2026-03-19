use std::sync::Arc;

use kukuri_desktop_runtime::{
    AcceptCommunityNodeConsentsRequest, CommunityNodeConfig, CommunityNodeNodeStatus,
    CommunityNodeTargetRequest, CreateGameRoomRequest, CreateLiveSessionRequest,
    CreatePostRequest, CreatePrivateChannelRequest, DesktopRuntime, DiscoveryConfig,
    ExportFriendOnlyGrantRequest, ExportPrivateChannelInviteRequest, GetBlobMediaRequest,
    GetBlobPreviewRequest, ImportFriendOnlyGrantRequest, ImportPeerTicketRequest,
    ImportPrivateChannelInviteRequest, ListGameRoomsRequest, ListJoinedPrivateChannelsRequest,
    ListLiveSessionsRequest, ListThreadRequest, ListTimelineRequest, LiveSessionCommandRequest,
    RotatePrivateChannelRequest, SetCommunityNodeConfigRequest, SetDiscoverySeedsRequest,
    SetMyProfileRequest, UnsubscribeTopicRequest, UpdateGameRoomRequest, AuthorRequest,
    resolve_db_path_from_env,
};
use tauri::Manager;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

struct DesktopState {
    runtime: Arc<DesktopRuntime>,
}

const DEFAULT_TRACING_DIRECTIVES: &str =
    "warn,kukuri_desktop_tauri_lib=info,kukuri_app_api=info";
const DEFAULT_SUPPRESS_DIRECTIVES: &[&str] = &[
    "mainline::rpc::socket=error",
    "iroh_quinn_proto::connection=error",
    "iroh::socket::remote_map::remote_state=error",
    "iroh_docs::engine::live=error",
    "iroh_gossip::net=error",
];

fn map_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn resolve_tracing_directives(rust_log: Option<&str>) -> String {
    let directives = rust_log
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_TRACING_DIRECTIVES)
        .to_owned();

    let mut resolved = directives.clone();
    for suppress_directive in DEFAULT_SUPPRESS_DIRECTIVES {
        let target = suppress_directive
            .split('=')
            .next()
            .expect("suppress directives must have a target");
        if directives_contains_target(&directives, target) {
            continue;
        }
        resolved.push(',');
        resolved.push_str(suppress_directive);
    }

    resolved
}

fn directives_contains_target(directives: &str, target: &str) -> bool {
    directives
        .split(',')
        .map(str::trim)
        .filter(|directive| !directive.is_empty())
        .any(|directive| directive == target || directive.starts_with(&format!("{target}=")))
}

fn init_tracing() {
    let env_filter = EnvFilter::new(resolve_tracing_directives(
        std::env::var("RUST_LOG").ok().as_deref(),
    ));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .try_init();
}

fn resolve_db_path(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data dir: {error}"))?;
    resolve_db_path_from_env(&app_data_dir).map_err(map_error)
}

#[tauri::command]
async fn create_post(
    state: tauri::State<'_, DesktopState>,
    request: CreatePostRequest,
) -> Result<String, String> {
    state.runtime.create_post(request).await.map_err(map_error)
}

#[tauri::command]
async fn create_private_channel(
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
async fn export_private_channel_invite(
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
async fn import_private_channel_invite(
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
async fn export_friend_only_grant(
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
async fn import_friend_only_grant(
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
async fn rotate_private_channel(
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
async fn list_joined_private_channels(
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
async fn list_timeline(
    state: tauri::State<'_, DesktopState>,
    request: ListTimelineRequest,
) -> Result<kukuri_app_api::TimelineView, String> {
    state
        .runtime
        .list_timeline(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn list_thread(
    state: tauri::State<'_, DesktopState>,
    request: ListThreadRequest,
) -> Result<kukuri_app_api::TimelineView, String> {
    state.runtime.list_thread(request).await.map_err(map_error)
}

#[tauri::command]
async fn get_my_profile(
    state: tauri::State<'_, DesktopState>,
) -> Result<kukuri_core::Profile, String> {
    state.runtime.get_my_profile().await.map_err(map_error)
}

#[tauri::command]
async fn set_my_profile(
    state: tauri::State<'_, DesktopState>,
    request: SetMyProfileRequest,
) -> Result<kukuri_core::Profile, String> {
    state.runtime.set_my_profile(request).await.map_err(map_error)
}

#[tauri::command]
async fn follow_author(
    state: tauri::State<'_, DesktopState>,
    request: AuthorRequest,
) -> Result<kukuri_app_api::AuthorSocialView, String> {
    state.runtime.follow_author(request).await.map_err(map_error)
}

#[tauri::command]
async fn unfollow_author(
    state: tauri::State<'_, DesktopState>,
    request: AuthorRequest,
) -> Result<kukuri_app_api::AuthorSocialView, String> {
    state.runtime.unfollow_author(request).await.map_err(map_error)
}

#[tauri::command]
async fn get_author_social_view(
    state: tauri::State<'_, DesktopState>,
    request: AuthorRequest,
) -> Result<kukuri_app_api::AuthorSocialView, String> {
    state
        .runtime
        .get_author_social_view(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn get_sync_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<kukuri_app_api::SyncStatus, String> {
    state.runtime.get_sync_status().await.map_err(map_error)
}

#[tauri::command]
async fn get_discovery_config(
    state: tauri::State<'_, DesktopState>,
) -> Result<DiscoveryConfig, String> {
    state.runtime.get_discovery_config().await.map_err(map_error)
}

#[tauri::command]
async fn list_live_sessions(
    state: tauri::State<'_, DesktopState>,
    request: ListLiveSessionsRequest,
) -> Result<Vec<kukuri_app_api::LiveSessionView>, String> {
    state
        .runtime
        .list_live_sessions(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn create_live_session(
    state: tauri::State<'_, DesktopState>,
    request: CreateLiveSessionRequest,
) -> Result<String, String> {
    state
        .runtime
        .create_live_session(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn end_live_session(
    state: tauri::State<'_, DesktopState>,
    request: LiveSessionCommandRequest,
) -> Result<(), String> {
    state
        .runtime
        .end_live_session(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn join_live_session(
    state: tauri::State<'_, DesktopState>,
    request: LiveSessionCommandRequest,
) -> Result<(), String> {
    state
        .runtime
        .join_live_session(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn leave_live_session(
    state: tauri::State<'_, DesktopState>,
    request: LiveSessionCommandRequest,
) -> Result<(), String> {
    state
        .runtime
        .leave_live_session(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn list_game_rooms(
    state: tauri::State<'_, DesktopState>,
    request: ListGameRoomsRequest,
) -> Result<Vec<kukuri_app_api::GameRoomView>, String> {
    state
        .runtime
        .list_game_rooms(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn create_game_room(
    state: tauri::State<'_, DesktopState>,
    request: CreateGameRoomRequest,
) -> Result<String, String> {
    state
        .runtime
        .create_game_room(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn update_game_room(
    state: tauri::State<'_, DesktopState>,
    request: UpdateGameRoomRequest,
) -> Result<(), String> {
    state
        .runtime
        .update_game_room(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn import_peer_ticket(
    state: tauri::State<'_, DesktopState>,
    request: ImportPeerTicketRequest,
) -> Result<(), String> {
    state
        .runtime
        .import_peer_ticket(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn set_discovery_seeds(
    state: tauri::State<'_, DesktopState>,
    request: SetDiscoverySeedsRequest,
) -> Result<DiscoveryConfig, String> {
    state
        .runtime
        .set_discovery_seeds(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn unsubscribe_topic(
    state: tauri::State<'_, DesktopState>,
    request: UnsubscribeTopicRequest,
) -> Result<(), String> {
    state
        .runtime
        .unsubscribe_topic(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn get_local_peer_ticket(
    state: tauri::State<'_, DesktopState>,
) -> Result<Option<String>, String> {
    state.runtime.local_peer_ticket().await.map_err(map_error)
}

#[tauri::command]
async fn get_blob_preview_url(
    state: tauri::State<'_, DesktopState>,
    request: GetBlobPreviewRequest,
) -> Result<Option<String>, String> {
    state
        .runtime
        .get_blob_preview_url(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn get_blob_media_payload(
    state: tauri::State<'_, DesktopState>,
    request: GetBlobMediaRequest,
) -> Result<Option<kukuri_app_api::BlobMediaPayload>, String> {
    let hash = request.hash.clone();
    let mime = request.mime.clone();
    info!(hash = %hash, mime = %mime, "received get_blob_media_payload command");
    match state.runtime.get_blob_media_payload(request).await {
        Ok(Some(payload)) => {
            info!(
                hash = %hash,
                mime = %mime,
                bytes_base64_len = payload.bytes_base64.len(),
                "returning get_blob_media_payload response"
            );
            Ok(Some(payload))
        }
        Ok(None) => {
            warn!(hash = %hash, mime = %mime, "get_blob_media_payload returned no blob");
            Ok(None)
        }
        Err(error) => {
            let error_message = map_error(error);
            warn!(
                hash = %hash,
                mime = %mime,
                error = %error_message,
                "get_blob_media_payload command failed"
            );
            Err(error_message)
        }
    }
}

#[tauri::command]
async fn get_community_node_config(
    state: tauri::State<'_, DesktopState>,
) -> Result<CommunityNodeConfig, String> {
    state
        .runtime
        .get_community_node_config()
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn get_community_node_statuses(
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<CommunityNodeNodeStatus>, String> {
    state
        .runtime
        .get_community_node_statuses()
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn set_community_node_config(
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
async fn clear_community_node_config(
    state: tauri::State<'_, DesktopState>,
) -> Result<(), String> {
    state
        .runtime
        .clear_community_node_config()
        .await
        .map_err(map_error)
}

#[tauri::command]
async fn authenticate_community_node(
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
async fn clear_community_node_token(
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
async fn get_community_node_consent_status(
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
async fn accept_community_node_consents(
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
async fn refresh_community_node_metadata(
    state: tauri::State<'_, DesktopState>,
    request: CommunityNodeTargetRequest,
) -> Result<CommunityNodeNodeStatus, String> {
    state
        .runtime
        .refresh_community_node_metadata(request)
        .await
        .map_err(map_error)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .setup(|app| {
            let db_path = resolve_db_path(app.handle())?;
            let runtime = tauri::async_runtime::block_on(DesktopRuntime::from_env(db_path))
                .map_err(map_error)?;
            info!("initialized kukuri desktop runtime");
            app.manage(DesktopState {
                runtime: Arc::new(runtime),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_post,
            create_private_channel,
            export_private_channel_invite,
            import_private_channel_invite,
            export_friend_only_grant,
            import_friend_only_grant,
            rotate_private_channel,
            list_joined_private_channels,
            list_timeline,
            list_thread,
            get_my_profile,
            set_my_profile,
            follow_author,
            unfollow_author,
            get_author_social_view,
            get_sync_status,
            get_discovery_config,
            list_live_sessions,
            create_live_session,
            end_live_session,
            join_live_session,
            leave_live_session,
            list_game_rooms,
            create_game_room,
            update_game_room,
            import_peer_ticket,
            set_discovery_seeds,
            unsubscribe_topic,
            get_local_peer_ticket,
            get_blob_media_payload,
            get_blob_preview_url,
            get_community_node_config,
            get_community_node_statuses,
            set_community_node_config,
            clear_community_node_config,
            authenticate_community_node,
            clear_community_node_token,
            get_community_node_consent_status,
            accept_community_node_consents,
            refresh_community_node_metadata
        ])
        .run(tauri::generate_context!())
        .expect("failed to run kukuri desktop tauri app");
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_SUPPRESS_DIRECTIVES, DEFAULT_TRACING_DIRECTIVES,
        resolve_tracing_directives,
    };

    #[test]
    fn default_tracing_directives_add_noise_suppression() {
        let directives = resolve_tracing_directives(None);
        assert!(directives.contains(DEFAULT_TRACING_DIRECTIVES));
        for suppress_directive in DEFAULT_SUPPRESS_DIRECTIVES {
            assert!(directives.contains(suppress_directive));
        }
    }

    #[test]
    fn explicit_rust_log_keeps_target_specific_override() {
        let directives = resolve_tracing_directives(Some(
            "info,iroh_docs::engine::live=warn,kukuri_desktop_tauri_lib=debug",
        ));
        assert!(
            directives.contains("iroh_docs::engine::live=warn"),
            "expected explicit target override to be preserved"
        );
        assert!(!directives.contains("iroh_docs::engine::live=error"));
        assert!(directives.contains("iroh_quinn_proto::connection=error"));
        assert!(directives.contains("mainline::rpc::socket=error"));
    }
}
