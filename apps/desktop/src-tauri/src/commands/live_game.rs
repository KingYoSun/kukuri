use kukuri_desktop_runtime::{
    AcceptDomeConnectionProposalRequest, CreateDomeConnectionProposalRequest,
    CloseDomeHostingRequest, CommitDomeLayoutRequest, CreateGameRoomRequest,
    CreateLiveSessionRequest,
    CreateMetaverseRoomRequest, DelegateDomeHostingRequest, GetDomeHostingRequest,
    ImportMetaverseRoomAssetRequest, ListDomeConnectionTopologyRequest, ListGameRoomsRequest,
    ListLiveSessionsRequest, ListMetaverseRoomEventsRequest, LiveSessionCommandRequest,
    MoveDomeRequest, PublishMetaverseRoomEventRequest, ResyncDomeSnapshotsRequest,
    RevokeDomeConnectionRequest, StartOwnerDomeHostingRequest, SubmitDomeSessionInputRequest,
    UpdateGameRoomRequest, UpdateMetaverseRoomRequest, WithdrawDomeConnectionProposalRequest,
};

use crate::state::{CommandError, DesktopState, map_error};

#[tauri::command]
pub async fn list_live_sessions(
    state: tauri::State<'_, DesktopState>,
    request: ListLiveSessionsRequest,
) -> Result<Vec<kukuri_app_api::LiveSessionView>, CommandError> {
    state
        .runtime
        .list_live_sessions(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn create_live_session(
    state: tauri::State<'_, DesktopState>,
    request: CreateLiveSessionRequest,
) -> Result<String, CommandError> {
    state
        .runtime
        .create_live_session(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn end_live_session(
    state: tauri::State<'_, DesktopState>,
    request: LiveSessionCommandRequest,
) -> Result<(), CommandError> {
    state
        .runtime
        .end_live_session(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn join_live_session(
    state: tauri::State<'_, DesktopState>,
    request: LiveSessionCommandRequest,
) -> Result<(), CommandError> {
    state
        .runtime
        .join_live_session(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn leave_live_session(
    state: tauri::State<'_, DesktopState>,
    request: LiveSessionCommandRequest,
) -> Result<(), CommandError> {
    state
        .runtime
        .leave_live_session(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn list_game_rooms(
    state: tauri::State<'_, DesktopState>,
    request: ListGameRoomsRequest,
) -> Result<Vec<kukuri_app_api::GameRoomView>, CommandError> {
    state
        .runtime
        .list_game_rooms(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn create_game_room(
    state: tauri::State<'_, DesktopState>,
    request: CreateGameRoomRequest,
) -> Result<String, CommandError> {
    state
        .runtime
        .create_game_room(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn update_game_room(
    state: tauri::State<'_, DesktopState>,
    request: UpdateGameRoomRequest,
) -> Result<(), CommandError> {
    state
        .runtime
        .update_game_room(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn create_metaverse_room(
    state: tauri::State<'_, DesktopState>,
    request: CreateMetaverseRoomRequest,
) -> Result<String, CommandError> {
    state
        .runtime
        .create_metaverse_room(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn update_metaverse_room(
    state: tauri::State<'_, DesktopState>,
    request: UpdateMetaverseRoomRequest,
) -> Result<(), CommandError> {
    state
        .runtime
        .update_metaverse_room(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn get_dome_hosting(
    state: tauri::State<'_, DesktopState>,
    request: GetDomeHostingRequest,
) -> Result<kukuri_app_api::DomeHostingView, CommandError> {
    state.runtime.get_dome_hosting(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn start_owner_dome_hosting(
    state: tauri::State<'_, DesktopState>,
    request: StartOwnerDomeHostingRequest,
) -> Result<kukuri_app_api::DomeHostingView, CommandError> {
    state
        .runtime
        .start_owner_dome_hosting(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn delegate_dome_hosting(
    state: tauri::State<'_, DesktopState>,
    request: DelegateDomeHostingRequest,
) -> Result<kukuri_app_api::DomeHostingView, CommandError> {
    state
        .runtime
        .delegate_dome_hosting(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn close_dome_hosting(
    state: tauri::State<'_, DesktopState>,
    request: CloseDomeHostingRequest,
) -> Result<kukuri_app_api::DomeHostingView, CommandError> {
    state.runtime.close_dome_hosting(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn submit_dome_session_input(
    state: tauri::State<'_, DesktopState>,
    request: SubmitDomeSessionInputRequest,
) -> Result<kukuri_core::DomePhysicsSnapshotV1, CommandError> {
    state
        .runtime
        .submit_dome_session_input(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn commit_dome_layout(
    state: tauri::State<'_, DesktopState>,
    request: CommitDomeLayoutRequest,
) -> Result<kukuri_app_api::DomeLayoutCommitView, CommandError> {
    state
        .runtime
        .commit_dome_layout(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn resync_dome_snapshots(
    state: tauri::State<'_, DesktopState>,
    request: ResyncDomeSnapshotsRequest,
) -> Result<Vec<kukuri_core::DomePhysicsSnapshotV1>, CommandError> {
    state
        .runtime
        .resync_dome_snapshots(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn move_dome(
    state: tauri::State<'_, DesktopState>,
    request: MoveDomeRequest,
) -> Result<kukuri_app_api::DomeMoveView, CommandError> {
    state.runtime.move_dome(request).await.map_err(map_error)
}

#[tauri::command]
pub async fn list_dome_connection_topology(
    state: tauri::State<'_, DesktopState>,
    request: ListDomeConnectionTopologyRequest,
) -> Result<kukuri_app_api::DomeConnectionTopologyView, CommandError> {
    state
        .runtime
        .list_dome_connection_topology(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn create_dome_connection_proposal(
    state: tauri::State<'_, DesktopState>,
    request: CreateDomeConnectionProposalRequest,
) -> Result<kukuri_app_api::DomeConnectionProposalView, CommandError> {
    state
        .runtime
        .create_dome_connection_proposal(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn accept_dome_connection_proposal(
    state: tauri::State<'_, DesktopState>,
    request: AcceptDomeConnectionProposalRequest,
) -> Result<kukuri_app_api::DomeConnectionView, CommandError> {
    state
        .runtime
        .accept_dome_connection_proposal(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn withdraw_dome_connection_proposal(
    state: tauri::State<'_, DesktopState>,
    request: WithdrawDomeConnectionProposalRequest,
) -> Result<kukuri_app_api::DomeConnectionProposalView, CommandError> {
    state
        .runtime
        .withdraw_dome_connection_proposal(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn revoke_dome_connection(
    state: tauri::State<'_, DesktopState>,
    request: RevokeDomeConnectionRequest,
) -> Result<kukuri_app_api::DomeConnectionView, CommandError> {
    state
        .runtime
        .revoke_dome_connection(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn publish_metaverse_room_event(
    state: tauri::State<'_, DesktopState>,
    request: PublishMetaverseRoomEventRequest,
) -> Result<kukuri_app_api::MetaverseRoomEventView, CommandError> {
    state
        .runtime
        .publish_metaverse_room_event(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn list_metaverse_room_events(
    state: tauri::State<'_, DesktopState>,
    request: ListMetaverseRoomEventsRequest,
) -> Result<Vec<kukuri_app_api::MetaverseRoomEventView>, CommandError> {
    state
        .runtime
        .list_metaverse_room_events(request)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn import_metaverse_room_asset(
    state: tauri::State<'_, DesktopState>,
    request: ImportMetaverseRoomAssetRequest,
) -> Result<kukuri_app_api::MetaverseAssetRefView, CommandError> {
    state
        .runtime
        .import_metaverse_room_asset(request)
        .await
        .map_err(map_error)
}
