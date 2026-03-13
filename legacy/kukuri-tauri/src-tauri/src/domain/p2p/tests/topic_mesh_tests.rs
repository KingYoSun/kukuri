#[cfg(test)]
mod tests {
    use crate::domain::p2p::message::{GossipMessage, MessageType};
    use crate::domain::p2p::topic_mesh::*;

    fn create_test_mesh() -> TopicMesh {
        TopicMesh::new("test-topic".to_string())
    }

    fn create_test_message(id: u8) -> GossipMessage {
        let mut message = GossipMessage::new(MessageType::NostrEvent, vec![id], vec![id; 32]);
        // 一意のIDを設定
        message.id[0] = id;
        message
    }

    #[tokio::test]
    async fn test_topic_mesh_creation() {
        let mesh = create_test_mesh();
        let stats = mesh.get_stats().await;

        assert_eq!(stats.peer_count, 0);
        assert_eq!(stats.message_count, 0);
        assert_eq!(stats.last_activity, 0);
    }

    #[tokio::test]
    async fn test_message_handling() {
        let mesh = create_test_mesh();
        let message = create_test_message(1);

        // メッセージ処理
        let result = mesh.handle_message(message.clone()).await;
        assert!(result.is_ok());

        // 統計情報の確認
        let stats = mesh.get_stats().await;
        assert_eq!(stats.message_count, 1);
        assert_eq!(stats.peer_count, 1);
        assert!(stats.last_activity > 0);
    }

    #[tokio::test]
    async fn test_duplicate_detection() {
        let mesh = create_test_mesh();
        let message = create_test_message(2);

        // 最初のメッセージは重複ではない
        assert!(!mesh.is_duplicate(&message.id).await);

        // メッセージを処理
        mesh.handle_message(message.clone()).await.unwrap();

        // 同じメッセージは重複として検出される
        assert!(mesh.is_duplicate(&message.id).await);
    }

    #[tokio::test]
    async fn test_peer_management() {
        let mesh = create_test_mesh();
        let peer1 = vec![1; 32];
        let peer2 = vec![2; 32];

        // ピアの追加
        mesh.update_peer_status(peer1.clone(), true).await;
        mesh.update_peer_status(peer2.clone(), true).await;

        let stats = mesh.get_stats().await;
        assert_eq!(stats.peer_count, 2);

        // ピアの削除
        mesh.update_peer_status(peer1, false).await;
        let stats = mesh.get_stats().await;
        assert_eq!(stats.peer_count, 1);
    }

    #[tokio::test]
    async fn test_cache_limit() {
        let mesh = create_test_mesh();

        // キャッシュ制限（1000）を超えるメッセージを追加しようとする
        // 実際にはLRUキャッシュが古いメッセージを削除する
        for i in 0..1100 {
            let mut message = create_test_message((i % 256) as u8);
            // より一意なIDを設定
            message.id[0] = (i % 256) as u8;
            message.id[1] = ((i >> 8) % 256) as u8;
            mesh.handle_message(message).await.unwrap();
        }

        let stats = mesh.get_stats().await;
        // キャッシュサイズは1000以下
        assert!(stats.message_count <= 1000);
    }

    #[tokio::test]
    async fn test_concurrent_message_handling() {
        use std::sync::Arc;
        use tokio::task;

        let mesh = Arc::new(create_test_mesh());
        let mut handles = vec![];

        // 10個の並行タスクでメッセージを送信
        for i in 0..10 {
            let mesh_clone = mesh.clone();
            let handle = task::spawn(async move {
                for j in 0..100 {
                    let mut message = create_test_message((i * 100 + j) as u8);
                    // より一意なメッセージIDを設定
                    message.id[0] = ((i * 100 + j) % 256) as u8;
                    message.id[1] = ((i * 100 + j) / 256) as u8;
                    mesh_clone.handle_message(message).await.unwrap();
                }
            });
            handles.push(handle);
        }

        // すべてのタスクが完了するのを待つ
        for handle in handles {
            handle.await.unwrap();
        }

        // 統計情報を確認
        let stats = mesh.get_stats().await;
        // 1000メッセージ送信したが、重複があるため実際のメッセージ数は少ない
        assert!(stats.message_count > 0);
        assert!(stats.message_count <= 1000); // キャッシュ制限を超えない
        assert!(stats.peer_count > 0);
        assert!(stats.peer_count <= 1000);
    }

    #[tokio::test]
    async fn test_concurrent_peer_updates() {
        use std::sync::Arc;
        use tokio::task;

        let mesh = Arc::new(create_test_mesh());
        let mut handles = vec![];

        // 並行してピアの追加/削除を行う
        for i in 0..5 {
            let mesh_clone = mesh.clone();
            let handle = task::spawn(async move {
                for j in 0..20 {
                    let peer = vec![(i * 20 + j) as u8; 32];
                    mesh_clone.update_peer_status(peer.clone(), true).await;
                    if j % 2 == 0 {
                        mesh_clone.update_peer_status(peer, false).await;
                    }
                }
            });
            handles.push(handle);
        }

        // すべてのタスクが完了するのを待つ
        for handle in handles {
            handle.await.unwrap();
        }

        // 最終的なピア数を確認
        let stats = mesh.get_stats().await;
        assert_eq!(stats.peer_count, 50); // 奇数番号のピアのみ残る
    }

    #[tokio::test]
    async fn test_concurrent_cache_operations() {
        use std::sync::Arc;
        use tokio::task;

        let mesh = Arc::new(create_test_mesh());

        // メッセージ追加タスク
        let mesh_add = mesh.clone();
        let add_task = task::spawn(async move {
            for i in 0..500 {
                let message = create_test_message(i as u8);
                mesh_add.handle_message(message).await.unwrap();
                tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
            }
        });

        // 統計情報取得タスク
        let mesh_stats = mesh.clone();
        let stats_task = task::spawn(async move {
            let mut last_count = 0;
            for _ in 0..50 {
                let stats = mesh_stats.get_stats().await;
                assert!(stats.message_count >= last_count); // 単調増加
                last_count = stats.message_count;
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            }
        });

        // すべてのタスクが完了するのを待つ
        add_task.await.unwrap();
        stats_task.await.unwrap();
    }
}
