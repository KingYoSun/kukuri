use super::schema::{array, nullable, object};
use serde_json::{Value, json};

fn string() -> Value {
    json!({"type": "string"})
}
fn strings() -> Value {
    array(string())
}
fn boolean() -> Value {
    json!({"type": "boolean"})
}
fn integer() -> Value {
    json!({"type": "integer"})
}
fn count() -> Value {
    json!({"type": "integer", "minimum": 0})
}
fn path() -> Value {
    json!({"enum": ["direct_p2p", "relay_supported_p2p", "relay_fallback"]})
}
fn mode() -> Value {
    json!({"enum": ["static_peer", "seeded_dht"]})
}
fn connect_mode() -> Value {
    json!({"enum": ["direct_only", "direct_or_relay"]})
}
fn delivery() -> Value {
    json!({"enum": ["Live", "DurableRecovering", "DurableReady", "Offline"]})
}

fn view(properties: Value) -> Value {
    let required = properties
        .as_object()
        .expect("view properties")
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    json!({"type": "object", "properties": properties, "required": required, "additionalProperties": false})
}

pub(super) fn input(name: &str) -> Value {
    match name {
        "get_sync_status" | "get_discovery_config" | "get_local_peer_ticket" => {
            object(json!({}), &[])
        }
        "import_peer_ticket" => object(json!({"ticket": string()}), &["ticket"]),
        "set_discovery_seeds" => object(json!({"seed_entries": strings()}), &["seed_entries"]),
        "unsubscribe_topic" => object(json!({"topic": string()}), &["topic"]),
        "set_topic_gossip_enabled" => object(
            json!({"topic": string(), "enabled": boolean()}),
            &["topic", "enabled"],
        ),
        "set_channel_gossip_enabled" => object(
            json!({"topic": string(), "channel": string(), "enabled": boolean()}),
            &["topic", "channel", "enabled"],
        ),
        _ => unreachable!("network input schema"),
    }
}

pub(super) fn output(name: &str) -> Value {
    match name {
        "get_sync_status" => sync_status(),
        "get_discovery_config" | "set_discovery_seeds" => view(json!({
            "mode": mode(), "connect_mode": connect_mode(), "env_locked": boolean(),
            "seed_peers": array(view(json!({"endpoint_id": string(), "addr_hint": nullable(string())})))
        })),
        "get_local_peer_ticket" => nullable(string()),
        "import_peer_ticket"
        | "unsubscribe_topic"
        | "set_topic_gossip_enabled"
        | "set_channel_gossip_enabled" => json!({"type": "null"}),
        _ => unreachable!("network output schema"),
    }
}

fn sync_status() -> Value {
    view(json!({
        "connected": boolean(), "delivery_state": delivery(), "last_sync_ts": nullable(integer()),
        "peer_count": count(), "pending_events": count(), "status_detail": string(), "last_error": nullable(string()),
        "configured_peers": strings(), "subscribed_topics": strings(), "active_path": path(), "fallback_peer_ids": strings(),
        "local_author_pubkey": string(), "gossip_disabled_topics": strings(), "gossip_disabled_channels": strings(),
        "topic_diagnostics": array(view(json!({
            "topic": string(), "joined": boolean(), "delivery_state": delivery(), "peer_count": count(),
            "connected_peers": strings(), "docs_assist_peer_ids": strings(), "configured_peer_ids": strings(), "missing_peer_ids": strings(),
            "active_path": path(), "rendezvous_peer_ids": strings(), "fallback_peer_ids": strings(),
            "last_received_at": nullable(integer()), "last_docs_activity_at": nullable(integer()), "status_detail": string(), "last_error": nullable(string())
        }))),
        "discovery": view(json!({
            "mode": mode(), "connect_mode": connect_mode(), "active_path": path(), "fallback_peer_ids": strings(), "env_locked": boolean(),
            "configured_seed_peer_ids": strings(), "bootstrap_seed_peer_ids": strings(), "manual_ticket_peer_ids": strings(),
            "connected_peer_ids": strings(), "docs_assist_peer_ids": strings(), "blob_assist_peer_ids": strings(),
            "local_endpoint_id": string(), "last_discovery_error": nullable(string())
        }))
    }))
}
