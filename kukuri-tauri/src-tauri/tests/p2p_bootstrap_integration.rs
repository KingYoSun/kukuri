use chrono::Utc;
use kukuri_lib::test_support::application::ports::group_key_store::GroupKeyStore;
use kukuri_lib::test_support::application::shared::tests::p2p::fixtures::nostr_to_domain;
use kukuri_lib::test_support::infrastructure::crypto::DefaultKeyManager;
use kukuri_lib::test_support::infrastructure::storage::{
    SecureGroupKeyStore, secure_storage::DefaultSecureStorage,
};
use kukuri_lib::test_support::presentation::dto::community_node_dto::{
    CommunityNodeBootstrapServicesRequest, CommunityNodeConfigNodeRequest,
    CommunityNodeConfigRequest, CommunityNodeRoleConfig,
};
use kukuri_lib::test_support::presentation::handlers::CommunityNodeHandler;
use kukuri_lib::test_support::state::handle_bootstrap_gossip_event;
use nostr_sdk::prelude::{Event as NostrEvent, EventBuilder, Keys, Kind, Tag};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration as StdDuration;
use tiny_http::{Header, Response, Server};

#[derive(Debug)]
struct MockHttpResponse {
    status: u16,
    body: serde_json::Value,
}

impl MockHttpResponse {
    fn json(status: u16, body: serde_json::Value) -> Self {
        Self { status, body }
    }
}

fn spawn_json_sequence_server(
    responses: Vec<MockHttpResponse>,
) -> (String, Receiver<String>, thread::JoinHandle<()>) {
    let server = Server::http("127.0.0.1:0").expect("mock server");
    let base_url = format!("http://{}", server.server_addr());
    let (tx, rx) = mpsc::channel();

    let handle = thread::spawn(move || {
        for response_spec in responses {
            let request = match server.recv_timeout(StdDuration::from_secs(2)) {
                Ok(Some(request)) => request,
                Ok(None) => break,
                Err(_) => break,
            };
            let path = request
                .url()
                .split('?')
                .next()
                .unwrap_or_default()
                .to_string();
            let _ = tx.send(path);

            let mut response = Response::from_string(response_spec.body.to_string());
            response.add_header(
                Header::from_bytes("Content-Type", "application/json")
                    .expect("content-type header"),
            );
            response = response.with_status_code(response_spec.status);
            let _ = request.respond(response);
        }
    });

    (base_url, rx, handle)
}

fn join_with_timeout(handle: thread::JoinHandle<()>, timeout: StdDuration) {
    let start = std::time::Instant::now();
    while !handle.is_finished() {
        assert!(
            start.elapsed() < timeout,
            "mock server join timed out after {:?}",
            timeout
        );
        thread::sleep(StdDuration::from_millis(10));
    }
    handle.join().expect("mock server thread panicked");
}

fn build_node_descriptor_event(marker: &str) -> NostrEvent {
    let keys = Keys::generate();
    let exp = Utc::now().timestamp() + 600;
    let d_tag = format!("descriptor:{marker}");
    let exp_str = exp.to_string();
    let tags = vec![
        Tag::parse(["d", d_tag.as_str()]).expect("d"),
        Tag::parse(["k", "kukuri"]).expect("k"),
        Tag::parse(["ver", "1"]).expect("ver"),
        Tag::parse(["exp", exp_str.as_str()]).expect("exp"),
        Tag::parse(["role", "bootstrap"]).expect("role"),
    ];
    let content = json!({
        "schema": "kukuri-node-desc-v1",
        "name": format!("state-layer-node-{marker}"),
        "roles": ["bootstrap"],
        "endpoints": { "http": format!("https://{marker}.example") }
    })
    .to_string();

    EventBuilder::new(Kind::Custom(39000), content)
        .tags(tags)
        .sign_with_keys(&keys)
        .expect("sign descriptor")
}

fn build_topic_service_event(topic_id: &str, marker: &str) -> NostrEvent {
    let keys = Keys::generate();
    let exp = Utc::now().timestamp() + 600;
    let d_tag = format!("topic_service:{topic_id}:bootstrap:public:{marker}");
    let exp_str = exp.to_string();
    let tags = vec![
        Tag::parse(["d", d_tag.as_str()]).expect("d"),
        Tag::parse(["t", topic_id]).expect("t"),
        Tag::parse(["role", "bootstrap"]).expect("role"),
        Tag::parse(["scope", "public"]).expect("scope"),
        Tag::parse(["k", "kukuri"]).expect("k"),
        Tag::parse(["ver", "1"]).expect("ver"),
        Tag::parse(["exp", exp_str.as_str()]).expect("exp"),
    ];
    let content = json!({
        "schema": "kukuri-topic-service-v1",
        "topic": topic_id,
        "role": "bootstrap",
        "scope": "public",
        "marker": marker,
    })
    .to_string();

    EventBuilder::new(Kind::Custom(39001), content)
        .tags(tags)
        .sign_with_keys(&keys)
        .expect("sign topic service")
}

fn contains_event_id(items: &[Value], event_id: &str) -> bool {
    items
        .iter()
        .any(|item| item.get("id").and_then(Value::as_str) == Some(event_id))
}

fn test_handler() -> Arc<CommunityNodeHandler> {
    let key_manager = Arc::new(DefaultKeyManager::new());
    let secure_storage = Arc::new(DefaultSecureStorage::new());
    let group_key_store =
        Arc::new(SecureGroupKeyStore::new(secure_storage.clone())) as Arc<dyn GroupKeyStore>;

    Arc::new(CommunityNodeHandler::new(
        key_manager,
        secure_storage,
        group_key_store,
    ))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn state_layer_p2p_bootstrap_receive_links_refresh_and_ingest() {
    let handler = test_handler();

    let topic_id = format!(
        "kukuri:state-layer-bootstrap-{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let service_path = format!("/v1/bootstrap/topics/{topic_id}/services");
    let now = Utc::now().timestamp();

    let config_sync_node_event = build_node_descriptor_event("config-sync");
    let refresh_node_event = build_node_descriptor_event("refresh-node");
    let refresh_topic_event = build_topic_service_event(&topic_id, "refresh-topic");
    let refresh_node_event_after_ingest = build_node_descriptor_event("refresh-after-ingest");

    let (base_url, request_rx, handle) = spawn_json_sequence_server(vec![
        MockHttpResponse::json(
            200,
            json!({
                "items": [serde_json::to_value(&config_sync_node_event).expect("config sync node json")],
                "next_refresh_at": now + 3600,
            }),
        ),
        MockHttpResponse::json(
            200,
            json!({
                "items": [serde_json::to_value(&refresh_node_event).expect("refresh node json")],
                "next_refresh_at": now + 3600,
            }),
        ),
        MockHttpResponse::json(
            200,
            json!({
                "items": [serde_json::to_value(&refresh_topic_event).expect("refresh topic json")],
                "next_refresh_at": now + 3600,
            }),
        ),
        MockHttpResponse::json(
            200,
            json!({
                "items": [serde_json::to_value(&refresh_node_event_after_ingest).expect("refresh node after ingest json")],
                "next_refresh_at": now + 3600,
            }),
        ),
    ]);

    handler.clear_config().await.expect("clear config");
    handler
        .set_config(CommunityNodeConfigRequest {
            nodes: vec![CommunityNodeConfigNodeRequest {
                base_url,
                roles: Some(CommunityNodeRoleConfig {
                    labels: false,
                    trust: false,
                    search: false,
                    bootstrap: true,
                }),
            }],
        })
        .await
        .expect("set config");

    let gossip_topic_event = build_topic_service_event(&topic_id, "gossip-ingest-topic");
    let gossip_topic_domain = nostr_to_domain(&gossip_topic_event);
    handle_bootstrap_gossip_event(Arc::clone(&handler), gossip_topic_domain.clone()).await;

    let request_1 = request_rx
        .recv_timeout(StdDuration::from_secs(5))
        .expect("request 1");
    let request_2 = request_rx
        .recv_timeout(StdDuration::from_secs(5))
        .expect("request 2");
    let mut observed_refresh_paths = vec![request_1, request_2];
    if let Ok(request_3) = request_rx.recv_timeout(StdDuration::from_secs(1)) {
        observed_refresh_paths.push(request_3);
    }

    let node_path = "/v1/bootstrap/nodes";
    let node_hits = observed_refresh_paths
        .iter()
        .filter(|path| path.as_str() == node_path)
        .count();
    let service_hits = observed_refresh_paths
        .iter()
        .filter(|path| *path == &service_path)
        .count();

    assert_eq!(service_hits, 1, "service refresh should occur exactly once");
    assert!(
        (1..=2).contains(&node_hits),
        "node refresh count should be 1 or 2 (observed: {node_hits}, paths: {observed_refresh_paths:?})"
    );
    assert_eq!(
        observed_refresh_paths.len(),
        node_hits + service_hits,
        "unexpected refresh path observed: {observed_refresh_paths:?}"
    );

    let gossip_node_event = build_node_descriptor_event("gossip-ingest-node");
    let gossip_node_domain = nostr_to_domain(&gossip_node_event);
    handle_bootstrap_gossip_event(Arc::clone(&handler), gossip_node_domain.clone()).await;

    let request_4 = request_rx
        .recv_timeout(StdDuration::from_secs(5))
        .expect("request 4");
    assert_eq!(request_4, "/v1/bootstrap/nodes");

    handler
        .clear_config()
        .await
        .expect("clear config after refresh checks");

    let nodes = handler
        .list_bootstrap_nodes()
        .await
        .expect("list bootstrap nodes after ingest");
    let node_items = nodes
        .get("items")
        .and_then(Value::as_array)
        .expect("node items");
    assert!(contains_event_id(
        node_items,
        gossip_node_domain.id.as_str()
    ));

    let services = handler
        .list_bootstrap_services(CommunityNodeBootstrapServicesRequest {
            base_url: None,
            topic_id,
        })
        .await
        .expect("list bootstrap services after ingest");
    let service_items = services
        .get("items")
        .and_then(Value::as_array)
        .expect("service items");
    assert!(contains_event_id(
        service_items,
        gossip_topic_domain.id.as_str()
    ));

    join_with_timeout(handle, StdDuration::from_secs(3));
}
