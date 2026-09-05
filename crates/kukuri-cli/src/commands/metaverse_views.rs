use super::{
    game_views,
    metaverse_schema::{event, integer, spatial_context, string, unsigned, vector, view},
    schema::{array, nullable, object},
};
use serde_json::{Value, json};

pub(super) fn hosting() -> Value {
    let host = object(
        json!({"kind": {"enum": ["owner_device", "community_node"]}, "endpoint_id": string(),
        "host_pubkey": string(), "node_id": string(), "api_base_url": string()}),
        &["kind"],
    );
    let lease = view(
        json!({"lease_id": string(), "spatial_context": spatial_context(), "instance_id": string(),
        "instance_generation": unsigned(), "owner_pubkey": string(), "host": host, "manifest_blob_hash": string(),
        "manifest_version": unsigned(), "epoch": unsigned(), "issued_at": integer(), "expires_at": integer()}),
    );
    view(
        json!({"instance_id": string(), "state": game_views::hosting_state(), "lease": nullable(lease),
        "signed_lease_json": nullable(string()), "signed_activation_json": nullable(string()), "signed_close_json": nullable(string()),
        "instance_manifest_json": string(), "preset_manifest_json": string(), "participants": unsigned(), "sleeping": {"type": "boolean"},
        "resource_budget": budget(), "resource_metrics": view(json!({"rejected_total": unsigned(),
            "rejection_counts": array(view(json!({"code": string(), "count": unsigned()}))), "participant_high_water": unsigned(),
            "rigid_body_high_water": unsigned(), "snapshot_bytes": unsigned(), "snapshot_throttled": unsigned()}))}),
    )
}

fn budget_fields(names: &[&str]) -> Value {
    view(Value::Object(
        names
            .iter()
            .map(|name| ((*name).to_string(), unsigned()))
            .collect(),
    ))
}
fn budget() -> Value {
    view(json!({
        "dome": budget_fields(&["max_persistent_props", "max_texture_bytes", "max_texture_dimension", "max_model_bytes", "max_model_triangles", "max_colliders", "max_rigid_bodies", "max_snapshot_hz"]),
        "player": budget_fields(&["max_guest_props", "max_guest_prop_bytes", "max_avatar_asset_bytes", "max_prop_spawns_per_minute", "max_interactions_per_second", "max_input_bytes_per_second", "max_proposals_per_ten_minutes_per_slot", "max_impulse_centimeters", "max_audio_frames_per_second", "max_audio_bytes_per_second"]),
        "host": budget_fields(&["max_participants", "max_simulated_rigid_bodies", "max_snapshot_bytes_per_second", "max_session_asset_bytes"]),
        "client": budget_fields(&["max_rendered_avatars", "max_texture_memory_bytes", "max_rendered_triangles", "max_interpolated_bodies", "max_neighbor_domes", "cache_capacity_bytes", "max_concurrent_audio_streams", "max_audio_jitter_frames"])
    }))
}

pub(super) fn snapshot() -> Value {
    view(
        json!({"instance_id": string(), "instance_generation": unsigned(), "lease_epoch": unsigned(), "session_id": string(),
        "host_pubkey": string(), "sequence": unsigned(), "simulated_at": integer(), "sleeping": {"type": "boolean"},
        "bodies": array(view(json!({"entity_id": string(), "kind": {"enum": ["avatar", "persistent_prop", "guest_prop"]},
            "position": vector(), "rotation": vector(), "linear_velocity": vector(), "animation": nullable(string()),
            "grabbed_by": nullable(string()), "expires_at": nullable(integer())})))}),
    )
}

pub(super) fn movement() -> Value {
    view(
        json!({"move_id": string(), "owner_pubkey": string(), "source_instance_id": string(), "source_context": spatial_context(),
        "source_generation": unsigned(), "target_instance_id": string(), "target_context": spatial_context(), "target_generation": unsigned(),
        "preset_ref": view(json!({"preset_id": string(), "owner_pubkey": string(), "revision": unsigned(), "manifest_blob_hash": string(), "manifest_mime": string(), "manifest_bytes": unsigned()})),
        "phase": {"enum": ["preparing", "target_staged", "source_detached", "target_active", "source_tombstoned", "completed", "failed"]},
        "failure_reason": nullable(string()), "updated_at": integer()}),
    )
}

pub(super) fn room_event() -> Value {
    view(
        json!({"envelope_id": string(), "received_at": integer(), "source_peer": string(),
        "content": view(json!({"event_id": string(), "topic_id": string(), "channel_id": nullable(string()), "room_id": string(),
            "spatial_context": spatial_context(), "instance_generation": unsigned(), "session_id": string(), "peer_id": string(),
            "seq": unsigned(), "sent_at": integer(), "event": event()})),
        "envelope": view(json!({"id": string(), "pubkey": string(), "created_at": integer(), "kind": string(),
            "tags": array(array(string())), "content": string(), "sig": string()}))}),
    )
}
