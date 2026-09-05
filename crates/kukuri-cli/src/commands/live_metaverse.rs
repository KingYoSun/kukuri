use super::{
    command, command_error, decode, encode, host_guards, media, metaverse_schema, runtime,
};
use crate::{
    protocol::{CommandEffect, ProtocolError, SecretInput},
    registry::{CommandHandler, CommandOutput, CommandRegistration, HandlerContext},
};
use async_trait::async_trait;
use kukuri_desktop_runtime::{
    AbortDomeTransitionRequest, AcceptDomeConnectionProposalRequest, CloseDomeHostingRequest,
    CommitDomeLayoutRequest, CommitDomeTransitionRequest, CreateDomeConnectionProposalRequest,
    CreateMetaverseRoomRequest, DelegateDomeHostingRequest, GetDomeHostingRequest,
    ImportMetaverseRoomAssetRequest, ListDomeConnectionTopologyRequest,
    ListMetaverseRoomEventsRequest, MoveDomeRequest, PrepareDomeTransitionRequest,
    PublishMetaverseRoomEventRequest, ResyncDomeSnapshotsRequest, RevokeDomeConnectionRequest,
    StartOwnerDomeHostingRequest, SubmitDomeSessionInputRequest, UpdateMetaverseRoomRequest,
    WithdrawDomeConnectionProposalRequest,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

struct Handler(&'static str);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetInput {
    topic: String,
    room_id: String,
    kind: kukuri_core::MetaverseAssetKind,
    file: media::FileAttachment,
}

#[async_trait]
impl CommandHandler for Handler {
    async fn execute(
        &self,
        context: HandlerContext<'_>,
        payload: Value,
        _: Option<&SecretInput>,
    ) -> Result<CommandOutput, ProtocolError> {
        let runtime = runtime(&context)?;
        match self.0 {
            "create_metaverse_room" => encode(
                runtime
                    .create_metaverse_room(decode::<CreateMetaverseRoomRequest>(payload)?)
                    .await
                    .map_err(command_error)?,
            ),
            "update_metaverse_room" => encode(
                runtime
                    .update_metaverse_room(decode::<UpdateMetaverseRoomRequest>(payload)?)
                    .await
                    .map_err(command_error)?,
            ),
            "get_dome_hosting" => encode(
                runtime
                    .get_dome_hosting(decode::<GetDomeHostingRequest>(payload)?)
                    .await
                    .map_err(command_error)?,
            ),
            "start_owner_dome_hosting" => encode(
                runtime
                    .start_owner_dome_hosting(decode::<StartOwnerDomeHostingRequest>(payload)?)
                    .await
                    .map_err(command_error)?,
            ),
            "delegate_dome_hosting" => encode(
                runtime
                    .delegate_dome_hosting(decode::<DelegateDomeHostingRequest>(payload)?)
                    .await
                    .map_err(command_error)?,
            ),
            "close_dome_hosting" => encode(
                runtime
                    .close_dome_hosting(decode::<CloseDomeHostingRequest>(payload)?)
                    .await
                    .map_err(command_error)?,
            ),
            "submit_dome_session_input" => encode(
                runtime
                    .submit_dome_session_input(decode::<SubmitDomeSessionInputRequest>(payload)?)
                    .await
                    .map_err(command_error)?,
            ),
            "prepare_dome_transition" => encode(
                runtime
                    .prepare_dome_transition(decode::<PrepareDomeTransitionRequest>(payload)?)
                    .await
                    .map_err(command_error)?,
            ),
            "preview_dome_transition_access" => encode(
                runtime
                    .preview_dome_transition_access(decode::<PrepareDomeTransitionRequest>(
                        payload,
                    )?)
                    .await
                    .map_err(command_error)?,
            ),
            "commit_dome_transition" => encode(
                runtime
                    .commit_dome_transition(decode::<CommitDomeTransitionRequest>(payload)?)
                    .await
                    .map_err(command_error)?,
            ),
            "abort_dome_transition" => encode(
                runtime
                    .abort_dome_transition(decode::<AbortDomeTransitionRequest>(payload)?)
                    .await
                    .map_err(command_error)?,
            ),
            "commit_dome_layout" => encode(
                runtime
                    .commit_dome_layout(decode::<CommitDomeLayoutRequest>(payload)?)
                    .await
                    .map_err(command_error)?,
            ),
            "resync_dome_snapshots" => encode(
                runtime
                    .resync_dome_snapshots(decode::<ResyncDomeSnapshotsRequest>(payload)?)
                    .await
                    .map_err(command_error)?,
            ),
            "move_dome" => encode(
                runtime
                    .move_dome(decode::<MoveDomeRequest>(payload)?)
                    .await
                    .map_err(command_error)?,
            ),
            "list_dome_connection_topology" => encode(
                runtime
                    .list_dome_connection_topology(decode::<ListDomeConnectionTopologyRequest>(
                        payload,
                    )?)
                    .await
                    .map_err(command_error)?,
            ),
            "create_dome_connection_proposal" => encode(
                runtime
                    .create_dome_connection_proposal(decode::<CreateDomeConnectionProposalRequest>(
                        payload,
                    )?)
                    .await
                    .map_err(command_error)?,
            ),
            "accept_dome_connection_proposal" => encode(
                runtime
                    .accept_dome_connection_proposal(decode::<AcceptDomeConnectionProposalRequest>(
                        payload,
                    )?)
                    .await
                    .map_err(command_error)?,
            ),
            "withdraw_dome_connection_proposal" => encode(
                runtime
                    .withdraw_dome_connection_proposal(decode::<
                        WithdrawDomeConnectionProposalRequest,
                    >(payload)?)
                    .await
                    .map_err(command_error)?,
            ),
            "revoke_dome_connection" => encode(
                runtime
                    .revoke_dome_connection(decode::<RevokeDomeConnectionRequest>(payload)?)
                    .await
                    .map_err(command_error)?,
            ),
            "publish_metaverse_room_event" => encode(
                runtime
                    .publish_metaverse_room_event(decode::<PublishMetaverseRoomEventRequest>(
                        payload,
                    )?)
                    .await
                    .map_err(command_error)?,
            ),
            "list_metaverse_room_events" => encode(
                runtime
                    .list_metaverse_room_events(decode::<ListMetaverseRoomEventsRequest>(payload)?)
                    .await
                    .map_err(command_error)?,
            ),
            "import_metaverse_room_asset" => {
                let request: AssetInput = decode(payload)?;
                let file = media::load_attachments(vec![request.file])
                    .await?
                    .pop()
                    .expect("assetは1件");
                encode(
                    runtime
                        .import_metaverse_room_asset(ImportMetaverseRoomAssetRequest {
                            topic: request.topic,
                            room_id: request.room_id,
                            kind: request.kind,
                            mime_type: file.mime,
                            name: file.file_name,
                            data_base64: file.data_base64,
                        })
                        .await
                        .map_err(command_error)?,
                )
            }
            _ => unreachable!("登録済みMetaverse command"),
        }
    }
}

pub(super) fn registrations() -> Vec<CommandRegistration> {
    use CommandEffect::{Destructive, Read, Write};
    [
        ("create_metaverse_room", Write),
        ("update_metaverse_room", Write),
        ("get_dome_hosting", Read),
        ("start_owner_dome_hosting", Write),
        ("delegate_dome_hosting", Write),
        ("close_dome_hosting", Destructive),
        ("submit_dome_session_input", Write),
        ("prepare_dome_transition", Write),
        ("preview_dome_transition_access", Read),
        ("commit_dome_transition", Write),
        ("abort_dome_transition", Write),
        ("commit_dome_layout", Write),
        ("resync_dome_snapshots", Read),
        ("move_dome", Write),
        ("list_dome_connection_topology", Read),
        ("create_dome_connection_proposal", Write),
        ("accept_dome_connection_proposal", Write),
        ("withdraw_dome_connection_proposal", Destructive),
        ("revoke_dome_connection", Destructive),
        ("publish_metaverse_room_event", Write),
        ("list_metaverse_room_events", Read),
        ("import_metaverse_room_asset", Write),
    ]
    .into_iter()
    .map(|(name, effect)| {
        command(
            name,
            effect,
            false,
            false,
            host_guards(),
            (
                metaverse_schema::input(name),
                metaverse_schema::output(name),
            ),
            Arc::new(Handler(name)),
        )
    })
    .collect()
}
