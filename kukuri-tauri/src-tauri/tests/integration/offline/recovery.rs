use super::{sample_save_params, setup_service};
use kukuri_lib::test_support::application::ports::offline_store::OfflinePersistence;
use kukuri_lib::test_support::application::services::offline_service::{
    OfflineActionsQuery, OfflineServiceTrait, SaveOfflineActionParams,
};
use kukuri_lib::test_support::infrastructure::offline::OfflineReindexJob;
use kukuri_lib::test_support::infrastructure::offline::SqliteOfflinePersistence;
use std::sync::Arc;

#[tokio::test]
async fn reindex_job_populates_pending_queue_and_reports() {
    let (offline_service, pool) = setup_service().await;

    // 保存するアクションを作成
    let params: SaveOfflineActionParams = sample_save_params();
    let saved = offline_service.save_action(params).await.unwrap();

    // 同じプールを共有する永続化レイヤーを作成
    let persistence = Arc::new(SqliteOfflinePersistence::new(pool.clone()));
    let persistence_trait: Arc<dyn OfflinePersistence> = persistence.clone();

    let job = OfflineReindexJob::with_emitter(None, persistence_trait.clone());
    let report = job.reindex_once().await.unwrap();

    assert_eq!(report.offline_action_count, 1);
    assert_eq!(report.queued_action_count, 1);
    assert_eq!(report.pending_queue_count, 1);
    assert_eq!(
        report.queued_offline_action_ids,
        vec![saved.action.action_id.to_string()]
    );

    // キューへ再投入されていることを確認
    let pending_queue = persistence.list_pending_sync_queue().await.unwrap();
    assert_eq!(pending_queue.len(), 1);

    // オフラインアクション自体はまだ未同期のまま
    let unsynced = offline_service
        .list_actions(OfflineActionsQuery {
            user_pubkey: None,
            include_synced: Some(false),
            limit: None,
        })
        .await
        .unwrap();
    assert_eq!(unsynced.len(), 1);
}
