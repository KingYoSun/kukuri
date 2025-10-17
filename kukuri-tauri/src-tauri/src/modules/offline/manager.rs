use anyhow::Result;
use chrono::Utc;
use sqlx::{Pool, Row, Sqlite};
use uuid::Uuid;

use super::models::*;

pub struct OfflineManager {
    pool: Pool<Sqlite>,
}

impl OfflineManager {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    // オフラインアクションの保存
    pub async fn save_offline_action(
        &self,
        request: SaveOfflineActionRequest,
    ) -> Result<SaveOfflineActionResponse> {
        let local_id = Uuid::new_v4().to_string();
        let action_data = serde_json::to_string(&request.action_data)?;
        let created_at = Utc::now().timestamp();

        let result = sqlx::query(
            r#"
            INSERT INTO offline_actions (
                user_pubkey, action_type, target_id, action_data, 
                local_id, is_synced, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)
            "#,
        )
        .bind(&request.user_pubkey)
        .bind(&request.action_type)
        .bind(&request.target_id)
        .bind(&action_data)
        .bind(&local_id)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        let id = result.last_insert_rowid();
        let action = self.get_offline_action_by_id(id).await?;

        Ok(SaveOfflineActionResponse {
            local_id: action.local_id.clone(),
            action,
        })
    }

    // オフラインアクションの取得
    pub async fn get_offline_actions(
        &self,
        _request: GetOfflineActionsRequest,
    ) -> Result<Vec<OfflineAction>> {
        // シンプルな実装に変更
        let actions = sqlx::query_as::<_, OfflineAction>(
            "SELECT * FROM offline_actions WHERE is_synced = 0 ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(actions)
    }

    // オフラインアクションの同期
    pub async fn sync_offline_actions(
        &self,
        request: SyncOfflineActionsRequest,
    ) -> Result<SyncOfflineActionsResponse> {
        // 未同期のアクションを取得
        let unsynced_actions = sqlx::query_as::<_, OfflineAction>(
            r#"
            SELECT * FROM offline_actions
            WHERE user_pubkey = ?1 AND is_synced = 0
            ORDER BY created_at ASC
            "#,
        )
        .bind(&request.user_pubkey)
        .fetch_all(&self.pool)
        .await?;

        let mut synced_count = 0;
        let failed_count = 0;

        for action in unsynced_actions.iter() {
            // 同期キューに追加
            let result = self
                .add_to_sync_queue(AddToSyncQueueRequest {
                    action_type: action.action_type.clone(),
                    payload: serde_json::from_str(&action.action_data)?,
                })
                .await;

            if result.is_ok() {
                // アクションを同期済みとしてマーク
                let synced_at = Utc::now().timestamp();
                sqlx::query(
                    r#"
                    UPDATE offline_actions 
                    SET is_synced = 1, synced_at = ?1
                    WHERE id = ?2
                    "#,
                )
                .bind(synced_at)
                .bind(action.id)
                .execute(&self.pool)
                .await?;
                synced_count += 1;
            }
        }

        let pending_result = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM offline_actions
            WHERE user_pubkey = ?1 AND is_synced = 0
            "#,
        )
        .bind(&request.user_pubkey)
        .fetch_one(&self.pool)
        .await?;

        let pending_count: i32 = pending_result.try_get("count").unwrap_or(0);

        Ok(SyncOfflineActionsResponse {
            synced_count,
            failed_count,
            pending_count,
        })
    }

    // キャッシュステータスの取得
    pub async fn get_cache_status(&self) -> Result<CacheStatusResponse> {
        let total_result = sqlx::query(r#"SELECT COUNT(*) as count FROM cache_metadata"#)
            .fetch_one(&self.pool)
            .await?;
        let total_items: i64 = total_result.try_get("count").unwrap_or(0);

        let stale_result =
            sqlx::query(r#"SELECT COUNT(*) as count FROM cache_metadata WHERE is_stale = 1"#)
                .fetch_one(&self.pool)
                .await?;
        let stale_items: i64 = stale_result.try_get("count").unwrap_or(0);

        let cache_types_result = sqlx::query(
            r#"
            SELECT 
                cache_type,
                COUNT(*) as item_count,
                MAX(last_synced_at) as last_synced_at,
                MAX(is_stale) as is_stale
            FROM cache_metadata
            GROUP BY cache_type
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let cache_types: Vec<CacheTypeStatus> = cache_types_result
            .into_iter()
            .map(|row| CacheTypeStatus {
                cache_type: row.try_get("cache_type").unwrap_or_default(),
                item_count: row.try_get("item_count").unwrap_or(0),
                last_synced_at: row.try_get("last_synced_at").ok(),
                is_stale: row.try_get::<i32, _>("is_stale").unwrap_or(0) > 0,
            })
            .collect();

        Ok(CacheStatusResponse {
            total_items,
            stale_items,
            cache_types,
        })
    }

    // 同期キューへの追加
    pub async fn add_to_sync_queue(&self, request: AddToSyncQueueRequest) -> Result<i64> {
        let payload = serde_json::to_string(&request.payload)?;
        let created_at = Utc::now().timestamp();

        let result = sqlx::query(
            r#"
            INSERT INTO sync_queue (action_type, payload, status, created_at, updated_at)
            VALUES (?1, ?2, 'pending', ?3, ?3)
            "#,
        )
        .bind(&request.action_type)
        .bind(&payload)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    // キャッシュメタデータの更新
    pub async fn update_cache_metadata(&self, request: UpdateCacheMetadataRequest) -> Result<()> {
        let now = Utc::now().timestamp();
        let metadata = request
            .metadata
            .map(|m| serde_json::to_string(&m))
            .transpose()?;
        let expiry_time = request.expiry_seconds.map(|seconds| now + seconds);

        sqlx::query(
            r#"
            INSERT INTO cache_metadata (
                cache_key, cache_type, last_synced_at, last_accessed_at,
                data_version, is_stale, expiry_time, metadata
            ) VALUES (?1, ?2, ?3, ?3, 1, 0, ?4, ?5)
            ON CONFLICT(cache_key) DO UPDATE SET
                cache_type = excluded.cache_type,
                last_synced_at = excluded.last_synced_at,
                last_accessed_at = excluded.last_accessed_at,
                data_version = data_version + 1,
                expiry_time = excluded.expiry_time,
                metadata = excluded.metadata
            "#,
        )
        .bind(&request.cache_key)
        .bind(&request.cache_type)
        .bind(now)
        .bind(expiry_time)
        .bind(metadata)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // 楽観的更新の保存
    pub async fn save_optimistic_update(
        &self,
        entity_type: String,
        entity_id: String,
        original_data: Option<String>,
        updated_data: String,
    ) -> Result<String> {
        let update_id = Uuid::new_v4().to_string();
        let created_at = Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO optimistic_updates (
                update_id, entity_type, entity_id, original_data,
                updated_data, is_confirmed, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)
            "#,
        )
        .bind(&update_id)
        .bind(&entity_type)
        .bind(&entity_id)
        .bind(&original_data)
        .bind(&updated_data)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        Ok(update_id)
    }

    // 楽観的更新の確認
    pub async fn confirm_optimistic_update(&self, update_id: String) -> Result<()> {
        let confirmed_at = Utc::now().timestamp();

        sqlx::query(
            r#"
            UPDATE optimistic_updates
            SET is_confirmed = 1, confirmed_at = ?1
            WHERE update_id = ?2
            "#,
        )
        .bind(confirmed_at)
        .bind(&update_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // 楽観的更新のロールバック
    pub async fn rollback_optimistic_update(&self, update_id: String) -> Result<Option<String>> {
        let update = sqlx::query_as::<_, OptimisticUpdate>(
            r#"
            SELECT * FROM optimistic_updates
            WHERE update_id = ?1
            "#,
        )
        .bind(&update_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(update) = update {
            // 削除
            sqlx::query(r#"DELETE FROM optimistic_updates WHERE update_id = ?1"#)
                .bind(&update_id)
                .execute(&self.pool)
                .await?;

            Ok(update.original_data)
        } else {
            Ok(None)
        }
    }

    // ヘルパーメソッド
    async fn get_offline_action_by_id(&self, id: i64) -> Result<OfflineAction> {
        let action = sqlx::query_as::<_, OfflineAction>(
            r#"
            SELECT * FROM offline_actions
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(action)
    }

    // 期限切れキャッシュのクリーンアップ
    pub async fn cleanup_expired_cache(&self) -> Result<i32> {
        let now = Utc::now().timestamp();

        let result = sqlx::query(
            r#"
            DELETE FROM cache_metadata
            WHERE expiry_time IS NOT NULL AND expiry_time < ?1
            "#,
        )
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i32)
    }

    // 同期状態の更新
    pub async fn update_sync_status(
        &self,
        entity_type: String,
        entity_id: String,
        sync_status: String,
        conflict_data: Option<String>,
    ) -> Result<()> {
        let now = Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO sync_status (
                entity_type, entity_id, local_version, last_local_update,
                sync_status, conflict_data
            ) VALUES (?1, ?2, 1, ?3, ?4, ?5)
            ON CONFLICT(entity_type, entity_id) DO UPDATE SET
                local_version = local_version + 1,
                last_local_update = excluded.last_local_update,
                sync_status = excluded.sync_status,
                conflict_data = excluded.conflict_data
            "#,
        )
        .bind(&entity_type)
        .bind(&entity_id)
        .bind(now)
        .bind(&sync_status)
        .bind(&conflict_data)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 指定されたオフラインアクションが同期キューに存在しない場合のみ追加する
    pub async fn ensure_offline_action_in_queue(&self, action: &OfflineAction) -> Result<bool> {
        let payload_value: serde_json::Value = serde_json::from_str(&action.action_data)?;
        let payload = serde_json::to_string(&payload_value)?;

        let existing = sqlx::query(
            r#"
            SELECT id FROM sync_queue
            WHERE action_type = ?1 AND payload = ?2
              AND status IN ('pending', 'failed')
            LIMIT 1
            "#,
        )
        .bind(&action.action_type)
        .bind(&payload)
        .fetch_optional(&self.pool)
        .await?;

        if existing.is_some() {
            return Ok(false);
        }

        self.add_to_sync_queue(AddToSyncQueueRequest {
            action_type: action.action_type.clone(),
            payload: payload_value,
        })
        .await?;

        Ok(true)
    }

    /// キュー内の未処理/失敗アクションを取得する
    pub async fn get_pending_sync_queue(&self) -> Result<Vec<SyncQueueItem>> {
        let items = sqlx::query_as::<_, SyncQueueItem>(
            r#"
            SELECT * FROM sync_queue
            WHERE status IN ('pending', 'failed')
            ORDER BY updated_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(items)
    }

    /// 期限切れ・ステール判定済みのキャッシュメタデータを取得する
    pub async fn get_stale_cache_entries(&self) -> Result<Vec<CacheMetadata>> {
        let now = Utc::now().timestamp();
        let items = sqlx::query_as::<_, CacheMetadata>(
            r#"
            SELECT * FROM cache_metadata
            WHERE is_stale = 1
               OR (expiry_time IS NOT NULL AND expiry_time < ?1)
            ORDER BY COALESCE(last_synced_at, 0) ASC
            "#,
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        Ok(items)
    }

    /// 未確定の楽観的更新を取得する
    pub async fn get_unconfirmed_updates(&self) -> Result<Vec<OptimisticUpdate>> {
        let items = sqlx::query_as::<_, OptimisticUpdate>(
            r#"
            SELECT * FROM optimistic_updates
            WHERE is_confirmed = 0
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(items)
    }

    /// コンフリクトや失敗状態の同期記録を取得する
    pub async fn get_sync_conflicts(&self) -> Result<Vec<SyncStatusRecord>> {
        let items = sqlx::query_as::<_, SyncStatusRecord>(
            r#"
            SELECT * FROM sync_status
            WHERE sync_status IN ('conflict', 'failed', 'pending')
            ORDER BY last_local_update DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(items)
    }
}
