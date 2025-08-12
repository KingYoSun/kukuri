import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { createLocalStoragePersist, createPartializer } from './utils/persistHelpers';
import { offlineApi } from '@/api/offline';
import { errorHandler } from '@/lib/errorHandler';
import type {
  OfflineState,
  OfflineAction,
  SaveOfflineActionRequest,
  OptimisticUpdate,
} from '@/types/offline';
import { EntityType } from '@/types/offline';

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
  
  // 楽観的更新ヘルパー
  applyOptimisticUpdate: <T>(
    entityType: EntityType,
    entityId: string,
    originalData: T,
    updatedData: T
  ) => Promise<string>;
  confirmUpdate: (updateId: string) => Promise<void>;
  rollbackUpdate: (updateId: string) => Promise<void>;
}

export const useOfflineStore = create<OfflineStore>()(
  persist(
    (set, get) => ({
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

      addPendingAction: (action) =>
        set((state) => ({
          pendingActions: [...state.pendingActions, action],
        })),

      removePendingAction: (localId) =>
        set((state) => ({
          pendingActions: state.pendingActions.filter(
            (a) => a.localId !== localId
          ),
        })),

      clearPendingActions: () => set({ pendingActions: [] }),

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

      setSyncError: (actionId, error) =>
        set((state) => {
          const errors = new Map(state.syncErrors);
          errors.set(actionId, error);
          return { syncErrors: errors };
        }),

      clearSyncError: (actionId) =>
        set((state) => {
          const errors = new Map(state.syncErrors);
          errors.delete(actionId);
          return { syncErrors: errors };
        }),

      startSync: () => set({ isSyncing: true }),
      endSync: () => set({ isSyncing: false }),
      updateLastSyncedAt: () => set({ lastSyncedAt: Date.now() }),

      // 非同期アクション
      saveOfflineAction: async (request) => {
        try {
          const response = await offlineApi.saveOfflineAction(request);
          get().addPendingAction(response.action);
          
          // オンラインの場合は即座に同期を試みる
          if (get().isOnline && !get().isSyncing) {
            await get().syncPendingActions(request.userPubkey);
          }
        } catch (error) {
          errorHandler.log('Failed to save offline action', error, {
            context: 'offlineStore.saveOfflineAction'
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
              pendingActions: state.pendingActions.filter(
                (a) => !syncedIds.has(a.localId)
              ),
            }));
          }
          
          set({ lastSyncedAt: Date.now() });
          
          // エラーがあった場合は再試行をスケジュール
          if (response.failedCount > 0) {
            setTimeout(() => {
              get().syncPendingActions(userPubkey);
            }, 30000); // 30秒後に再試行
          }
        } catch (error) {
          errorHandler.log('Sync failed', error, {
            context: 'offlineStore.syncPendingActions'
          });
        } finally {
          set({ isSyncing: false });
        }
      },

      loadPendingActions: async (userPubkey) => {
        try {
          const actions = await offlineApi.getOfflineActions({
            userPubkey,
            isSynced: false,
          });
          set({ pendingActions: actions });
        } catch (error) {
          errorHandler.log('Failed to load pending actions', error, {
            context: 'offlineStore.loadPendingActions'
          });
        }
      },

      cleanupExpiredCache: async () => {
        try {
          const cleanedCount = await offlineApi.cleanupExpiredCache();
          errorHandler.info(`Cleaned up ${cleanedCount} expired cache entries`, 'offlineStore.cleanupExpiredCache');
        } catch (error) {
          errorHandler.log('Failed to cleanup cache', error, {
            context: 'offlineStore.cleanupExpiredCache'
          });
        }
      },

      // 楽観的更新ヘルパー
      applyOptimisticUpdate: async (entityType, entityId, originalData, updatedData) => {
        const originalDataStr = JSON.stringify(originalData);
        const updatedDataStr = JSON.stringify(updatedData);
        
        const updateId = await offlineApi.saveOptimisticUpdate(
          entityType,
          entityId,
          originalDataStr,
          updatedDataStr
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
    }),
    createLocalStoragePersist(
      'offline-store',
      createPartializer(['lastSyncedAt', 'pendingActions', 'syncQueue']),
    )
  )
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
  setInterval(() => {
    useOfflineStore.getState().cleanupExpiredCache();
  }, 60 * 60 * 1000);
}