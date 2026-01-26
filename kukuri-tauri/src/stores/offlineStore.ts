import { create } from 'zustand';

import { offlineApi } from '@/api/offline';
import { errorHandler } from '@/lib/errorHandler';
import type {
  OfflineState,
  OfflineAction,
  SaveOfflineActionRequest,
  OptimisticUpdate,
  OfflineReindexReport,
} from '@/types/offline';
import { EntityType } from '@/types/offline';
import { withPersist } from './utils/persistHelpers';
import { createOfflinePersistConfig } from './config/persist';

const OFFLINE_CACHE_KEY = 'offline_actions';
const OFFLINE_CACHE_TYPE = 'sync_queue';
const CACHE_METADATA_TTL_SECONDS = 60 * 60; // 1 hour

const nextLastSyncedAt = (current?: number) => {
  const now = Date.now();
  if (typeof current !== 'number') {
    return now;
  }
  return now > current ? now : current + 1;
};

declare global {
  interface Window {
    __TAURI__?: unknown;
  }
}

interface OfflineStore extends OfflineState {
  // アクション
  setOnlineStatus: (isOnline: boolean) => void;
  addPendingAction: (action: OfflineAction) => void;
  removePendingAction: (localId: string) => void;
  clearPendingActions: () => void;
  addOptimisticUpdate: (update: OptimisticUpdate) => void;
  confirmOptimisticUpdate: (updateId: string) => void;
  rollbackOptimisticUpdate: (updateId: string) => void;
  setSyncError: (actionId: string, error: string) => void;
  clearSyncError: (actionId: string) => void;
  startSync: () => void;
  endSync: () => void;
  updateLastSyncedAt: () => void;

  // 非同期アクション
  saveOfflineAction: (request: SaveOfflineActionRequest) => Promise<void>;
  syncPendingActions: (userPubkey: string) => Promise<void>;
  loadPendingActions: (userPubkey?: string) => Promise<void>;
  cleanupExpiredCache: () => Promise<void>;
  refreshCacheMetadata: () => Promise<void>;

  // 楽観的更新ヘルパー
  applyOptimisticUpdate: <T>(
    entityType: EntityType,
    entityId: string,
    originalData: T,
    updatedData: T,
  ) => Promise<string>;
  confirmUpdate: (updateId: string) => Promise<void>;
  rollbackUpdate: (updateId: string) => Promise<string | null>;
}

export const useOfflineStore = create<OfflineStore>()(
  withPersist<OfflineStore>((set, get) => {
    const refreshMetadata = async () => {
      const state = get();
      try {
        await offlineApi.updateCacheMetadata({
          cacheKey: OFFLINE_CACHE_KEY,
          cacheType: OFFLINE_CACHE_TYPE,
          metadata: {
            pendingCount: state.pendingActions.length,
            syncErrorCount: state.syncErrors.size,
            isSyncing: state.isSyncing,
            lastSyncedAt: state.lastSyncedAt ?? null,
            updatedAt: Date.now(),
          },
          expirySeconds: CACHE_METADATA_TTL_SECONDS,
          isStale: state.pendingActions.length > 0 || state.syncErrors.size > 0,
        });
      } catch (error) {
        errorHandler.log('Failed to update cache metadata', error, {
          context: 'offlineStore.refreshCacheMetadata',
        });
      }
    };

    return {
      // 初期状態
      isOnline: navigator.onLine,
      lastSyncedAt: undefined,
      pendingActions: [],
      syncQueue: [],
      optimisticUpdates: new Map(),
      isSyncing: false,
      syncErrors: new Map(),

      // 同期アクション
      setOnlineStatus: (isOnline) => set({ isOnline }),

      addPendingAction: (action) => {
        set((state) => ({
          pendingActions: [...state.pendingActions, action],
        }));
        void refreshMetadata();
      },

      removePendingAction: (localId) => {
        set((state) => ({
          pendingActions: state.pendingActions.filter((a) => a.localId !== localId),
        }));
        void refreshMetadata();
      },

      clearPendingActions: () => {
        set({ pendingActions: [] });
        void refreshMetadata();
      },

      addOptimisticUpdate: (update) =>
        set((state) => {
          const updates = new Map(state.optimisticUpdates);
          updates.set(update.updateId, update);
          return { optimisticUpdates: updates };
        }),

      confirmOptimisticUpdate: (updateId) =>
        set((state) => {
          const updates = new Map(state.optimisticUpdates);
          const update = updates.get(updateId);
          if (update) {
            update.isConfirmed = true;
            update.confirmedAt = Date.now();
            updates.set(updateId, update);
          }
          return { optimisticUpdates: updates };
        }),

      rollbackOptimisticUpdate: (updateId) =>
        set((state) => {
          const updates = new Map(state.optimisticUpdates);
          updates.delete(updateId);
          return { optimisticUpdates: updates };
        }),

      setSyncError: (actionId, error) => {
        set((state) => {
          const errors = new Map(state.syncErrors);
          errors.set(actionId, error);
          return { syncErrors: errors };
        });
        void refreshMetadata();
      },

      clearSyncError: (actionId) => {
        set((state) => {
          const errors = new Map(state.syncErrors);
          errors.delete(actionId);
          return { syncErrors: errors };
        });
        void refreshMetadata();
      },

      startSync: () => set({ isSyncing: true }),
      endSync: () => set({ isSyncing: false }),
      updateLastSyncedAt: () =>
        set((state) => ({ lastSyncedAt: nextLastSyncedAt(state.lastSyncedAt) })),

      // 非同期アクション
      saveOfflineAction: async (request) => {
        try {
          const response = await offlineApi.saveOfflineAction(request);
          get().addPendingAction(response.action);

          try {
            await offlineApi.updateSyncStatus(
              request.entityType,
              request.entityId,
              'pending',
              null,
            );
          } catch (error) {
            errorHandler.log('Failed to update sync status (pending)', error, {
              context: 'offlineStore.saveOfflineAction',
            });
          }

          await refreshMetadata();

          // オンラインの場合は即座に同期を試みる
          if (get().isOnline && !get().isSyncing) {
            await get().syncPendingActions(request.userPubkey);
          }
        } catch (error) {
          errorHandler.log('Failed to save offline action', error, {
            context: 'offlineStore.saveOfflineAction',
          });
          throw error;
        }
      },

      syncPendingActions: async (userPubkey) => {
        if (get().isSyncing || !get().isOnline) return;

        try {
          set({ isSyncing: true });

          const response = await offlineApi.syncOfflineActions({ userPubkey });

          if (response.syncedCount > 0) {
            // 同期済みのアクションを削除
            const syncedActions = await offlineApi.getOfflineActions({
              userPubkey,
              isSynced: true,
            });

            const syncedIds = new Set(syncedActions.map((a) => a.localId));
            set((state) => ({
              pendingActions: state.pendingActions.filter((a) => !syncedIds.has(a.localId)),
            }));
          }

          set((state) => ({ lastSyncedAt: nextLastSyncedAt(state.lastSyncedAt) }));

          // エラーがあった場合は再試行をスケジュール
          if (response.failedCount > 0) {
            setTimeout(() => {
              get().syncPendingActions(userPubkey);
            }, 30000); // 30秒後に再試行
          }
        } catch (error) {
          errorHandler.log('Sync failed', error, {
            context: 'offlineStore.syncPendingActions',
          });
        } finally {
          set({ isSyncing: false });
          await refreshMetadata();
        }
      },

      loadPendingActions: async (userPubkey) => {
        try {
          const actions = await offlineApi.getOfflineActions({
            userPubkey,
            isSynced: false,
          });
          set({ pendingActions: actions });
          await refreshMetadata();
        } catch (error) {
          errorHandler.log('Failed to load pending actions', error, {
            context: 'offlineStore.loadPendingActions',
          });
        }
      },

      cleanupExpiredCache: async () => {
        try {
          const cleanedCount = await offlineApi.cleanupExpiredCache();
          errorHandler.info(
            `Cleaned up ${cleanedCount} expired cache entries`,
            'offlineStore.cleanupExpiredCache',
          );
        } catch (error) {
          errorHandler.log('Failed to cleanup cache', error, {
            context: 'offlineStore.cleanupExpiredCache',
          });
        }
      },

      refreshCacheMetadata: refreshMetadata,

      // 楽観的更新ヘルパー
      applyOptimisticUpdate: async (entityType, entityId, originalData, updatedData) => {
        const originalDataStr = JSON.stringify(originalData);
        const updatedDataStr = JSON.stringify(updatedData);

        const updateId = await offlineApi.saveOptimisticUpdate(
          entityType,
          entityId,
          originalDataStr,
          updatedDataStr,
        );

        get().addOptimisticUpdate({
          id: 0, // サーバーから返される
          updateId,
          entityType,
          entityId,
          originalData: originalDataStr,
          updatedData: updatedDataStr,
          isConfirmed: false,
          createdAt: Date.now(),
        });

        return updateId;
      },

      confirmUpdate: async (updateId) => {
        await offlineApi.confirmOptimisticUpdate(updateId);
        set((state) => {
          const updates = new Map(state.optimisticUpdates);
          const update = updates.get(updateId);
          if (update) {
            update.isConfirmed = true;
            update.confirmedAt = Date.now();
            updates.set(updateId, update);
          }
          return { optimisticUpdates: updates };
        });
      },

      rollbackUpdate: async (updateId) => {
        const originalData = await offlineApi.rollbackOptimisticUpdate(updateId);
        set((state) => {
          const updates = new Map(state.optimisticUpdates);
          updates.delete(updateId);
          return { optimisticUpdates: updates };
        });
        return originalData;
      },
    };
  }, createOfflinePersistConfig<OfflineStore>()),
);

// オンライン/オフライン状態の監視を設定
if (typeof window !== 'undefined') {
  window.addEventListener('online', () => {
    const store = useOfflineStore.getState();
    store.setOnlineStatus(true);

    // オンラインになったら自動的に同期を開始
    const userPubkey = localStorage.getItem('currentUserPubkey');
    if (userPubkey && store.pendingActions.length > 0) {
      store.syncPendingActions(userPubkey);
    }
  });

  window.addEventListener('offline', () => {
    useOfflineStore.getState().setOnlineStatus(false);
  });

  // 定期的なキャッシュクリーンアップ（1時間ごと）
  setInterval(
    () => {
      useOfflineStore.getState().cleanupExpiredCache();
    },
    60 * 60 * 1000,
  );

  if (window.__TAURI__) {
    void import('@tauri-apps/api/event')
      .then(({ listen }) => {
        void listen<OfflineReindexReport>('offline://reindex_complete', async ({ payload }) => {
          const store = useOfflineStore.getState();
          const userPubkey = localStorage.getItem('currentUserPubkey') ?? undefined;
          try {
            await store.loadPendingActions(userPubkey || undefined);
            store.updateLastSyncedAt();
          } catch (error) {
            errorHandler.log('Failed to refresh pending actions after reindex', error, {
              context: 'offlineStore.reindex',
            });
          }

          if (payload.queued_action_count > 0) {
            errorHandler.info(
              `再索引で ${payload.queued_action_count} 件のアクションを同期キューに再投入しました`,
              'offlineStore.reindex',
            );
          }

          if (payload.sync_conflicts.length > 0) {
            errorHandler.warn(
              `未解決の同期コンフリクトが ${payload.sync_conflicts.length} 件あります`,
              'offlineStore.reindex',
            );
          }
        }).catch((error) => {
          errorHandler.log('Failed to subscribe offline reindex completion event', error, {
            context: 'offlineStore.reindex',
          });
        });

        void listen<string>('offline://reindex_failed', ({ payload }) => {
          errorHandler.warn(`オフライン再索引に失敗しました: ${payload}`, 'offlineStore.reindex');
        }).catch((error) => {
          errorHandler.log('Failed to subscribe offline reindex failure event', error, {
            context: 'offlineStore.reindex',
          });
        });
      })
      .catch((error) => {
        errorHandler.log('Failed to setup Tauri event listeners for offline reindex', error, {
          context: 'offlineStore.reindex',
        });
      });
  }
}
