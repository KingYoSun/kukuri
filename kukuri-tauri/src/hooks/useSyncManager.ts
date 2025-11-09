import { useState, useCallback, useEffect } from 'react';
import { useOfflineStore } from '@/stores/offlineStore';
import { useAuthStore } from '@/stores/authStore';
import { syncEngine, type SyncResult, type SyncConflict } from '@/lib/sync/syncEngine';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';
import { offlineApi } from '@/api/offline';
import type { CacheStatusResponse, OfflineAction, SyncQueueItem } from '@/types/offline';
import { OfflineActionType } from '@/types/offline';

export interface SyncStatus {
  isSyncing: boolean;
  progress: number;
  totalItems: number;
  syncedItems: number;
  conflicts: SyncConflict[];
  lastSyncTime?: Date;
  error?: string;
}

const SYNC_QUEUE_HISTORY_LIMIT = 30;

function inferEntityType(actionType: string): string | null {
  switch (actionType) {
    case OfflineActionType.CREATE_POST:
    case OfflineActionType.DELETE_POST:
    case OfflineActionType.LIKE_POST:
    case OfflineActionType.LIKE:
    case OfflineActionType.BOOST:
    case OfflineActionType.BOOKMARK:
    case OfflineActionType.UNBOOKMARK:
      return 'post';
    case OfflineActionType.FOLLOW:
    case OfflineActionType.UNFOLLOW:
    case OfflineActionType.PROFILE_UPDATE:
      return 'user';
    case OfflineActionType.JOIN_TOPIC:
    case OfflineActionType.LEAVE_TOPIC:
    case OfflineActionType.TOPIC_JOIN:
    case OfflineActionType.TOPIC_LEAVE:
      return 'topic_membership';
    case OfflineActionType.TOPIC_CREATE:
    case OfflineActionType.TOPIC_UPDATE:
    case OfflineActionType.TOPIC_DELETE:
      return 'topic';
    default:
      return null;
  }
}

function extractEntityContext(
  action: OfflineAction,
): { entityType: string; entityId: string } | null {
  try {
    const rawData = action.actionData;
    const data =
      typeof rawData === 'string' && rawData.length > 0 ? JSON.parse(rawData) : (rawData as any);
    const entityType: string | null =
      (data && typeof data.entityType === 'string' && data.entityType) ??
      inferEntityType(action.actionType);
    const candidateId =
      (data && data.entityId) ??
      action.targetId ??
      (data && (data.topicId || data.postId || data.userId)) ??
      action.localId;

    if (!entityType || !candidateId) {
      return null;
    }

    return {
      entityType,
      entityId: String(candidateId),
    };
  } catch {
    return null;
  }
}

export function useSyncManager() {
  const {
    pendingActions,
    isOnline,
    lastSyncedAt,
    syncPendingActions,
    setSyncError,
    clearSyncError,
    refreshCacheMetadata,
  } = useOfflineStore();

  const { currentUser } = useAuthStore();

  const [syncStatus, setSyncStatus] = useState<SyncStatus>({
    isSyncing: false,
    progress: 0,
    totalItems: 0,
    syncedItems: 0,
    conflicts: [],
    lastSyncTime: lastSyncedAt ? new Date(lastSyncedAt) : undefined,
  });

  const [showConflictDialog, setShowConflictDialog] = useState(false);
  const [cacheStatus, setCacheStatus] = useState<CacheStatusResponse | null>(null);
  const [cacheStatusError, setCacheStatusError] = useState<string | null>(null);
  const [isCacheStatusLoading, setCacheStatusLoading] = useState(false);
  const [queueItems, setQueueItems] = useState<SyncQueueItem[]>([]);
  const [queueItemsError, setQueueItemsError] = useState<string | null>(null);
  const [isQueueItemsLoading, setQueueItemsLoading] = useState(false);
  const [lastQueuedItemId, setLastQueuedItemId] = useState<number | null>(null);
  const [queueingType, setQueueingType] = useState<string | null>(null);

  const refreshCacheStatus = useCallback(async () => {
    setCacheStatusLoading(true);
    try {
      const status = await offlineApi.getCacheStatus();
      setCacheStatus(status);
      setCacheStatusError(null);
    } catch (error) {
      errorHandler.log('Failed to fetch cache status', error, {
        context: 'useSyncManager.refreshCacheStatus',
      });
      setCacheStatusError('キャッシュ状態の取得に失敗しました');
    } finally {
      setCacheStatusLoading(false);
    }
  }, []);

  const refreshQueueItems = useCallback(async () => {
    setQueueItemsLoading(true);
    try {
      const items = await offlineApi.listSyncQueueItems({
        limit: SYNC_QUEUE_HISTORY_LIMIT,
      });
      setQueueItems(items);
      setQueueItemsError(null);
    } catch (error) {
      errorHandler.log('Failed to fetch sync queue items', error, {
        context: 'useSyncManager.refreshQueueItems',
      });
      setQueueItemsError('再送キューの取得に失敗しました');
    } finally {
      setQueueItemsLoading(false);
    }
  }, []);

  const enqueueSyncRequest = useCallback(
    async (cacheType: string) => {
      setQueueingType(cacheType);
      try {
        const queueId = await offlineApi.addToSyncQueue({
          action_type: 'manual_sync_refresh',
          payload: {
            cacheType,
            requestedAt: new Date().toISOString(),
            source: 'sync_status_indicator',
            userPubkey: currentUser?.npub ?? 'unknown',
          },
          priority: 5,
        });
        toast.success(`再送キューに追加しました (#${queueId})`);
        setLastQueuedItemId(queueId);
        await refreshCacheStatus();
        await refreshQueueItems();
        return queueId;
      } catch (error) {
        errorHandler.log('Failed to enqueue sync request', error, {
          context: 'useSyncManager.enqueueSyncRequest',
        });
        toast.error('再送キューへの追加に失敗しました');
        throw error;
      } finally {
        setQueueingType((current) => (current === cacheType ? null : current));
      }
    },
    [currentUser?.npub, refreshCacheStatus, refreshQueueItems],
  );

  const persistSyncStatuses = useCallback(
    async (result: SyncResult) => {
      const tasks: Promise<unknown>[] = [];
      const syncedIds = new Set(result.syncedActions.map((action) => action.localId));

      for (const action of result.syncedActions) {
        const context = extractEntityContext(action);
        if (!context) {
          continue;
        }
        tasks.push(
          offlineApi
            .updateSyncStatus(context.entityType, context.entityId, 'fully_synced', null)
            .catch((error) => {
              errorHandler.log('Failed to update sync status (fully_synced)', error, {
                context: 'useSyncManager.persistSyncStatuses',
              });
            }),
        );
      }

      for (const action of result.failedActions) {
        const context = extractEntityContext(action);
        if (!context) {
          continue;
        }
        tasks.push(
          offlineApi
            .updateSyncStatus(context.entityType, context.entityId, 'failed', null)
            .catch((error) => {
              errorHandler.log('Failed to update sync status (failed)', error, {
                context: 'useSyncManager.persistSyncStatuses',
              });
            }),
        );
      }

      for (const conflict of result.conflicts) {
        const context = extractEntityContext(conflict.localAction);
        if (!context) {
          continue;
        }

        if (syncedIds.has(conflict.localAction.localId)) {
          // 競合が解消され既に同期済みのものはスキップ
          continue;
        }

        tasks.push(
          offlineApi
            .updateSyncStatus(
              context.entityType,
              context.entityId,
              'conflict',
              JSON.stringify({
                conflictType: conflict.conflictType,
                resolution: conflict.resolution ?? null,
              }),
            )
            .catch((error) => {
              errorHandler.log('Failed to update sync status (conflict)', error, {
                context: 'useSyncManager.persistSyncStatuses',
              });
            }),
        );
      }

      if (tasks.length > 0) {
        await Promise.allSettled(tasks);
      }

      await refreshCacheMetadata();
      await refreshCacheStatus();
    },
    [refreshCacheMetadata, refreshCacheStatus],
  );

  /**
   * 手動同期トリガー
   */
  const triggerManualSync = useCallback(async () => {
    if (!isOnline) {
      toast.error('オフラインのため同期できません');
      return;
    }

    if (syncStatus.isSyncing) {
      toast.warning('同期処理が既に実行中です');
      return;
    }

    if (pendingActions.length === 0) {
      toast.info('同期するアクションがありません');
      return;
    }

    setSyncStatus((prev) => ({
      ...prev,
      isSyncing: true,
      progress: 0,
      totalItems: pendingActions.length,
      syncedItems: 0,
      conflicts: [],
      error: undefined,
    }));

    try {
      // 差分同期を実行
      const result = await syncEngine.performDifferentialSync(pendingActions);

      // 同期結果を処理
      await processSyncResult(result);

      // 成功したアクションをクリア
      if (result.syncedActions.length > 0) {
        for (const action of result.syncedActions) {
          clearSyncError(action.localId);
        }
      }

      setSyncStatus((prev) => ({
        ...prev,
        isSyncing: false,
        progress: 100,
        syncedItems: result.syncedActions.length,
        conflicts: result.conflicts,
        lastSyncTime: new Date(),
      }));

      // 競合がある場合は通知
      if (result.conflicts.length > 0) {
        toast.warning(`${result.conflicts.length}件の競合が検出されました`);
      } else {
        toast.success(`${result.syncedActions.length}件のアクションを同期しました`);
      }

      await persistSyncStatuses(result);
    } catch (error) {
      errorHandler.log('同期エラー', error, {
        context: 'useSyncManager.triggerManualSync',
      });
      setSyncStatus((prev) => ({
        ...prev,
        isSyncing: false,
        error: error instanceof Error ? error.message : '同期に失敗しました',
      }));
      toast.error('同期に失敗しました');
      await refreshCacheMetadata();
      await refreshCacheStatus();
    }
  }, [
    isOnline,
    syncStatus.isSyncing,
    pendingActions,
    clearSyncError,
    persistSyncStatuses,
    refreshCacheMetadata,
    refreshCacheStatus,
  ]);

  /**
   * 同期結果を処理
   */
  const processSyncResult = async (result: SyncResult) => {
    // 失敗したアクションにエラーをマーク
    for (const failedAction of result.failedActions) {
      setSyncError(failedAction.localId, '同期に失敗しました');
    }

    // 競合の手動解決が必要な場合
    const manualConflicts = result.conflicts.filter((c) => c.resolution === 'manual');
    if (manualConflicts.length > 0) {
      // 競合解決UIのためにステートを更新
      setSyncStatus((prev) => ({
        ...prev,
        conflicts: manualConflicts,
      }));
      setShowConflictDialog(true);
    }

    // Zustandストアの同期処理を呼び出し
    if (currentUser?.npub) {
      await syncPendingActions(currentUser.npub);
    }
  };

  /**
   * 競合を手動で解決
   */
  const resolveConflict = useCallback(
    async (conflict: SyncConflict, resolution: 'local' | 'remote' | 'merge') => {
      conflict.resolution = resolution;

      try {
        if (resolution === 'local') {
          // ローカルのアクションを適用
          await syncEngine['applyAction'](conflict.localAction);
          toast.success('ローカルの変更を適用しました');
        } else if (resolution === 'remote' && conflict.remoteAction) {
          // リモートのアクションを適用
          await syncEngine['applyAction'](conflict.remoteAction);
          toast.success('リモートの変更を適用しました');
        } else if (resolution === 'merge' && conflict.mergedData) {
          // マージしたデータを適用
          const mergedAction = {
            ...conflict.localAction,
            data: conflict.mergedData,
            timestamp: Date.now(),
          };
          await syncEngine['applyAction'](mergedAction);
          toast.success('変更をマージしました');
        }

        // 競合リストから削除
        setSyncStatus((prev) => ({
          ...prev,
          conflicts: prev.conflicts.filter((c) => c !== conflict),
        }));
      } catch (error) {
        errorHandler.log('競合解決エラー', error, {
          context: 'useSyncManager.resolveConflict',
        });
        toast.error('競合の解決に失敗しました');
      }
    },
    [],
  );

  /**
   * 同期進捗の更新
   */
  const updateProgress = useCallback((syncedItems: number, totalItems: number) => {
    const progress = totalItems > 0 ? (syncedItems / totalItems) * 100 : 0;

    setSyncStatus((prev) => ({
      ...prev,
      progress,
      syncedItems,
      totalItems,
    }));
  }, []);

  /**
   * 自動同期の設定
   */
  useEffect(() => {
    if (!isOnline || pendingActions.length === 0) {
      return;
    }

    // オンライン復帰時に自動同期
    const syncTimer = setTimeout(() => {
      triggerManualSync();
    }, 2000); // 2秒後に同期

    return () => clearTimeout(syncTimer);
  }, [isOnline]); // triggerManualSyncは依存配列に含めない（無限ループ防止）

  /**
   * 定期同期の設定
   */
  useEffect(() => {
    if (!isOnline) {
      return;
    }

    // 5分ごとに自動同期
    const interval = setInterval(
      () => {
        if (pendingActions.length > 0 && !syncStatus.isSyncing) {
          triggerManualSync();
        }
      },
      5 * 60 * 1000,
    );

    return () => clearInterval(interval);
  }, [isOnline, pendingActions.length]); // triggerManualSyncとsyncStatus.isSyncingは依存配列に含めない

  /**
   * キャッシュステータスの自動更新
   */
  useEffect(() => {
    void refreshCacheStatus();
    const interval = setInterval(() => {
      void refreshCacheStatus();
    }, 60 * 1000);
    return () => clearInterval(interval);
  }, [refreshCacheStatus]);

  useEffect(() => {
    void refreshCacheStatus();
  }, [pendingActions.length, refreshCacheStatus]);

  useEffect(() => {
    void refreshQueueItems();
    const interval = setInterval(() => {
      void refreshQueueItems();
    }, 60 * 1000);
    return () => clearInterval(interval);
  }, [refreshQueueItems]);

  useEffect(() => {
    void refreshQueueItems();
  }, [pendingActions.length, refreshQueueItems]);

  return {
    syncStatus,
    triggerManualSync,
    resolveConflict,
    updateProgress,
    pendingActionsCount: pendingActions.length,
    isOnline,
    showConflictDialog,
    setShowConflictDialog,
    cacheStatus,
    cacheStatusError,
    isCacheStatusLoading,
    refreshCacheStatus,
    queueItems,
    queueItemsError,
    isQueueItemsLoading,
    refreshQueueItems,
    lastQueuedItemId,
    queueingType,
    enqueueSyncRequest,
  };
}
