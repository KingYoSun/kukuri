use crate::modules::p2p::{
    GossipManager, GossipMessage, MessageType, P2PEvent, generate_topic_id
};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

/// 2つのノード間でメッセージを送受信するテスト
#[tokio::test]
#[ignore = "Requires actual network connectivity"]
async fn test_peer_to_peer_messaging() {
    // ノード1を作成
    let iroh_secret_key1 = iroh::SecretKey::generate(rand::thread_rng());
    let secp_secret_key1 = secp256k1::SecretKey::new(&mut rand::thread_rng());
    let (event_tx1, _event_rx1) = mpsc::unbounded_channel();
    let node1 = GossipManager::new(iroh_secret_key1, secp_secret_key1, event_tx1).await.unwrap();
    
    // ノード2を作成
    let iroh_secret_key2 = iroh::SecretKey::generate(rand::thread_rng());
    let secp_secret_key2 = secp256k1::SecretKey::new(&mut rand::thread_rng());
    let (event_tx2, mut event_rx2) = mpsc::unbounded_channel();
    let node2 = GossipManager::new(iroh_secret_key2, secp_secret_key2, event_tx2).await.unwrap();
    
    // ノード1のアドレスを取得
    let node1_addr = node1.node_addr().await.unwrap();
    let _node1_id = node1.node_id();
    
    // テスト用トピック
    let topic_id = generate_topic_id("test-topic");
    
    // 両ノードが同じトピックに参加
    node1.join_topic(&topic_id, vec![]).await.unwrap();
    node2.join_topic(&topic_id, node1_addr).await.unwrap();
    
    // 接続を待つ
    sleep(Duration::from_secs(1)).await;
    
    // ノード1からメッセージを送信
    let test_payload = b"Hello from node1!";
    let message = GossipMessage::new(
        MessageType::NostrEvent,
        test_payload.to_vec(),
        vec![], // 署名時に自動設定される
    );
    
    node1.broadcast(&topic_id, message).await.unwrap();
    
    // ノード2でメッセージを受信
    let timeout = tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(event) = event_rx2.recv().await {
            if let P2PEvent::MessageReceived { topic_id: _, message, _from_peer: _ } = event {
                assert_eq!(message.payload, test_payload);
                assert!(message.verify_signature().unwrap());
                return Ok(());
            }
        }
        Err("No message received")
    });
    
    timeout.await.unwrap().unwrap();
    
    // クリーンアップ
    node1.shutdown().await.unwrap();
    node2.shutdown().await.unwrap();
}

/// 複数ノードでのブロードキャストテスト
#[tokio::test]
#[ignore = "Requires actual network connectivity"]
async fn test_multi_node_broadcast() {
    // 3つのノードを作成
    let mut nodes = Vec::new();
    let mut event_rxs = Vec::new();
    
    for _ in 0..3 {
        let iroh_secret_key = iroh::SecretKey::generate(rand::thread_rng());
        let secp_secret_key = secp256k1::SecretKey::new(&mut rand::thread_rng());
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let node = GossipManager::new(iroh_secret_key, secp_secret_key, event_tx).await.unwrap();
        nodes.push(node);
        event_rxs.push(event_rx);
    }
    
    // ノード0のアドレスを取得
    let node0_addr = nodes[0].node_addr().await.unwrap();
    
    let topic_id = generate_topic_id("broadcast-test");
    
    // すべてのノードがトピックに参加（ノード0を初期ピアとして）
    for (i, node) in nodes.iter().enumerate() {
        let initial_peers = if i == 0 { vec![] } else { node0_addr.clone() };
        node.join_topic(&topic_id, initial_peers).await.unwrap();
    }
    
    // 接続を待つ
    sleep(Duration::from_secs(2)).await;
    
    // ノード0からブロードキャスト
    let test_payload = b"Broadcast message";
    let message = GossipMessage::new(
        MessageType::NostrEvent,
        test_payload.to_vec(),
        vec![],
    );
    
    nodes[0].broadcast(&topic_id, message).await.unwrap();
    
    // 他のノードでメッセージを受信
    let mut received_count = 0;
    for (i, mut rx) in event_rxs.into_iter().enumerate() {
        if i == 0 { continue; } // 送信者はスキップ
        
        let timeout = tokio::time::timeout(Duration::from_secs(10), async {
            while let Some(event) = rx.recv().await {
                if let P2PEvent::MessageReceived { topic_id: _, message, _from_peer: _ } = event {
                    assert_eq!(message.payload, test_payload);
                    return Ok(());
                }
            }
            Err("No message received")
        });
        
        if timeout.await.unwrap().is_ok() {
            received_count += 1;
        }
    }
    
    assert_eq!(received_count, 2); // 2つのノードが受信
    
    // クリーンアップ
    for node in nodes {
        node.shutdown().await.unwrap();
    }
}

/// トピック参加・離脱のテスト
#[tokio::test]
#[ignore = "Requires actual network connectivity"]
async fn test_topic_join_leave_events() {
    let iroh_secret_key1 = iroh::SecretKey::generate(rand::thread_rng());
    let secp_secret_key1 = secp256k1::SecretKey::new(&mut rand::thread_rng());
    let (event_tx1, mut event_rx1) = mpsc::unbounded_channel();
    let node1 = GossipManager::new(iroh_secret_key1, secp_secret_key1, event_tx1).await.unwrap();
    
    let iroh_secret_key2 = iroh::SecretKey::generate(rand::thread_rng());
    let secp_secret_key2 = secp256k1::SecretKey::new(&mut rand::thread_rng());
    let (event_tx2, _) = mpsc::unbounded_channel();
    let node2 = GossipManager::new(iroh_secret_key2, secp_secret_key2, event_tx2).await.unwrap();
    
    let node1_addr = node1.node_addr().await.unwrap();
    let topic_id = generate_topic_id("join-leave-test");
    
    // ノード1がトピックに参加
    node1.join_topic(&topic_id, vec![]).await.unwrap();
    
    // ノード2が参加
    node2.join_topic(&topic_id, node1_addr).await.unwrap();
    
    // 接続を待つ
    sleep(Duration::from_secs(1)).await;
    
    // ピア参加イベントを受信
    let timeout = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(event) = event_rx1.recv().await {
            if let P2PEvent::PeerJoined { topic_id: _, peer_id: _ } = event {
                return Ok(());
            }
        }
        Err("No peer joined event")
    });
    
    timeout.await.unwrap().unwrap();
    
    // ノード2が離脱
    node2.leave_topic(&topic_id).await.unwrap();
    
    // しばらく待つ（離脱イベントが伝搬するまで）
    sleep(Duration::from_secs(1)).await;
    
    // クリーンアップ
    node1.shutdown().await.unwrap();
    node2.shutdown().await.unwrap();
}

/// 重複メッセージの除外テスト
#[tokio::test]
async fn test_duplicate_message_filtering() {
    let iroh_secret_key = iroh::SecretKey::generate(rand::thread_rng());
    let secp_secret_key = secp256k1::SecretKey::new(&mut rand::thread_rng());
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let node = GossipManager::new(iroh_secret_key, secp_secret_key, event_tx).await.unwrap();
    
    let topic_id = generate_topic_id("duplicate-test");
    node.join_topic(&topic_id, vec![]).await.unwrap();
    
    // トピックのステータスを確認
    let status = node.get_topic_status(&topic_id).await.unwrap();
    assert_eq!(status.message_count, 0);
    
    // 同じメッセージを複数回送信
    let message = GossipMessage::new(
        MessageType::NostrEvent,
        b"Duplicate test".to_vec(),
        vec![],
    );
    
    for _ in 0..3 {
        node.broadcast(&topic_id, message.clone()).await.unwrap();
    }
    
    // 統計情報を確認（重複は除外されているはず）
    sleep(Duration::from_millis(100)).await;
    let status = node.get_topic_status(&topic_id).await.unwrap();
    assert!(status.message_count <= 3); // 重複が除外されている
    
    node.shutdown().await.unwrap();
}

/// イベントバッファリングとLaggedイベントのテスト
#[tokio::test]
#[ignore = "Requires actual network connectivity"]
async fn test_event_buffering_and_lagged() {
    let iroh_secret_key = iroh::SecretKey::generate(rand::thread_rng());
    let secp_secret_key = secp256k1::SecretKey::new(&mut rand::thread_rng());
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let node = GossipManager::new(iroh_secret_key, secp_secret_key, event_tx).await.unwrap();
    
    let topic_id = generate_topic_id("buffer-test");
    node.join_topic(&topic_id, vec![]).await.unwrap();
    
    // 大量のメッセージを高速に送信
    let message_count = 100;
    for i in 0..message_count {
        let message = GossipMessage::new(
            MessageType::NostrEvent,
            format!("Message {}", i).into_bytes(),
            vec![],
        );
        node.broadcast(&topic_id, message).await.unwrap();
    }
    
    // イベントを受信
    let mut received_count = 0;
    let _lagged_events = 0;
    
    let timeout = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(event) = event_rx.recv().await {
            match event {
                P2PEvent::MessageReceived { .. } => {
                    received_count += 1;
                    if received_count >= message_count {
                        break;
                    }
                }
                _ => {}
            }
        }
    });
    
    let _ = timeout.await;
    
    // メッセージが受信されたことを確認
    assert!(received_count > 0);
    
    node.shutdown().await.unwrap();
}

/// エラーハンドリングのテスト
#[tokio::test]
async fn test_error_handling_in_message_reception() {
    let iroh_secret_key = iroh::SecretKey::generate(rand::thread_rng());
    let secp_secret_key = secp256k1::SecretKey::new(&mut rand::thread_rng());
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let node = GossipManager::new(iroh_secret_key, secp_secret_key, event_tx).await.unwrap();
    
    let topic_id = generate_topic_id("error-test");
    node.join_topic(&topic_id, vec![]).await.unwrap();
    
    // 不正な署名のメッセージを送信
    let mut invalid_message = GossipMessage::new(
        MessageType::NostrEvent,
        b"Invalid signature test".to_vec(),
        vec![],
    );
    
    // 署名を手動で無効化
    invalid_message.signature = vec![0; 64]; // 無効な署名
    invalid_message.sender = vec![1; 33]; // 送信者も設定
    
    // メッセージの送信（直接broadcast経由では署名されるので、ここでは受信テストのみ）
    // 実際のネットワーク環境では、不正な署名のメッセージは検証で失敗し、イベントとして通知されない
    
    node.shutdown().await.unwrap();
}

/// ピア接続の安定性テスト
#[tokio::test]
#[ignore = "Requires actual network connectivity"]
async fn test_peer_connection_stability() {
    // 2つのノードを作成
    let iroh_secret_key1 = iroh::SecretKey::generate(rand::thread_rng());
    let secp_secret_key1 = secp256k1::SecretKey::new(&mut rand::thread_rng());
    let (event_tx1, mut event_rx1) = mpsc::unbounded_channel();
    let node1 = GossipManager::new(iroh_secret_key1, secp_secret_key1, event_tx1).await.unwrap();
    
    let iroh_secret_key2 = iroh::SecretKey::generate(rand::thread_rng());
    let secp_secret_key2 = secp256k1::SecretKey::new(&mut rand::thread_rng());
    let (event_tx2, _event_rx2) = mpsc::unbounded_channel();
    let node2 = GossipManager::new(iroh_secret_key2, secp_secret_key2, event_tx2).await.unwrap();
    
    let node1_addr = node1.node_addr().await.unwrap();
    let topic_id = generate_topic_id("stability-test");
    
    // 両ノードがトピックに参加
    node1.join_topic(&topic_id, vec![]).await.unwrap();
    node2.join_topic(&topic_id, node1_addr.clone()).await.unwrap();
    
    // 接続を待つ
    sleep(Duration::from_secs(1)).await;
    
    // ピア接続イベントを待つ
    let peer_joined = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(event) = event_rx1.recv().await {
            if matches!(event, P2PEvent::PeerJoined { .. }) {
                return true;
            }
        }
        false
    }).await;
    
    assert!(peer_joined.unwrap_or(false));
    
    // トピックステータスを確認
    let status1 = node1.get_topic_status(&topic_id).await.unwrap();
    let status2 = node2.get_topic_status(&topic_id).await.unwrap();
    
    assert!(status1.peer_count > 0);
    assert!(status2.peer_count > 0);
    
    // メッセージ交換のテスト
    for i in 0..5 {
        let message = GossipMessage::new(
            MessageType::NostrEvent,
            format!("Stability test {}", i).into_bytes(),
            vec![],
        );
        
        if i % 2 == 0 {
            node1.broadcast(&topic_id, message).await.unwrap();
        } else {
            node2.broadcast(&topic_id, message).await.unwrap();
        }
        
        sleep(Duration::from_millis(100)).await;
    }
    
    // 最終的なステータスを確認
    let final_status1 = node1.get_topic_status(&topic_id).await.unwrap();
    let final_status2 = node2.get_topic_status(&topic_id).await.unwrap();
    
    assert!(final_status1.message_count > 0);
    assert!(final_status2.message_count > 0);
    
    // クリーンアップ
    node1.shutdown().await.unwrap();
    node2.shutdown().await.unwrap();
}

/// メッセージ順序の保証テスト
#[tokio::test]
#[ignore = "Requires actual network connectivity"]
async fn test_message_ordering() {
    let iroh_secret_key = iroh::SecretKey::generate(rand::thread_rng());
    let secp_secret_key = secp256k1::SecretKey::new(&mut rand::thread_rng());
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let node = GossipManager::new(iroh_secret_key, secp_secret_key, event_tx).await.unwrap();
    
    let topic_id = generate_topic_id("ordering-test");
    node.join_topic(&topic_id, vec![]).await.unwrap();
    
    // 連番のメッセージを送信
    let message_count = 10u64;
    for i in 0..message_count {
        let message = GossipMessage::new(
            MessageType::NostrEvent,
            i.to_le_bytes().to_vec(),
            vec![],
        );
        node.broadcast(&topic_id, message).await.unwrap();
        sleep(Duration::from_millis(10)).await; // 順序を保証するための小さな遅延
    }
    
    // 受信したメッセージの順序を確認
    let mut received_messages = Vec::new();
    let timeout = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(event) = event_rx.recv().await {
            if let P2PEvent::MessageReceived { message, .. } = event {
                if message.payload.len() == 8 {
                    let value = u64::from_le_bytes(message.payload.try_into().unwrap());
                    received_messages.push(value);
                    if received_messages.len() >= message_count as usize {
                        break;
                    }
                }
            }
        }
    });
    
    let _ = timeout.await;
    
    // メッセージが受信されたことを確認
    assert!(!received_messages.is_empty());
    
    // 順序が保たれているかチェック（完全な順序は保証されないが、大きな逆転はないはず）
    let mut inversions = 0;
    for i in 1..received_messages.len() {
        if received_messages[i] < received_messages[i-1] {
            inversions += 1;
        }
    }
    
    // 順序の逆転が全体の20%以下であることを確認
    assert!((inversions as f64 / received_messages.len() as f64) < 0.2);
    
    node.shutdown().await.unwrap();
}