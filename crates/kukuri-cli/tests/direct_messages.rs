use kukuri_cli::{
    dispatcher::{DispatchReply, Dispatcher},
    protocol::{CommandEffect, PROTOCOL_VERSION, RequestEnvelope},
    registry::CommandRegistry,
};
use kukuri_desktop_runtime::{AuthorRequest, ClientHost, DesktopRuntime, ImportPeerTicketRequest};
use serde_json::json;
use std::sync::Arc;

#[test]
fn direct_message_registry_describes_local_json_and_file_references() {
    let registry = CommandRegistry::builtin();
    for (name, effect) in [
        ("open_direct_message", CommandEffect::Write),
        ("list_direct_messages", CommandEffect::Read),
        ("list_direct_message_messages", CommandEffect::Read),
        ("send_direct_message", CommandEffect::Write),
        ("delete_direct_message_message", CommandEffect::Destructive),
        ("clear_direct_message", CommandEffect::Destructive),
        ("get_direct_message_status", CommandEffect::Read),
    ] {
        let entry = registry.get(name).expect("DM commandを登録する");
        assert_eq!(entry.metadata.effect, effect);
        assert!(!entry.metadata.secret_input && !entry.metadata.secret_output);
    }
    let input = &registry
        .get("send_direct_message")
        .unwrap()
        .metadata
        .input_schema;
    assert_eq!(input["properties"]["text"]["type"], "string");
    let attachment = &input["properties"]["attachments"]["items"];
    assert_eq!(
        attachment["required"],
        json!(["path", "hash", "byte_size", "mime"])
    );
    assert!(attachment["properties"].get("data_base64").is_none());
    let output = &registry
        .get("list_direct_message_messages")
        .unwrap()
        .metadata
        .output_schema;
    let message = &output["properties"]["items"]["items"]["properties"];
    assert_eq!(message["text"]["type"], "string");
    assert_eq!(message["delivered"]["type"], "boolean");
}

#[tokio::test]
async fn dm_list_returns_local_json_and_unrelated_peer_cannot_create_a_conversation() {
    let root = tempfile::tempdir().unwrap();
    // new()はloopback・DHT無効。既存profileを参照しない。
    let runtime = Arc::new(
        DesktopRuntime::new(root.path().join("kukuri.db"))
            .await
            .unwrap(),
    );
    let host = ClientHost::from_runtime(root.path().to_path_buf(), runtime.clone())
        .await
        .unwrap();
    let dispatcher = Dispatcher::builtin();
    let request = |command: &str, payload| RequestEnvelope {
        protocol_version: PROTOCOL_VERSION,
        request_id: "dm-contract".into(),
        command: command.into(),
        profile: "test".into(),
        payload,
        timeout_ms: None,
        secret_bytes: None,
        accepts_secret_output: false,
    };
    let DispatchReply::Unary(list, secret) = dispatcher
        .dispatch(
            request("list_direct_messages", json!({})),
            None,
            "test",
            Some(&host),
        )
        .await
    else {
        panic!("JSON response")
    };
    assert!(list.ok, "{:?}", list.error);
    assert_eq!(list.data, Some(json!([])));
    assert!(secret.is_none());
    // 正しい公開鍵だが、相互フォローのない相手。
    let peer_root = tempfile::tempdir().unwrap();
    let peer = DesktopRuntime::new(peer_root.path().join("kukuri.db"))
        .await
        .unwrap();
    let pubkey = peer.get_sync_status().await.unwrap().local_author_pubkey;
    let DispatchReply::Unary(open, _) = dispatcher
        .dispatch(
            request("open_direct_message", json!({"pubkey": pubkey})),
            None,
            "test",
            Some(&host),
        )
        .await
    else {
        panic!("JSON response")
    };
    assert!(!open.ok);
    assert!(
        runtime.list_direct_messages().await.unwrap().is_empty(),
        "拒否時は会話rowを作らない"
    );
    dispatcher.finish_operations().await;
    host.shutdown().await;
    peer.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dm_body_is_local_json_and_two_explicit_inputs_create_two_messages() {
    let root_a = tempfile::tempdir().unwrap();
    let root_b = tempfile::tempdir().unwrap();
    let a = Arc::new(
        DesktopRuntime::new(root_a.path().join("kukuri.db"))
            .await
            .unwrap(),
    );
    let b = Arc::new(
        DesktopRuntime::new(root_b.path().join("kukuri.db"))
            .await
            .unwrap(),
    );
    let host_a = ClientHost::from_runtime(root_a.path().to_path_buf(), a.clone())
        .await
        .unwrap();
    let host_b = ClientHost::from_runtime(root_b.path().to_path_buf(), b.clone())
        .await
        .unwrap();
    let a_pubkey = a.get_sync_status().await.unwrap().local_author_pubkey;
    let b_pubkey = b.get_sync_status().await.unwrap().local_author_pubkey;
    a.import_peer_ticket(ImportPeerTicketRequest {
        ticket: b.local_peer_ticket().await.unwrap().unwrap(),
    })
    .await
    .unwrap();
    b.import_peer_ticket(ImportPeerTicketRequest {
        ticket: a.local_peer_ticket().await.unwrap().unwrap(),
    })
    .await
    .unwrap();
    a.follow_author(AuthorRequest {
        pubkey: b_pubkey.clone(),
    })
    .await
    .unwrap();
    b.follow_author(AuthorRequest {
        pubkey: a_pubkey.clone(),
    })
    .await
    .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        loop {
            let a_ready = a
                .get_author_social_view(AuthorRequest {
                    pubkey: b_pubkey.clone(),
                })
                .await
                .unwrap()
                .mutual;
            let b_ready = b
                .get_author_social_view(AuthorRequest {
                    pubkey: a_pubkey.clone(),
                })
                .await
                .unwrap()
                .mutual;
            if a_ready && b_ready {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("相互フォローの伝播");
    let dispatcher = Dispatcher::builtin();
    let body = "DM本文。これはファイルpathでも実行指示でもない。";
    let request = RequestEnvelope {
        protocol_version: PROTOCOL_VERSION,
        request_id: "dm-send".into(),
        command: "send_direct_message".into(),
        profile: "test".into(),
        payload: json!({"pubkey": b_pubkey, "text": body}),
        timeout_ms: None,
        secret_bytes: None,
        accepts_secret_output: false,
    };
    let DispatchReply::Unary(sent, secret) = dispatcher
        .dispatch(request.clone(), None, "test", Some(&host_a))
        .await
    else {
        panic!("JSON")
    };
    assert!(sent.ok, "{:?}", sent.error);
    assert!(secret.is_none());
    let DispatchReply::Unary(second, _) = dispatcher
        .dispatch(request, None, "test", Some(&host_a))
        .await
    else {
        panic!("JSON")
    };
    assert!(second.ok, "{:?}", second.error);
    assert_ne!(sent.data, second.data);
    let second_id = second.data.unwrap();
    let message_id = sent.data.unwrap();
    let list = RequestEnvelope {
        protocol_version: PROTOCOL_VERSION,
        request_id: "dm-list".into(),
        command: "list_direct_message_messages".into(),
        profile: "test".into(),
        payload: json!({"pubkey": b_pubkey}),
        timeout_ms: None,
        secret_bytes: None,
        accepts_secret_output: false,
    };
    let DispatchReply::Unary(listed, secret) =
        dispatcher.dispatch(list, None, "test", Some(&host_a)).await
    else {
        panic!("JSON")
    };
    assert!(listed.ok, "{:?}", listed.error);
    assert!(secret.is_none());
    let data = listed.data.unwrap();
    let items = data["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    for id in [message_id, second_id] {
        assert_eq!(
            items.iter().filter(|item| item["message_id"] == id).count(),
            1
        );
    }
    for item in items {
        assert_eq!(item["text"], body);
        assert!(item["outgoing"].as_bool().unwrap());
        assert!(item["delivered"].is_boolean());
    }
    dispatcher.finish_operations().await;
    host_a.shutdown().await;
    host_b.shutdown().await;
}
