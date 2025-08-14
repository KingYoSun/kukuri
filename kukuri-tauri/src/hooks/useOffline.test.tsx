import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useOffline, useOptimisticUpdate } from './useOffline';
import { useOfflineStore } from '@/stores/offlineStore';
import { useAuthStore } from '@/stores/authStore';
import { OfflineActionType, EntityType } from '@/types/offline';

// モックの設定
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
  },
}));

vi.mock('@/stores/authStore', () => ({
  useAuthStore: vi.fn(),
}));

vi.mock('@/stores/offlineStore', () => ({
  useOfflineStore: vi.fn(),
}));

describe('useOffline', () => {
  const mockSaveOfflineAction = vi.fn();
  const mockSyncPendingActions = vi.fn();
  const mockLoadPendingActions = vi.fn();
  
  const defaultOfflineState = {
    isOnline: true,
    pendingActions: [],
    isSyncing: false,
    lastSyncedAt: undefined,
    saveOfflineAction: mockSaveOfflineAction,
    syncPendingActions: mockSyncPendingActions,
    loadPendingActions: mockLoadPendingActions,
  };

  const defaultAuthState = {
    currentUser: {
      npub: 'test_npub',
      displayName: 'Test User',
    },
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useOfflineStore).mockReturnValue(defaultOfflineState as any);
    vi.mocked(useAuthStore).mockReturnValue(defaultAuthState as any);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('初期化', () => {
    it('マウント時に保留中のアクションを読み込む', () => {
      renderHook(() => useOffline());
      
      expect(mockLoadPendingActions).toHaveBeenCalledWith('test_npub');
    });

    it('ユーザーが未ログインの場合はアクションを読み込まない', () => {
      vi.mocked(useAuthStore).mockReturnValue({
        currentUser: null,
      } as any);

      renderHook(() => useOffline());
      
      expect(mockLoadPendingActions).not.toHaveBeenCalled();
    });
  });

  describe('オンライン/オフライン状態の監視', () => {
    it('オフライン時に通知を表示する', async () => {
      const { toast } = await import('sonner');
      vi.mocked(useOfflineStore).mockReturnValue({
        ...defaultOfflineState,
        isOnline: false,
      } as any);

      renderHook(() => useOffline());

      expect(toast.info).toHaveBeenCalledWith(
        'オフラインモードです。変更は後で同期されます。'
      );
    });

    it('オンラインになった時に同期を開始する', async () => {
      const { toast } = await import('sonner');
      vi.mocked(useOfflineStore).mockReturnValue({
        ...defaultOfflineState,
        pendingActions: [{ id: 1 }],
      } as any);

      renderHook(() => useOffline());

      // オンラインイベントを発火
      window.dispatchEvent(new Event('online'));

      await waitFor(() => {
        expect(toast.success).toHaveBeenCalledWith(
          'オンラインになりました。データを同期しています...'
        );
        expect(mockSyncPendingActions).toHaveBeenCalledWith('test_npub');
      });
    });
  });

  describe('saveAction', () => {
    it('アクションを保存できる', async () => {
      const { result } = renderHook(() => useOffline());

      await act(async () => {
        await result.current.saveAction(
          OfflineActionType.CREATE_POST,
          'post_123',
          { content: 'Test post' }
        );
      });

      expect(mockSaveOfflineAction).toHaveBeenCalledWith({
        userPubkey: 'test_npub',
        actionType: OfflineActionType.CREATE_POST,
        entityType: EntityType.POST,
        entityId: 'post_123',
        data: JSON.stringify({ content: 'Test post' }),
      });
    });

    it('オフライン時に通知を表示する', async () => {
      const { toast } = await import('sonner');
      vi.mocked(useOfflineStore).mockReturnValue({
        ...defaultOfflineState,
        isOnline: false,
      } as any);

      const { result } = renderHook(() => useOffline());

      await act(async () => {
        await result.current.saveAction(OfflineActionType.LIKE, 'post_123');
      });

      expect(toast.info).toHaveBeenCalledWith(
        'アクションが保存されました。オンライン時に同期されます。'
      );
    });

    it('未ログイン時はエラーをスローする', async () => {
      vi.mocked(useAuthStore).mockReturnValue({
        currentUser: null,
      } as any);

      const { result } = renderHook(() => useOffline());

      await expect(
        result.current.saveAction(OfflineActionType.LIKE, 'post_123')
      ).rejects.toThrow('User not authenticated');
    });
  });

  describe('triggerSync', () => {
    it('手動で同期をトリガーできる', async () => {
      const { toast } = await import('sonner');
      vi.mocked(useOfflineStore).mockReturnValue({
        ...defaultOfflineState,
        pendingActions: [{ id: 1 }],
      } as any);

      const { result } = renderHook(() => useOffline());

      await act(async () => {
        await result.current.triggerSync();
      });

      expect(mockSyncPendingActions).toHaveBeenCalledWith('test_npub');
      expect(toast.success).toHaveBeenCalledWith('同期が完了しました');
    });

    it('オフライン時は同期できない', async () => {
      const { toast } = await import('sonner');
      vi.mocked(useOfflineStore).mockReturnValue({
        ...defaultOfflineState,
        isOnline: false,
        pendingActions: [{ id: 1 }],
      } as any);

      const { result } = renderHook(() => useOffline());

      await act(async () => {
        await result.current.triggerSync();
      });

      expect(mockSyncPendingActions).not.toHaveBeenCalled();
      expect(toast.warning).toHaveBeenCalledWith('オフラインのため同期できません');
    });

    it('同期中は再度同期しない', async () => {
      const { toast } = await import('sonner');
      vi.mocked(useOfflineStore).mockReturnValue({
        ...defaultOfflineState,
        isSyncing: true,
        pendingActions: [{ id: 1 }],
      } as any);

      const { result } = renderHook(() => useOffline());

      await act(async () => {
        await result.current.triggerSync();
      });

      expect(mockSyncPendingActions).not.toHaveBeenCalled();
      expect(toast.info).toHaveBeenCalledWith('すでに同期中です');
    });

    it('保留中のアクションがない場合は通知する', async () => {
      const { toast } = await import('sonner');
      
      const { result } = renderHook(() => useOffline());

      await act(async () => {
        await result.current.triggerSync();
      });

      expect(mockSyncPendingActions).not.toHaveBeenCalled();
      expect(toast.info).toHaveBeenCalledWith('同期するアクションはありません');
    });

    it('同期エラー時にエラーメッセージを表示する', async () => {
      const { toast } = await import('sonner');
      mockSyncPendingActions.mockRejectedValue(new Error('Sync failed'));
      vi.mocked(useOfflineStore).mockReturnValue({
        ...defaultOfflineState,
        pendingActions: [{ id: 1 }],
      } as any);

      const { result } = renderHook(() => useOffline());

      await act(async () => {
        await result.current.triggerSync();
      });

      expect(toast.error).toHaveBeenCalledWith('同期に失敗しました');
    });
  });

  describe('返り値', () => {
    it('正しい値を返す', () => {
      vi.mocked(useOfflineStore).mockReturnValue({
        ...defaultOfflineState,
        isOnline: false,
        isSyncing: true,
        pendingActions: [{ id: 1 }, { id: 2 }],
        lastSyncedAt: 1234567890,
      } as any);

      const { result } = renderHook(() => useOffline());

      expect(result.current.isOnline).toBe(false);
      expect(result.current.isSyncing).toBe(true);
      expect(result.current.pendingCount).toBe(2);
      expect(result.current.lastSyncedAt).toBe(1234567890);
      expect(typeof result.current.saveAction).toBe('function');
      expect(typeof result.current.triggerSync).toBe('function');
    });
  });
});

describe('useOptimisticUpdate', () => {
  const mockApplyOptimisticUpdate = vi.fn();
  const mockConfirmUpdate = vi.fn();
  const mockRollbackUpdate = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useOfflineStore).mockReturnValue({
      applyOptimisticUpdate: mockApplyOptimisticUpdate,
      confirmUpdate: mockConfirmUpdate,
      rollbackUpdate: mockRollbackUpdate,
    } as any);
  });

  it('楽観的更新を適用できる', async () => {
    mockApplyOptimisticUpdate.mockResolvedValue('update_123');
    
    const { result } = renderHook(() => useOptimisticUpdate());

    const originalData = { likes: 10 };
    const updatedData = { likes: 11 };
    const onSuccess = vi.fn().mockResolvedValue(undefined);

    await act(async () => {
      const updateId = await result.current.apply(
        'post',
        'post_123',
        originalData,
        updatedData,
        onSuccess
      );
      expect(updateId).toBe('update_123');
    });

    expect(mockApplyOptimisticUpdate).toHaveBeenCalledWith(
      'post',
      'post_123',
      originalData,
      updatedData
    );
    expect(onSuccess).toHaveBeenCalled();
    expect(mockConfirmUpdate).toHaveBeenCalledWith('update_123');
  });

  it('エラー時にロールバックする', async () => {
    mockApplyOptimisticUpdate.mockResolvedValue('update_123');
    
    const { result } = renderHook(() => useOptimisticUpdate());

    const originalData = { likes: 10 };
    const updatedData = { likes: 11 };
    const onSuccess = vi.fn().mockRejectedValue(new Error('API error'));
    const onError = vi.fn();

    await act(async () => {
      try {
        await result.current.apply(
          'post',
          'post_123',
          originalData,
          updatedData,
          onSuccess,
          onError
        );
      } catch {
        // エラーが期待される
      }
    });

    expect(mockApplyOptimisticUpdate).toHaveBeenCalled();
    expect(onSuccess).toHaveBeenCalled();
    expect(mockRollbackUpdate).toHaveBeenCalledWith('update_123');
    expect(onError).toHaveBeenCalledWith(expect.any(Error));
  });

  it('onSuccessがない場合でも動作する', async () => {
    mockApplyOptimisticUpdate.mockResolvedValue('update_123');
    
    const { result } = renderHook(() => useOptimisticUpdate());

    const originalData = { likes: 10 };
    const updatedData = { likes: 11 };

    await act(async () => {
      const updateId = await result.current.apply(
        'post',
        'post_123',
        originalData,
        updatedData
      );
      expect(updateId).toBe('update_123');
    });

    expect(mockApplyOptimisticUpdate).toHaveBeenCalled();
    expect(mockConfirmUpdate).not.toHaveBeenCalled();
  });
});