use anyhow::Result;
use chrono::Utc;
use sqlx::{Pool, Sqlite};
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

        let id = sqlx::query(
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
        .await?
        .last_insert_rowid();

        let action = self.get_offline_action_by_id(id).await?;

        Ok(SaveOfflineActionResponse {
            local_id: action.local_id.clone(),
            action,
        })
    }

    // オフラインアクションの取得
    pub async fn get_offline_actions(
        &self,
        request: GetOfflineActionsRequest,
    ) -> Result<Vec<OfflineAction>> {
        let mut query = String::from("SELECT * FROM offline_actions WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        if let Some(user_pubkey) = request.user_pubkey {
            query.push_str(" AND user_pubkey = ?");
            params.push(user_pubkey);
        }

        if let Some(is_synced) = request.is_synced {
            query.push_str(" AND is_synced = ?");
            params.push(if is_synced { "1" } else { "0" }.to_string());
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = request.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let actions = sqlx::query_as::<_, OfflineAction>(&query)
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
        let unsynced_actions = sqlx::query_as!(
            OfflineAction,
            r#"
            SELECT id, user_pubkey, action_type, target_id, action_data,
                   local_id, remote_id, is_synced as "is_synced: bool", 
                   created_at, synced_at
            FROM offline_actions
            WHERE user_pubkey = ?1 AND is_synced = 0
            ORDER BY created_at ASC
            "#,
            request.user_pubkey
        )
        .fetch_all(&self.pool)
        .await?;

        let mut synced_count = 0;
        let mut failed_count = 0;

        for action in unsynced_actions.iter() {
            // 同期キューに追加
            let result = self
                .add_to_sync_queue(AddToSyncQueueRequest {
                    action_type: action.action_type.clone(),
                    payload: serde_json::from_str(&action.action_data)?,
                })
                .await;

            match result {
                Ok(_) => {
                    // アクションを同期済みとしてマーク
                    let synced_at = Utc::now().timestamp();
                    sqlx::query!(
                        r#"
                        UPDATE offline_actions 
                        SET is_synced = 1, synced_at = ?1
                        WHERE id = ?2
                        "#,
                        synced_at,
                        action.id
                    )
                    .execute(&self.pool)
                    .await?;
                    synced_count += 1;
                }
                Err(_) => {
                    failed_count += 1;
                }
            }
        }

        let pending_count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as count
            FROM offline_actions
            WHERE user_pubkey = ?1 AND is_synced = 0
            "#,
            request.user_pubkey
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0) as i32;

        Ok(SyncOfflineActionsResponse {
            synced_count,
            failed_count,
            pending_count,
        })
    }

    // キャッシュステータスの取得
    pub async fn get_cache_status(&self) -> Result<CacheStatusResponse> {
        let total_items = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as count FROM cache_metadata"#
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        let stale_items = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as count FROM cache_metadata WHERE is_stale = 1"#
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        let cache_types = sqlx::query!(
            r#"
            SELECT 
                cache_type,
                COUNT(*) as item_count,
                MAX(last_synced_at) as last_synced_at,
                MAX(is_stale) as is_stale
            FROM cache_metadata
            GROUP BY cache_type
            "#
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| CacheTypeStatus {
            cache_type: row.cache_type,
            item_count: row.item_count.unwrap_or(0),
            last_synced_at: row.last_synced_at,
            is_stale: row.is_stale.unwrap_or(0) > 0,
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

        let id = sqlx::query!(
            r#"
            INSERT INTO sync_queue (action_type, payload, status, created_at, updated_at)
            VALUES (?1, ?2, 'pending', ?3, ?3)
            "#,
            request.action_type,
            payload,
            created_at
        )
        .execute(&self.pool)
        .await?
        .last_insert_rowid();

        Ok(id)
    }

    // キャッシュメタデータの更新
    pub async fn update_cache_metadata(
        &self,
        request: UpdateCacheMetadataRequest,
    ) -> Result<()> {
        let now = Utc::now().timestamp();
        let metadata = request
            .metadata
            .map(|m| serde_json::to_string(&m))
            .transpose()?;
        let expiry_time = request
            .expiry_seconds
            .map(|seconds| now + seconds);

        sqlx::query!(
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
            request.cache_key,
            request.cache_type,
            now,
            expiry_time,
            metadata
        )
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

        sqlx::query!(
            r#"
            INSERT INTO optimistic_updates (
                update_id, entity_type, entity_id, original_data,
                updated_data, is_confirmed, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)
            "#,
            update_id,
            entity_type,
            entity_id,
            original_data,
            updated_data,
            created_at
        )
        .execute(&self.pool)
        .await?;

        Ok(update_id)
    }

    // 楽観的更新の確認
    pub async fn confirm_optimistic_update(&self, update_id: String) -> Result<()> {
        let confirmed_at = Utc::now().timestamp();

        sqlx::query!(
            r#"
            UPDATE optimistic_updates
            SET is_confirmed = 1, confirmed_at = ?1
            WHERE update_id = ?2
            "#,
            confirmed_at,
            update_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // 楽観的更新のロールバック
    pub async fn rollback_optimistic_update(&self, update_id: String) -> Result<Option<String>> {
        let update = sqlx::query_as!(
            OptimisticUpdate,
            r#"
            SELECT id, update_id, entity_type, entity_id, original_data,
                   updated_data, is_confirmed as "is_confirmed: bool", 
                   created_at, confirmed_at
            FROM optimistic_updates
            WHERE update_id = ?1
            "#,
            update_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(update) = update {
            // 削除
            sqlx::query!(
                r#"DELETE FROM optimistic_updates WHERE update_id = ?1"#,
                update_id
            )
            .execute(&self.pool)
            .await?;

            Ok(update.original_data)
        } else {
            Ok(None)
        }
    }

    // ヘルパーメソッド
    async fn get_offline_action_by_id(&self, id: i64) -> Result<OfflineAction> {
        let action = sqlx::query_as!(
            OfflineAction,
            r#"
            SELECT id, user_pubkey, action_type, target_id, action_data,
                   local_id, remote_id, is_synced as "is_synced: bool", 
                   created_at, synced_at
            FROM offline_actions
            WHERE id = ?1
            "#,
            id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(action)
    }

    // 期限切れキャッシュのクリーンアップ
    pub async fn cleanup_expired_cache(&self) -> Result<i32> {
        let now = Utc::now().timestamp();

        let result = sqlx::query!(
            r#"
            DELETE FROM cache_metadata
            WHERE expiry_time IS NOT NULL AND expiry_time < ?1
            "#,
            now
        )
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

        sqlx::query!(
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
            entity_type,
            entity_id,
            now,
            sync_status,
            conflict_data
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}