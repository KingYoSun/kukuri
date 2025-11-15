import { useState, useCallback, useEffect, useRef } from 'react';
import { useOfflineStore } from '@/stores/offlineStore';
import { useAuthStore } from '@/stores/authStore';
import { syncEngine, type SyncResult, type SyncConflict } from '@/lib/sync/syncEngine';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';
import { offlineApi } from '@/api/offline';
import type {
  CacheStatusResponse,
  OfflineAction,
  OfflineRetryMetrics,
  SyncQueueItem,
} from '@/types/offline';
import { OfflineActionType } from '@/types/offline';
import {
  enqueueOfflineSyncJob,
  OFFLINE_SYNC_CHANNEL,
  registerOfflineSyncWorker,
} from '@/serviceWorker/offlineSyncBridge';

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
const WORKER_SCHEDULE_COOLDOWN_MS = 30 * 1000;

type OfflineSyncWorkerJob = {
  jobId: string;
  userPubkey?: string;
  reason?: string;
  requestedAt?: string;
  retryCount?: number;
  maxRetries?: number;
  retryDelayMs?: number;
};

type RetryJobContext = {
  jobId: string;
  reason?: string;
  userPubkey?: string;
  retryCount: number;
  maxRetries: number;
  retryDelayMs?: number;
  requestedAt?: string;
  trigger: 'manual' | 'worker';
};

type ScheduledRetryPayload = {
  jobId: string;
  retryCount: number;
  maxRetries: number;
  retryDelayMs: number;
  nextRunAt: string;
};

type ScheduledRetryInfo = ScheduledRetryPayload | null;

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
    removePendingAction,
    setSyncError,
    clearSyncError,
    refreshCacheMetadata,
    updateLastSyncedAt,
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
  const [retryMetrics, setRetryMetrics] = useState<OfflineRetryMetrics | null>(null);
  const [retryMetricsError, setRetryMetricsError] = useState<string | null>(null);
  const [isRetryMetricsLoading, setRetryMetricsLoading] = useState(false);
  const [scheduledRetry, setScheduledRetry] = useState<ScheduledRetryInfo>(null);
  const workerChannelRef = useRef<BroadcastChannel | null>(null);
  const workerScheduleRef = useRef(0);
  const pendingActionsRef = useRef(pendingActions.length);
  const isOnlineRef = useRef(isOnline);
  const currentJobContextRef = useRef<RetryJobContext | null>(null);
  const jobStartRef = useRef<number | null>(null);

  useEffect(() => {
    pendingActionsRef.current = pendingActions.length;
  }, [pendingActions.length]);

  useEffect(() => {
    isOnlineRef.current = isOnline;
  }, [isOnline]);

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

  const refreshRetryMetrics = useCallback(async () => {
    setRetryMetricsLoading(true);
    try {
      const metrics = await offlineApi.getOfflineRetryMetrics();
      setRetryMetrics(metrics);
      setRetryMetricsError(null);
    } catch (error) {
      errorHandler.log('Failed to fetch retry metrics', error, {
        context: 'useSyncManager.refreshRetryMetrics',
      });
      setRetryMetricsError('再送メトリクスの取得に失敗しました');
    } finally {
      setRetryMetricsLoading(false);
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

  const processSyncResult = useCallback(
    async (result: SyncResult) => {
      let hasSynced = false;

      if (result.syncedActions.length > 0) {
        for (const action of result.syncedActions) {
          removePendingAction(action.localId);
        }
        hasSynced = true;
      }

      if (result.failedActions.length > 0) {
        for (const action of result.failedActions) {
          setSyncError(action.localId, '蜷梧悄縺ｫ螟ｱ謨励＠縺ｾ縺励◆');
        }
      }

      if (result.conflicts.length > 0) {
        for (const conflict of result.conflicts) {
          setSyncError(conflict.localAction.localId, '遶ｶ蜷医′蜿門ｾ励＠縺ｾ縺励◆');
        }
      }

      if (hasSynced) {
        updateLastSyncedAt();
      }
    },
    [removePendingAction, setSyncError, updateLastSyncedAt],
  );

  const beginRetryContext = useCallback(
    (options?: { job?: OfflineSyncWorkerJob; trigger?: 'manual' | 'worker'; reason?: string }) => {
      const job = options?.job;
      const trigger = options?.trigger ?? (job ? 'worker' : 'manual');
      const context: RetryJobContext = {
        jobId: job?.jobId ?? `manual-sync-${Date.now()}`,
        reason: job?.reason ?? options?.reason ?? 'manual_sync',
        userPubkey: job?.userPubkey ?? currentUser?.npub,
        retryCount: job?.retryCount ?? 0,
        maxRetries: job?.maxRetries ?? 1,
        retryDelayMs: job?.retryDelayMs,
        requestedAt: job?.requestedAt,
        trigger,
      };
      currentJobContextRef.current = context;
      jobStartRef.current = Date.now();
      return context;
    },
    [currentUser?.npub],
  );

  const submitRetryOutcome = useCallback(
    async (status: 'success' | 'failure', counts: { success: number; failure: number }) => {
      const context = currentJobContextRef.current;
      const durationMs = jobStartRef.current ? Date.now() - jobStartRef.current : undefined;
      currentJobContextRef.current = null;
      jobStartRef.current = null;
      if (!context) {
        return;
      }
      try {
        const metrics = await offlineApi.recordOfflineRetryOutcome({
          jobId: context.jobId,
          status,
          jobReason: context.reason,
          trigger: context.trigger,
          userPubkey: context.userPubkey,
          retryCount: context.retryCount,
          maxRetries: context.maxRetries,
          backoffMs: context.retryDelayMs,
          durationMs,
          successCount: counts.success,
          failureCount: counts.failure,
          timestampMs: context.requestedAt ? Date.parse(context.requestedAt) : undefined,
        });
        setRetryMetrics(metrics);
        setRetryMetricsError(null);
      } catch (error) {
        errorHandler.log('Failed to record retry metrics', error, {
          context: 'useSyncManager.recordRetryOutcome',
          metadata: { jobId: context.jobId },
        });
        setRetryMetricsError('再送メトリクスの記録に失敗しました');
      } finally {
        setScheduledRetry(null);
      }
    },
    [],
  );

  /**
   * 手動同期トリガー
   */
  const triggerManualSync = useCallback(
    async (options?: { job?: OfflineSyncWorkerJob; trigger?: 'manual' | 'worker' }) => {
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

      beginRetryContext(options);

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
        const result = await syncEngine.performDifferentialSync(pendingActions);
        await processSyncResult(result);

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

        if (result.conflicts.length > 0) {
          toast.warning(`${result.conflicts.length}件の競合が検出されました`);
        } else {
          toast.success(`${result.syncedActions.length}件のアクションを同期しました`);
        }

        await persistSyncStatuses(result);
        await submitRetryOutcome(result.failedActions.length > 0 ? 'failure' : 'success', {
          success: result.syncedActions.length,
          failure: result.failedActions.length,
        });
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
        await submitRetryOutcome('failure', {
          success: 0,
          failure: pendingActions.length,
        });
        await refreshCacheMetadata();
        await refreshCacheStatus();
      }
    },
    [
      beginRetryContext,
      clearSyncError,
      isOnline,
      pendingActions,
      persistSyncStatuses,
      processSyncResult,
      refreshCacheMetadata,
      refreshCacheStatus,
      submitRetryOutcome,
      syncStatus.isSyncing,
    ],
  );

  const resolveConflict = useCallback(
    async (conflict: SyncConflict, resolution: 'local' | 'remote' | 'merge') => {
      const applyAction =
        (syncEngine as unknown as { applyAction?: (action: OfflineAction) => Promise<void> })
          .applyAction ?? null;
      if (!applyAction) {
        errorHandler.log('SyncEngine.applyActionUnavailable', null, {
          context: 'useSyncManager.resolveConflict',
        });
        toast.error('遶ｶ蜷医・隗｣豎ｺ縺ｫ螟ｱ謨励＠縺ｾ縺励◆');
        return;
      }

      let actionToApply: OfflineAction | null = null;
      switch (resolution) {
        case 'local':
          actionToApply = conflict.localAction;
          break;
        case 'remote':
          actionToApply = conflict.remoteAction ?? null;
          break;
        case 'merge':
          if (!conflict.mergedData) {
            actionToApply = null;
            break;
          }
          actionToApply = {
            ...conflict.localAction,
            actionData:
              typeof conflict.mergedData === 'string'
                ? conflict.mergedData
                : JSON.stringify(conflict.mergedData),
          };
          break;
        default:
          actionToApply = conflict.localAction;
          break;
      }

      if (!actionToApply) {
        toast.error('遶ｶ蜷医・隗｣豎ｺ縺ｫ螟ｱ謨励＠縺ｾ縺励◆');
        return;
      }

      try {
        await applyAction.call(syncEngine, actionToApply);

        removePendingAction(conflict.localAction.localId);
        clearSyncError(conflict.localAction.localId);
        updateLastSyncedAt();

        setSyncStatus((prev) => ({
          ...prev,
          syncedItems: prev.syncedItems + 1,
          totalItems: Math.max(prev.totalItems, prev.syncedItems + 1),
          conflicts: prev.conflicts.filter(
            (item) => item.localAction.localId !== conflict.localAction.localId,
          ),
        }));

        await persistSyncStatuses({
          syncedActions: [conflict.localAction],
          conflicts: [],
          failedActions: [],
          totalProcessed: 1,
        });

        const successMessage =
          resolution === 'remote'
            ? '繝ｪ繝｢繝ｼ繝医・螟画峩繧帝←逕ｨ縺励∪縺励◆'
            : resolution === 'merge'
              ? '螟画峩繧偵・繝ｼ繧ｸ縺励∪縺励◆'
              : '繝ｭ繝ｼ繧ｫ繝ｫ縺ｮ螟画峩繧帝←逕ｨ縺励∪縺励◆';
        toast.success(successMessage);
      } catch (error) {
        errorHandler.log('Failed to resolve sync conflict', error, {
          context: 'useSyncManager.resolveConflict',
          metadata: { conflictType: conflict.conflictType },
        });
        toast.error('遶ｶ蜷医・隗｣豎ｺ縺ｫ螟ｱ謨励＠縺ｾ縺励◆');
      }
    },
    [clearSyncError, persistSyncStatuses, removePendingAction, updateLastSyncedAt],
  );

  const updateProgress = useCallback((synced: number, total: number) => {
    setSyncStatus((prev) => {
      const normalizedTotal = total > 0 ? total : prev.totalItems;
      const safeTotal = Math.max(normalizedTotal, synced, 0);
      const progress =
        safeTotal > 0 ? Math.max(0, Math.min(100, Math.round((synced / safeTotal) * 100))) : 0;
      return {
        ...prev,
        syncedItems: synced,
        totalItems: safeTotal,
        progress,
      };
    });
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

  useEffect(() => {
    void refreshRetryMetrics();
  }, [refreshRetryMetrics]);

  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }
    void registerOfflineSyncWorker();
  }, []);

  useEffect(() => {
    if (typeof window === 'undefined' || typeof BroadcastChannel === 'undefined') {
      return;
    }
    const channel = new BroadcastChannel(OFFLINE_SYNC_CHANNEL);
    workerChannelRef.current = channel;

    const handleMessage = (
      event: MessageEvent<{
        type?: string;
        payload?: OfflineSyncWorkerJob | ScheduledRetryPayload;
      }>,
    ) => {
      const message = event.data;
      if (!message) {
        return;
      }
      if (message.type === 'offline-sync:scheduled' && message.payload) {
        const payload = message.payload as ScheduledRetryInfo;
        setScheduledRetry(payload);
        return;
      }
      if (message.type !== 'offline-sync:process' || !message.payload) {
        return;
      }
      const job = message.payload as OfflineSyncWorkerJob;

      const buildCompletionPayload = (success: boolean) => ({
        jobId: job.jobId,
        success,
        retryCount: job.retryCount ?? 0,
        maxRetries: job.maxRetries ?? 3,
        retryDelayMs: job.retryDelayMs ?? 0,
      });

      if (job.userPubkey && currentUser?.npub && job.userPubkey !== currentUser.npub) {
        channel.postMessage({
          type: 'offline-sync:complete',
          payload: buildCompletionPayload(true),
        });
        return;
      }

      if (!isOnlineRef.current) {
        channel.postMessage({
          type: 'offline-sync:complete',
          payload: buildCompletionPayload(false),
        });
        return;
      }

      if (pendingActionsRef.current === 0) {
        channel.postMessage({
          type: 'offline-sync:complete',
          payload: buildCompletionPayload(true),
        });
        return;
      }

      setScheduledRetry(null);

      const run = async () => {
        try {
          await triggerManualSync({ job });
          channel.postMessage({
            type: 'offline-sync:complete',
            payload: buildCompletionPayload(true),
          });
        } catch {
          channel.postMessage({
            type: 'offline-sync:complete',
            payload: buildCompletionPayload(false),
          });
        }
      };

      void run();
    };

    channel.addEventListener('message', handleMessage);

    return () => {
      channel.removeEventListener('message', handleMessage);
      channel.close();
      workerChannelRef.current = null;
    };
  }, [currentUser?.npub, triggerManualSync]);

  useEffect(() => {
    if (!currentUser?.npub) {
      return;
    }
    if (!isOnline) {
      return;
    }
    if (pendingActions.length === 0) {
      return;
    }
    const now = Date.now();
    if (now - workerScheduleRef.current < WORKER_SCHEDULE_COOLDOWN_MS) {
      return;
    }
    workerScheduleRef.current = now;

    const schedule = async () => {
      const jobId = await enqueueOfflineSyncJob({
        userPubkey: currentUser.npub,
        reason: 'pending-actions',
      });
      if (!jobId) {
        errorHandler.log('OfflineSync.enqueueFailed', null, {
          context: 'useSyncManager.scheduleWorkerJob',
        });
      }
    };

    void schedule();
  }, [currentUser?.npub, isOnline, pendingActions.length, triggerManualSync]);

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
    retryMetrics,
    retryMetricsError,
    isRetryMetricsLoading,
    refreshRetryMetrics,
    scheduledRetry,
  };
}
