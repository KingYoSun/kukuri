#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::super::{OfflineManager, models, reindex::OfflineReindexJob};
    use crate::infrastructure::p2p::{
        ConnectionEvent, DiscoveryOptions, iroh_network_service::IrohNetworkService,
        network_service::NetworkService,
    };
    use crate::shared::config::AppConfig;
    use iroh::SecretKey;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::sync::Arc;
    use tokio::time::{Duration, sleep, timeout};

    async fn setup_test_db() -> sqlx::Pool<sqlx::Sqlite> {
        // メモリ内SQLiteデータベースを使用（Docker環境での権限問題を回避）
        let db_url = "sqlite::memory:";

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(db_url)
            .await
            .unwrap();

        // マイグレーションを実行
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        pool
    }

    #[tokio::test]
    async fn test_save_offline_action() {
        let pool = setup_test_db().await;
        let manager = OfflineManager::new(pool);

        let request = models::SaveOfflineActionRequest {
            user_pubkey: "test_user".to_string(),
            action_type: "create_post".to_string(),
            target_id: Some("post_123".to_string()),
            action_data: serde_json::json!({
                "content": "Test post content",
                "topic": "test_topic"
            }),
        };

        let response = manager.save_offline_action(request).await.unwrap();

        assert!(!response.local_id.is_empty());
        assert_eq!(response.action.user_pubkey, "test_user");
        assert_eq!(response.action.action_type, "create_post");
        assert_eq!(response.action.target_id, Some("post_123".to_string()));
        assert!(!response.action.is_synced);
    }

    #[tokio::test]
    async fn test_get_offline_actions() {
        let pool = setup_test_db().await;
        let manager = OfflineManager::new(pool);

        // 複数のアクションを保存
        for i in 0..3 {
            let request = models::SaveOfflineActionRequest {
                user_pubkey: "test_user".to_string(),
                action_type: format!("action_{i}"),
                target_id: None,
                action_data: serde_json::json!({"index": i}),
            };
            manager.save_offline_action(request).await.unwrap();
        }

        // アクションを取得（簡略化されたテスト）
        let request = models::GetOfflineActionsRequest {
            user_pubkey: Some("test_user".to_string()),
            is_synced: Some(false),
            limit: None,
        };
        let actions = manager.get_offline_actions(request).await.unwrap();

        // 少なくとも1つのアクションが返されることを確認
        assert!(!actions.is_empty());
    }

    #[tokio::test]
    async fn test_sync_queue_operations() {
        let pool = setup_test_db().await;
        let manager = OfflineManager::new(pool);

        let request = models::AddToSyncQueueRequest {
            action_type: "test_action".to_string(),
            payload: serde_json::json!({
                "test": "data",
                "value": 123
            }),
        };

        let id = manager.add_to_sync_queue(request).await.unwrap();
        assert!(id > 0);
    }

    #[tokio::test]
    async fn test_cache_metadata_operations() {
        let pool = setup_test_db().await;
        let manager = OfflineManager::new(pool);

        let request = models::UpdateCacheMetadataRequest {
            cache_key: "test_cache".to_string(),
            cache_type: "posts".to_string(),
            metadata: Some(serde_json::json!({
                "last_post_id": "post_999"
            })),
            expiry_seconds: Some(3600),
        };

        manager
            .update_cache_metadata(request.clone())
            .await
            .unwrap();

        // キャッシュステータスを確認
        let status = manager.get_cache_status().await.unwrap();
        assert_eq!(status.total_items, 1);
        assert_eq!(status.stale_items, 0);
        assert!(status.cache_types.iter().any(|ct| ct.cache_type == "posts"));
    }

    #[tokio::test]
    async fn test_optimistic_update_lifecycle() {
        let pool = setup_test_db().await;
        let manager = OfflineManager::new(pool);

        let original_data = serde_json::json!({"likes": 10}).to_string();
        let updated_data = serde_json::json!({"likes": 11}).to_string();

        // 楽観的更新を保存
        let update_id = manager
            .save_optimistic_update(
                "post".to_string(),
                "post_123".to_string(),
                Some(original_data.clone()),
                updated_data.clone(),
            )
            .await
            .unwrap();

        assert!(!update_id.is_empty());

        // 更新を確認
        manager
            .confirm_optimistic_update(update_id.clone())
            .await
            .unwrap();

        // ロールバックテスト（別の更新で）
        let update_id2 = manager
            .save_optimistic_update(
                "post".to_string(),
                "post_456".to_string(),
                Some(original_data.clone()),
                updated_data,
            )
            .await
            .unwrap();

        let rolled_back = manager
            .rollback_optimistic_update(update_id2)
            .await
            .unwrap();

        assert_eq!(rolled_back, Some(original_data));
    }

    #[tokio::test]
    async fn test_sync_status_update() {
        let pool = setup_test_db().await;
        let manager = OfflineManager::new(pool);

        // 同期ステータスを更新
        manager
            .update_sync_status(
                "post".to_string(),
                "post_789".to_string(),
                "pending".to_string(),
                None,
            )
            .await
            .unwrap();

        // 競合データと共に更新
        let conflict_data = serde_json::json!({
            "local": {"content": "Local version"},
            "remote": {"content": "Remote version"}
        })
        .to_string();

        manager
            .update_sync_status(
                "post".to_string(),
                "post_789".to_string(),
                "conflict".to_string(),
                Some(conflict_data),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_cleanup_expired_cache() {
        let pool = setup_test_db().await;
        let manager = OfflineManager::new(pool.clone());

        // 期限切れのキャッシュを作成
        let expired_time = chrono::Utc::now().timestamp() - 3600; // 1時間前
        sqlx::query(
            r#"
            INSERT INTO cache_metadata (
                cache_key, cache_type, expiry_time, data_version, is_stale
            ) VALUES (?1, ?2, ?3, 1, 0)
            "#,
        )
        .bind("expired_cache")
        .bind("test")
        .bind(expired_time)
        .execute(&pool)
        .await
        .unwrap();

        // 有効なキャッシュも作成
        let valid_time = chrono::Utc::now().timestamp() + 3600; // 1時間後
        sqlx::query(
            r#"
            INSERT INTO cache_metadata (
                cache_key, cache_type, expiry_time, data_version, is_stale
            ) VALUES (?1, ?2, ?3, 1, 0)
            "#,
        )
        .bind("valid_cache")
        .bind("test")
        .bind(valid_time)
        .execute(&pool)
        .await
        .unwrap();

        // クリーンアップを実行
        let cleaned = manager.cleanup_expired_cache().await.unwrap();
        assert_eq!(cleaned, 1);

        // キャッシュステータスを確認
        let status = manager.get_cache_status().await.unwrap();
        assert_eq!(status.total_items, 1);
    }

    #[tokio::test]
    async fn test_sync_offline_actions() {
        let pool = setup_test_db().await;
        let manager = OfflineManager::new(pool);

        // オフラインアクションを作成
        for i in 0..5 {
            let request = models::SaveOfflineActionRequest {
                user_pubkey: "sync_test_user".to_string(),
                action_type: format!("action_{i}"),
                target_id: None,
                action_data: serde_json::json!({"index": i}),
            };
            manager.save_offline_action(request).await.unwrap();
        }

        // 同期を実行
        let sync_request = models::SyncOfflineActionsRequest {
            user_pubkey: "sync_test_user".to_string(),
        };
        let result = manager.sync_offline_actions(sync_request).await.unwrap();

        // 全てのアクションが同期キューに追加されたことを確認
        assert_eq!(result.synced_count, 5);
        assert_eq!(result.failed_count, 0);
        assert_eq!(result.pending_count, 0);
    }

    #[tokio::test]
    async fn test_ensure_offline_action_in_queue_deduplicates() {
        let pool = setup_test_db().await;
        let manager = OfflineManager::new(pool);

        let request = models::SaveOfflineActionRequest {
            user_pubkey: "queue_user".to_string(),
            action_type: "create_post".to_string(),
            target_id: Some("post_001".to_string()),
            action_data: serde_json::json!({"content": "queued post"}),
        };

        let saved = manager.save_offline_action(request).await.unwrap();
        let action = saved.action.clone();

        let inserted = manager
            .ensure_offline_action_in_queue(&action)
            .await
            .unwrap();
        assert!(inserted);

        let inserted_again = manager
            .ensure_offline_action_in_queue(&action)
            .await
            .unwrap();
        assert!(!inserted_again);

        let pending = manager.get_pending_sync_queue().await.unwrap();
        assert_eq!(pending.len(), 1);
    }

    #[tokio::test]
    async fn test_get_stale_cache_entries() {
        let pool = setup_test_db().await;
        let manager = OfflineManager::new(pool.clone());

        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            r#"
            INSERT INTO cache_metadata (
                cache_key, cache_type, last_synced_at, is_stale, expiry_time, data_version
            ) VALUES (?1, ?2, ?3, 1, ?4, 1)
            "#,
        )
        .bind("stale_cache")
        .bind("posts")
        .bind(now - 10)
        .bind(now - 5)
        .execute(&pool)
        .await
        .unwrap();

        let stale = manager.get_stale_cache_entries().await.unwrap();
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].cache_key, "stale_cache");
    }

    #[tokio::test]
    async fn test_offline_reindex_job_requeues_actions() {
        let pool = setup_test_db().await;
        let manager = Arc::new(OfflineManager::new(pool));

        let request = models::SaveOfflineActionRequest {
            user_pubkey: "reindex_user".to_string(),
            action_type: "follow".to_string(),
            target_id: Some("user_123".to_string()),
            action_data: serde_json::json!({"follow": "user_123"}),
        };

        manager
            .save_offline_action(request)
            .await
            .expect("failed to save offline action");

        let job = OfflineReindexJob::create(None, Arc::clone(&manager));
        let report = job.reindex_once().await.unwrap();

        assert_eq!(report.offline_action_count, 1);
        assert_eq!(report.queued_action_count, 1);

        let pending = manager.get_pending_sync_queue().await.unwrap();
        assert_eq!(pending.len(), 1);

        let report_second = job.reindex_once().await.unwrap();
        assert_eq!(report_second.queued_action_count, 0);
    }

    #[tokio::test]
    async fn test_reindex_triggered_on_connection_event() {
        let pool = setup_test_db().await;
        let manager = Arc::new(OfflineManager::new(pool));

        let request = models::SaveOfflineActionRequest {
            user_pubkey: "connection_user".to_string(),
            action_type: "follow".to_string(),
            target_id: Some("user_999".to_string()),
            action_data: serde_json::json!({"target": "user_999"}),
        };
        manager
            .save_offline_action(request)
            .await
            .expect("failed to save offline action");

        let job = OfflineReindexJob::create(None, Arc::clone(&manager));

        let network_cfg = AppConfig::default().network;
        let secret = SecretKey::from_bytes(&[7u8; 32]);
        let service = IrohNetworkService::new(secret, network_cfg, DiscoveryOptions::default())
            .await
            .expect("failed to create network service");

        let mut connection_rx = service.subscribe_connection_events();
        let job_watcher = Arc::clone(&job);
        let watcher = tokio::spawn(async move {
            while let Ok(event) = connection_rx.recv().await {
                if matches!(event, ConnectionEvent::Connected) {
                    job_watcher.trigger();
                    break;
                }
            }
        });

        service.connect().await.expect("connect should succeed");

        let manager_for_check = Arc::clone(&manager);
        let pending = timeout(Duration::from_secs(5), async move {
            loop {
                let queue = manager_for_check
                    .get_pending_sync_queue()
                    .await
                    .expect("failed to query queue");
                if !queue.is_empty() {
                    return queue;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("offline reindex did not enqueue actions in time");

        watcher.abort();
        service
            .disconnect()
            .await
            .expect("disconnect should succeed");

        assert_eq!(pending.len(), 1);
    }
}
