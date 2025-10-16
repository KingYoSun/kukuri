import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useSyncManager } from './useSyncManager';
import { useOfflineStore } from '@/stores/offlineStore';
import { useAuthStore } from '@/stores/authStore';
import { syncEngine } from '@/lib/sync/syncEngine';
import type { OfflineAction } from '@/types/offline';
import { OfflineActionType } from '@/types/offline';

// モックの設定
vi.mock('@/stores/offlineStore');
vi.mock('@/stores/authStore');
vi.mock('@/lib/sync/syncEngine');
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
    info: vi.fn(),
  },
}));

describe('useSyncManager', () => {
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

  const defaultOfflineState = {
    pendingActions: mockPendingActions,
    isOnline: true,
    lastSyncedAt: undefined,
    syncPendingActions: vi.fn().mockResolvedValue(undefined),
    clearPendingActions: vi.fn(),
    setSyncError: vi.fn(),
    clearSyncError: vi.fn(),
  };

  const defaultAuthState = {
    currentAccount: {
      npub: 'npub123',
      name: 'Test User',
    },
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useOfflineStore).mockReturnValue(defaultOfflineState as any);
    vi.mocked(useAuthStore).mockReturnValue(defaultAuthState as any);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('初期状態', () => {
    it('正しい初期状態を持つ', () => {
      const { result } = renderHook(() => useSyncManager());

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
      vi.mocked(useOfflineStore).mockReturnValue({
        ...defaultOfflineState,
        isOnline: false,
      } as any);

      const { result } = renderHook(() => useSyncManager());

      await act(async () => {
        await result.current.triggerManualSync();
      });

      expect(toast.error).toHaveBeenCalledWith('オフラインのため同期できません');
      expect(syncEngine.performDifferentialSync).not.toHaveBeenCalled();
    });

    it('保留中のアクションがない場合は同期をスキップ', async () => {
      const { toast } = await import('sonner');
      vi.mocked(useOfflineStore).mockReturnValue({
        ...defaultOfflineState,
        pendingActions: [],
      } as any);

      const { result } = renderHook(() => useSyncManager());

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

      const { result } = renderHook(() => useSyncManager());

      await act(async () => {
        await result.current.triggerManualSync();
      });

      expect(syncEngine.performDifferentialSync).toHaveBeenCalledWith(mockPendingActions);
      expect(toast.success).toHaveBeenCalledWith('1件のアクションを同期しました');
      expect(result.current.syncStatus.syncedItems).toBe(1);
      expect(result.current.syncStatus.progress).toBe(100);
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

      const { result } = renderHook(() => useSyncManager());

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

      const { result } = renderHook(() => useSyncManager());

      await act(async () => {
        await result.current.triggerManualSync();
      });

      expect(toast.error).toHaveBeenCalledWith('同期に失敗しました');
      expect(result.current.syncStatus.error).toBe(errorMessage);
      expect(result.current.syncStatus.isSyncing).toBe(false);
    });

    it('同期中は重複実行を防ぐ', async () => {
      const { toast } = await import('sonner');
      const { result } = renderHook(() => useSyncManager());

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

      const { result } = renderHook(() => useSyncManager());

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

      const { result } = renderHook(() => useSyncManager());

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

      const { result } = renderHook(() => useSyncManager());

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

      const { result } = renderHook(() => useSyncManager());

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
    it('同期進捗を更新できる', () => {
      const { result } = renderHook(() => useSyncManager());

      act(() => {
        result.current.updateProgress(5, 10);
      });

      expect(result.current.syncStatus.progress).toBe(50);
      expect(result.current.syncStatus.syncedItems).toBe(5);
      expect(result.current.syncStatus.totalItems).toBe(10);
    });

    it('0アイテムの場合は進捗0%', () => {
      const { result } = renderHook(() => useSyncManager());

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
      vi.mocked(useOfflineStore).mockReturnValue({
        ...defaultOfflineState,
        isOnline: false,
        pendingActions: mockPendingActions,
      } as any);

      const { rerender } = renderHook(() => useSyncManager());

      // オンラインに変更（pendingActionsも保持）
      vi.mocked(useOfflineStore).mockReturnValue({
        ...defaultOfflineState,
        isOnline: true,
        pendingActions: mockPendingActions,
      } as any);

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
});
