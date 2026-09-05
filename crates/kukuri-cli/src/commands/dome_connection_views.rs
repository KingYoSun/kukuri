use super::{
    metaverse_schema::{direction, integer, spatial_context, string, unsigned, view},
    schema::{array, nullable},
};
use serde_json::{Value, json};

fn endpoint() -> Value {
    view(
        json!({"instance_id": string(), "instance_generation": unsigned(), "owner_pubkey": string(), "direction": direction()}),
    )
}
fn terminal_reason() -> Value {
    json!({"enum": [null, "owner_revoked", "proposer_withdrew", "proposer_slot_occupied", "instance_detached", "instance_deleted", "owners_blocked"]})
}
pub(super) fn proposal() -> Value {
    view(
        json!({"proposal": view(json!({"proposal_id": string(), "spatial_context": spatial_context(), "proposer": endpoint(),
        "receiver": endpoint(), "sequence": unsigned(), "created_at": integer()})),
        "selection": nullable(view(json!({"selection_id": string(), "proposal_id": string(), "spatial_context": spatial_context(),
            "receiver": endpoint(), "slot_generation": unsigned(), "observed_active_connection_ids": array(string()), "selected_at": integer()}))),
        "status": {"enum": ["proposed", "reserved", "accepted", "waiting_for_slot", "discarded"]}, "terminal_reason": terminal_reason(), "connection_id": string()}),
    )
}
pub(super) fn connection() -> Value {
    view(json!({"record": view(json!({
        "agreement": view(json!({"connection_id": string(), "proposal_id": string(), "spatial_context": spatial_context(), "proposer": endpoint(), "receiver": endpoint(), "activation_generation": unsigned()})),
        "receiver_slot_generation": unsigned(), "observed_active_connection_ids": array(string()),
        "status": {"enum": ["accepted", "active", "draining", "revoked"]}, "lifecycle_generation": unsigned(),
        "lifecycle_actor": nullable(string()), "lifecycle_reason": terminal_reason(), "lifecycle_deadline_at": nullable(integer())
    }))}))
}
pub(super) fn topology() -> Value {
    view(
        json!({"proposals": array(proposal()), "connections": array(connection()), "resolution": view(json!({
            "topology": view(json!({"spatial_context": spatial_context(), "active_connection_ids": array(string()), "topology_digest": string(),
                "components": array(view(json!({"root_instance_id": string(), "instance_ids": array(string()), "connection_ids": array(string()),
                    "coordinates_cm": {"type": "object", "additionalProperties": true, "description": "instance IDから3要素の整数座標へのmap。共有DomeComponentTopologyV1が生成する。"}})))})),
            "rejected_connections": array(view(json!({"connection_id": string(), "reason": string()})))
        }))}),
    )
}
