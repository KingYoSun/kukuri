import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useSyncManager } from '@/hooks/useSyncManager';
import { syncEngine } from '@/lib/sync/syncEngine';
import { offlineApi } from '@/api/offline';
import type { OfflineAction } from '@/types/offline';
import { OfflineActionType } from '@/types/offline';
import type { ZustandStoreMock } from '@/tests/utils/zustandTestUtils';
import type { OfflineStoreTestState } from '@/tests/utils/offlineStoreMocks';

type AuthStoreState = {
  currentUser: {
    npub: string;
    name: string;
  } | null;
};

vi.mock('sonner', async () => {
  const { createToastMock } =
    await vi.importActual<typeof import('@/tests/utils/toastMock')>('@/tests/utils/toastMock');
  return { toast: createToastMock() };
});

const mockPendingActions: OfflineAction[] = [
  {
    id: 1,
    localId: 'local_1',
    userPubkey: 'user123',
    actionType: OfflineActionType.CREATE_POST,
    actionData: { content: 'Test post' },
    createdAt: new Date().toISOString(),
    isSynced: false,
  },
];

const createPendingAction = (overrides: Partial<OfflineAction> = {}): OfflineAction => ({
  id: Math.floor(Math.random() * 1000) + 2,
  localId: `local_${Math.random().toString(36).slice(2, 8)}`,
  userPubkey: 'user123',
  actionType: OfflineActionType.CREATE_POST,
  actionData: { content: 'Test' },
  createdAt: Date.now(),
  isSynced: false,
  ...overrides,
});

var authStoreMock: ZustandStoreMock<AuthStoreState>;
var offlineStoreMock: ZustandStoreMock<OfflineStoreTestState>;

vi.mock('@/stores/authStore', async () => {
  const { createZustandStoreMock } = await vi.importActual<
    typeof import('@/tests/utils/zustandTestUtils')
  >('@/tests/utils/zustandTestUtils');

  authStoreMock = createZustandStoreMock<AuthStoreState>(() => ({
    currentUser: {
      npub: 'npub123',
      name: 'Test User',
    },
  }));

  return {
    useAuthStore: authStoreMock.hook,
  };
});

vi.mock('@/stores/offlineStore', async () => {
  const [{ createZustandStoreMock }, { createOfflineStoreTestState }] = await Promise.all([
    vi.importActual<typeof import('@/tests/utils/zustandTestUtils')>(
      '@/tests/utils/zustandTestUtils',
    ),
    vi.importActual<typeof import('@/tests/utils/offlineStoreMocks')>(
      '@/tests/utils/offlineStoreMocks',
    ),
  ]);

  offlineStoreMock = createZustandStoreMock<OfflineStoreTestState>(() =>
    createOfflineStoreTestState(),
  );

  return {
    useOfflineStore: offlineStoreMock.hook,
  };
});

// モックの設定
vi.mock('@/lib/sync/syncEngine');
vi.mock('@/api/offline', () => ({
  offlineApi: {
    updateSyncStatus: vi.fn().mockResolvedValue(undefined),
    updateCacheMetadata: vi.fn().mockResolvedValue(undefined),
    getCacheStatus: vi.fn().mockResolvedValue({
      total_items: 0,
      stale_items: 0,
      cache_types: [],
    }),
    addToSyncQueue: vi.fn().mockResolvedValue(1),
    listSyncQueueItems: vi.fn().mockResolvedValue([]),
    getOfflineRetryMetrics: vi.fn().mockResolvedValue({
      totalSuccess: 0,
      totalFailure: 0,
      consecutiveFailure: 0,
      lastSuccessMs: null,
      lastFailureMs: null,
      lastOutcome: null,
      lastJobId: null,
      lastJobReason: null,
      lastTrigger: null,
      lastUserPubkey: null,
      lastRetryCount: null,
      lastMaxRetries: null,
      lastBackoffMs: null,
      lastDurationMs: null,
      lastSuccessCount: null,
      lastFailureCount: null,
      lastTimestampMs: null,
    }),
    recordOfflineRetryOutcome: vi.fn().mockResolvedValue({
      totalSuccess: 1,
      totalFailure: 0,
      consecutiveFailure: 0,
      lastSuccessMs: Date.now(),
      lastFailureMs: null,
      lastOutcome: 'success',
      lastJobId: 'manual-sync',
      lastJobReason: 'manual_sync',
      lastTrigger: 'manual',
      lastUserPubkey: 'npub123',
      lastRetryCount: 0,
      lastMaxRetries: 1,
      lastBackoffMs: 0,
      lastDurationMs: 1200,
      lastSuccessCount: 1,
      lastFailureCount: 0,
      lastTimestampMs: Date.now(),
    }),
  },
}));
vi.mock('@/serviceWorker/offlineSyncBridge', () => ({
  registerOfflineSyncWorker: vi.fn().mockResolvedValue(null),
  enqueueOfflineSyncJob: vi.fn().mockResolvedValue('job-1'),
  OFFLINE_SYNC_CHANNEL: 'offline-sync',
}));

describe('useSyncManager', () => {
  const setOfflineStoreState = (overrides?: Partial<OfflineStoreTestState>) => {
    offlineStoreMock.apply({
      pendingActions: mockPendingActions,
      ...overrides,
    });
  };

  const setAuthStoreState = (overrides?: Partial<AuthStoreState>) => {
    authStoreMock.apply(overrides);
  };

  beforeEach(() => {
    vi.clearAllMocks();
    setOfflineStoreState();
    setAuthStoreState();
  });

  const renderManagerHook = async (options?: { skipClear?: boolean }) => {
    const utils = renderHook(() => useSyncManager());
    await waitFor(() => {
      expect(offlineApi.getCacheStatus).toHaveBeenCalled();
      expect(offlineApi.listSyncQueueItems).toHaveBeenCalled();
      expect(offlineApi.getOfflineRetryMetrics).toHaveBeenCalled();
    });
    if (!options?.skipClear) {
      vi.mocked(offlineApi.getCacheStatus).mockClear();
      vi.mocked(offlineApi.listSyncQueueItems).mockClear();
      vi.mocked(offlineApi.getOfflineRetryMetrics).mockClear();
    }
    return utils;
  };

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('初期状態', () => {
    it('正しい初期状態を持つ', async () => {
      const { result } = await renderManagerHook();

      expect(result.current.syncStatus.isSyncing).toBe(false);
      expect(result.current.syncStatus.progress).toBe(0);
      expect(result.current.syncStatus.totalItems).toBe(0);
      expect(result.current.syncStatus.syncedItems).toBe(0);
      expect(result.current.syncStatus.conflicts).toEqual([]);
      expect(result.current.pendingActionsCount).toBe(1);
      expect(result.current.isOnline).toBe(true);
    });
  });

  describe('triggerManualSync', () => {
    it('オフライン時は同期をスキップ', async () => {
      const { toast } = await import('sonner');
      setOfflineStoreState({ isOnline: false });

      const { result } = await renderManagerHook();

      await act(async () => {
        await result.current.triggerManualSync();
      });

      expect(toast.error).toHaveBeenCalledWith('オフラインのため同期できません');
      expect(syncEngine.performDifferentialSync).not.toHaveBeenCalled();
    });

    it('保留中のアクションがない場合は同期をスキップ', async () => {
      const { toast } = await import('sonner');
      setOfflineStoreState({ pendingActions: [] });

      const { result } = await renderManagerHook();

      await act(async () => {
        await result.current.triggerManualSync();
      });

      expect(toast.info).toHaveBeenCalledWith('同期するアクションがありません');
      expect(syncEngine.performDifferentialSync).not.toHaveBeenCalled();
    });

    it('同期を正常に実行できる', async () => {
      const { toast } = await import('sonner');
      const mockSyncResult = {
        syncedActions: mockPendingActions,
        conflicts: [],
        failedActions: [],
        totalProcessed: 1,
      };

      vi.mocked(syncEngine.performDifferentialSync).mockResolvedValue(mockSyncResult);

      const { result } = await renderManagerHook();

      await act(async () => {
        await result.current.triggerManualSync();
      });

      expect(syncEngine.performDifferentialSync).toHaveBeenCalledWith(mockPendingActions);
      expect(toast.success).toHaveBeenCalledWith('1件のアクションを同期しました');
      expect(result.current.syncStatus.syncedItems).toBe(1);
      expect(result.current.syncStatus.progress).toBe(100);
      expect(offlineApi.recordOfflineRetryOutcome).toHaveBeenCalledWith(
        expect.objectContaining({
          status: 'success',
          successCount: 1,
          failureCount: 0,
        }),
      );
    });

    it('競合がある場合は警告を表示', async () => {
      const { toast } = await import('sonner');
      const mockConflict = {
        localAction: mockPendingActions[0],
        conflictType: 'timestamp' as const,
        resolution: 'local' as const,
      };

      const mockSyncResult = {
        syncedActions: mockPendingActions,
        conflicts: [mockConflict],
        failedActions: [],
        totalProcessed: 1,
      };

      vi.mocked(syncEngine.performDifferentialSync).mockResolvedValue(mockSyncResult);

      const { result } = await renderManagerHook();

      await act(async () => {
        await result.current.triggerManualSync();
      });

      expect(toast.warning).toHaveBeenCalledWith('1件の競合が検出されました');
      expect(result.current.syncStatus.conflicts).toHaveLength(1);
    });

    it('同期エラーを処理できる', async () => {
      const { toast } = await import('sonner');
      const errorMessage = 'Network error';
      vi.mocked(syncEngine.performDifferentialSync).mockRejectedValue(new Error(errorMessage));

      const { result } = await renderManagerHook();

      await act(async () => {
        await result.current.triggerManualSync();
      });

      expect(toast.error).toHaveBeenCalledWith('同期に失敗しました');
      expect(result.current.syncStatus.error).toBe(errorMessage);
      expect(result.current.syncStatus.isSyncing).toBe(false);
      expect(offlineApi.recordOfflineRetryOutcome).toHaveBeenCalledWith(
        expect.objectContaining({
          status: 'failure',
          failureCount: mockPendingActions.length,
        }),
      );
    });

    it('同期中は重複実行を防ぐ', async () => {
      const { toast } = await import('sonner');
      const { result } = await renderManagerHook();

      // 同期中の状態をシミュレート
      act(() => {
        result.current.syncStatus.isSyncing = true;
      });

      await act(async () => {
        await result.current.triggerManualSync();
      });

      expect(toast.warning).toHaveBeenCalledWith('同期処理が既に実行中です');
      expect(syncEngine.performDifferentialSync).not.toHaveBeenCalled();
    });
  });

  describe('resolveConflict', () => {
    it('ローカルの変更を適用できる', async () => {
      const { toast } = await import('sonner');
      const mockConflict = {
        localAction: mockPendingActions[0],
        conflictType: 'timestamp' as const,
      };

      const { result } = await renderManagerHook();

      // 競合を追加
      act(() => {
        result.current.syncStatus.conflicts = [mockConflict];
      });

      await act(async () => {
        await result.current.resolveConflict(mockConflict, 'local');
      });

      expect(toast.success).toHaveBeenCalledWith('ローカルの変更を適用しました');
      expect(result.current.syncStatus.conflicts).toHaveLength(0);
    });

    it('リモートの変更を適用できる', async () => {
      const { toast } = await import('sonner');
      const mockConflict = {
        localAction: mockPendingActions[0],
        remoteAction: {
          ...mockPendingActions[0],
          localId: 'remote_1',
        },
        conflictType: 'timestamp' as const,
      };

      const { result } = await renderManagerHook();

      // 競合を追加
      act(() => {
        result.current.syncStatus.conflicts = [mockConflict];
      });

      await act(async () => {
        await result.current.resolveConflict(mockConflict, 'remote');
      });

      expect(toast.success).toHaveBeenCalledWith('リモートの変更を適用しました');
      expect(result.current.syncStatus.conflicts).toHaveLength(0);
    });

    it('マージした変更を適用できる', async () => {
      const { toast } = await import('sonner');
      const mockConflict = {
        localAction: mockPendingActions[0],
        conflictType: 'merge' as const,
        mergedData: { content: 'Merged content' },
      };

      const { result } = await renderManagerHook();

      // 競合を追加
      act(() => {
        result.current.syncStatus.conflicts = [mockConflict];
      });

      await act(async () => {
        await result.current.resolveConflict(mockConflict, 'merge');
      });

      expect(toast.success).toHaveBeenCalledWith('変更をマージしました');
      expect(result.current.syncStatus.conflicts).toHaveLength(0);
    });

    it('競合解決エラーを処理できる', async () => {
      const { toast } = await import('sonner');
      const mockConflict = {
        localAction: mockPendingActions[0],
        conflictType: 'timestamp' as const,
      };

      // applyActionがエラーをスローするようにモック
      vi.spyOn(syncEngine as any, 'applyAction').mockRejectedValue(new Error('Apply failed'));

      const { result } = await renderManagerHook();

      // 競合を追加
      act(() => {
        result.current.syncStatus.conflicts = [mockConflict];
      });

      await act(async () => {
        await result.current.resolveConflict(mockConflict, 'local');
      });

      expect(toast.error).toHaveBeenCalledWith('競合の解決に失敗しました');
      // 競合は削除されない
      expect(result.current.syncStatus.conflicts).toHaveLength(1);
    });
  });

  describe('updateProgress', () => {
    it('同期進捗を更新できる', async () => {
      const { result } = await renderManagerHook();

      act(() => {
        result.current.updateProgress(5, 10);
      });

      expect(result.current.syncStatus.progress).toBe(50);
      expect(result.current.syncStatus.syncedItems).toBe(5);
      expect(result.current.syncStatus.totalItems).toBe(10);
    });

    it('0アイテムの場合は進捗0%', async () => {
      const { result } = await renderManagerHook();

      act(() => {
        result.current.updateProgress(0, 0);
      });

      expect(result.current.syncStatus.progress).toBe(0);
    });
  });

  describe('自動同期', () => {
    beforeEach(() => {
      vi.useFakeTimers();
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    it.skip('オンライン復帰時に自動同期を実行', async () => {
      const mockSyncResult = {
        syncedActions: mockPendingActions,
        conflicts: [],
        failedActions: [],
        totalProcessed: 1,
      };

      vi.mocked(syncEngine.performDifferentialSync).mockResolvedValue(mockSyncResult);

      // 最初はオフライン
      setOfflineStoreState({ isOnline: false, pendingActions: mockPendingActions });

      const { rerender } = await renderManagerHook();

      // オンラインに変更（pendingActionsも保持）
      setOfflineStoreState({ isOnline: true, pendingActions: mockPendingActions });

      rerender();

      // 2秒待つ
      await act(async () => {
        vi.advanceTimersByTime(2000);
      });

      await waitFor(
        () => {
          expect(syncEngine.performDifferentialSync).toHaveBeenCalledWith(mockPendingActions);
        },
        { timeout: 5000 },
      );
    }, 15000);
  });

  describe('再送キュー', () => {
    it('マウント時に再送キューを取得する', async () => {
      vi.mocked(offlineApi.listSyncQueueItems).mockResolvedValue([
        {
          id: 42,
          action_type: 'manual_sync_refresh',
          status: 'pending',
          retry_count: 0,
          max_retries: 3,
          created_at: 1_700_000_000,
          updated_at: 1_700_000_100,
          payload: { cacheType: 'offline_actions' },
        },
      ] as any);

      const { result } = await renderManagerHook();
      expect(result.current.queueItems).toHaveLength(1);
      expect(result.current.queueItems[0].id).toBe(42);
      vi.mocked(offlineApi.listSyncQueueItems).mockResolvedValue([]);
    });

    it('再送キュー追加後に履歴を更新し、IDを記録する', async () => {
      const { result } = await renderManagerHook();

      await act(async () => {
        const queuedId = await result.current.enqueueSyncRequest('sync_queue');
        expect(queuedId).toBe(1);
      });

      expect(offlineApi.listSyncQueueItems).toHaveBeenCalledTimes(1);
      expect(result.current.lastQueuedItemId).toBe(1);
    });
  });

  describe('キャッシュステータス', () => {
    it('マウント時にキャッシュ状態を取得する', async () => {
      await renderManagerHook({ skipClear: true });
      expect(offlineApi.getCacheStatus).toHaveBeenCalled();
      vi.mocked(offlineApi.getCacheStatus).mockClear();
    });

    it('手動キュー追加で API を呼び出す', async () => {
      const { result } = await renderManagerHook();

      await act(async () => {
        await result.current.enqueueSyncRequest('sync_queue');
      });

      expect(offlineApi.addToSyncQueue).toHaveBeenCalledWith(
        expect.objectContaining({
          action_type: 'manual_sync_refresh',
          payload: expect.objectContaining({
            cacheType: 'sync_queue',
            source: 'sync_status_indicator',
          }),
          priority: 5,
        }),
      );
      // enqueue 後にキャッシュステータスを再取得
      expect(offlineApi.getCacheStatus).toHaveBeenCalledTimes(1);
    });
  });

  describe('pendingActionSummary', () => {
    it('カテゴリごとの件数とサンプルを計算する', async () => {
      setOfflineStoreState({
        pendingActions: [
          createPendingAction({
            actionType: OfflineActionType.TOPIC_CREATE,
            targetId: 'topic-1',
          }),
          createPendingAction({
            actionType: OfflineActionType.CREATE_POST,
            targetId: 'post-1',
          }),
          createPendingAction({
            actionType: OfflineActionType.FOLLOW,
            targetId: 'npub-follow',
          }),
          createPendingAction({
            actionType: 'send_direct_message',
            targetId: 'dm-1',
          }),
        ],
      });

      const { result } = await renderManagerHook();
      const summary = result.current.pendingActionSummary;
      expect(summary.total).toBe(4);
      expect(summary.categories).toEqual(
        expect.arrayContaining([
          expect.objectContaining({ category: 'topic', count: 1 }),
          expect.objectContaining({ category: 'post', count: 1 }),
          expect.objectContaining({ category: 'follow', count: 1 }),
          expect.objectContaining({ category: 'dm', count: 1 }),
        ]),
      );
      const dmCategory = summary.categories.find((category) => category.category === 'dm');
      expect(dmCategory?.samples[0]).toMatchObject({ targetId: 'dm-1' });
    });
  });
});
