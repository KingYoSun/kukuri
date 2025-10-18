import { invokeCommand, invokeCommandVoid } from '@/lib/api/tauriClient';
import type {
  SaveOfflineActionRequest,
  SaveOfflineActionResponse,
  GetOfflineActionsRequest,
  OfflineAction,
  SyncOfflineActionsRequest,
  SyncOfflineActionsResponse,
  CacheStatusResponse,
  AddToSyncQueueRequest,
  UpdateCacheMetadataRequest,
} from '@/types/offline';

/**
 * オフラインストレージAPI
 */
export const offlineApi = {
  /**
   * オフラインアクションを保存
   */
  async saveOfflineAction(request: SaveOfflineActionRequest): Promise<SaveOfflineActionResponse> {
    return invokeCommand('save_offline_action', { request });
  },

  /**
   * オフラインアクションを取得
   */
  async getOfflineActions(request: GetOfflineActionsRequest = {}): Promise<OfflineAction[]> {
    return invokeCommand('get_offline_actions', { request });
  },

  /**
   * オフラインアクションを同期
   */
  async syncOfflineActions(
    request: SyncOfflineActionsRequest,
  ): Promise<SyncOfflineActionsResponse> {
    return invokeCommand('sync_offline_actions', { request });
  },

  /**
   * キャッシュステータスを取得
   */
  async getCacheStatus(): Promise<CacheStatusResponse> {
    return invokeCommand('get_cache_status');
  },

  /**
   * 同期キューに追加
   */
  async addToSyncQueue(request: AddToSyncQueueRequest): Promise<number> {
    return invokeCommand('add_to_sync_queue', { request });
  },

  /**
   * キャッシュメタデータを更新
   */
  async updateCacheMetadata(request: UpdateCacheMetadataRequest): Promise<void> {
    await invokeCommandVoid('update_cache_metadata', { request });
  },

  /**
   * 楽観的更新を保存
   */
  async saveOptimisticUpdate(
    entityType: string,
    entityId: string,
    originalData: string | null,
    updatedData: string,
  ): Promise<string> {
    return invokeCommand('save_optimistic_update', {
      entityType,
      entityId,
      originalData,
      updatedData,
    });
  },

  /**
   * 楽観的更新を確認
   */
  async confirmOptimisticUpdate(updateId: string): Promise<void> {
    await invokeCommandVoid('confirm_optimistic_update', { updateId });
  },

  /**
   * 楽観的更新をロールバック
   */
  async rollbackOptimisticUpdate(updateId: string): Promise<string | null> {
    return invokeCommand('rollback_optimistic_update', { updateId });
  },

  /**
   * 期限切れキャッシュをクリーンアップ
   */
  async cleanupExpiredCache(): Promise<number> {
    return invokeCommand('cleanup_expired_cache');
  },

  /**
   * 同期ステータスを更新
   */
  async updateSyncStatus(
    entityType: string,
    entityId: string,
    syncStatus: string,
    conflictData: string | null = null,
  ): Promise<void> {
    await invokeCommandVoid('update_sync_status', {
      entityType,
      entityId,
      syncStatus,
      conflictData,
    });
  },
};
