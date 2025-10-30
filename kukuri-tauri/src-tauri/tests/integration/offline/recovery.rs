use super::offline_support::{
    build_params_for_index, sample_save_params, setup_offline_service, OfflineTestContext,
};
use kukuri_lib::test_support::application::ports::offline_store::OfflinePersistence;
use kukuri_lib::test_support::application::services::offline_service::{
    OfflineActionsQuery, OfflineServiceTrait, SaveOfflineActionParams,
};
use kukuri_lib::test_support::infrastructure::offline::{OfflineReindexJob, SqliteOfflinePersistence};

use std::sync::Arc;

#[tokio::test]
async fn reindex_job_populates_pending_queue_and_reports() {
    let OfflineTestContext {
        service: offline_service,
        pool,
    } = setup_offline_service().await;

    let params: SaveOfflineActionParams = sample_save_params();
    let saved = offline_service.save_action(params).await.unwrap();

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

    let pending_queue = persistence.list_pending_sync_queue().await.unwrap();
    assert_eq!(pending_queue.len(), 1);

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

#[tokio::test]
async fn reindex_job_ignores_already_synced_actions() {
    let OfflineTestContext {
        service: offline_service,
        pool,
    } = setup_offline_service().await;

    let first = offline_service
        .save_action(build_params_for_index(0))
        .await
        .expect("save first action");
    let second = offline_service
        .save_action(build_params_for_index(1))
        .await
        .expect("save second action");

    sqlx::query("UPDATE offline_actions SET is_synced = 1 WHERE local_id = ?")
        .bind(second.local_id.as_str())
        .execute(&pool)
        .await
        .expect("mark second as synced");

    let persistence = Arc::new(SqliteOfflinePersistence::new(pool.clone()));
    let job = OfflineReindexJob::with_emitter(None, persistence.clone());

    let report = job.reindex_once().await.expect("reindex report");

    assert_eq!(report.offline_action_count, 1);
    assert_eq!(report.queued_action_count, 1);
    assert_eq!(report.pending_queue_count, 1);
    assert_eq!(
        report.queued_offline_action_ids,
        vec![first.action.action_id.to_string()]
    );

    let pending_queue = persistence.list_pending_sync_queue().await.unwrap();
    assert_eq!(pending_queue.len(), 1);
    assert_eq!(pending_queue[0].status.as_str(), "pending");

    let synced = offline_service
        .list_actions(OfflineActionsQuery {
            user_pubkey: None,
            include_synced: Some(true),
            limit: None,
        })
        .await
        .expect("list synced")
        .len();
    assert_eq!(synced, 1);

    let unsynced = offline_service
        .list_actions(OfflineActionsQuery {
            user_pubkey: None,
            include_synced: Some(false),
            limit: None,
        })
        .await
        .expect("list unsynced")
        .len();
    assert_eq!(unsynced, 1);

}
