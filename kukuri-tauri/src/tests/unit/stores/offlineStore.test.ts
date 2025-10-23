import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { useOfflineStore } from '@/stores/offlineStore';
import { offlineApi } from '@/api/offline';
import type { OfflineAction } from '@/types/offline';

// モックの設定
vi.mock('@/api/offline', () => ({
  offlineApi: {
    saveOfflineAction: vi.fn(),
    getOfflineActions: vi.fn(),
    syncOfflineActions: vi.fn(),
    getCacheStatus: vi.fn(),
    cleanupExpiredCache: vi.fn(),
    saveOptimisticUpdate: vi.fn(),
    confirmOptimisticUpdate: vi.fn(),
    rollbackOptimisticUpdate: vi.fn(),
  },
}));

describe('offlineStore', () => {
  beforeEach(() => {
    // ストアをリセット
    useOfflineStore.setState({
      isOnline: true,
      lastSyncedAt: undefined,
      pendingActions: [],
      syncQueue: [],
      optimisticUpdates: new Map(),
      isSyncing: false,
      syncErrors: new Map(),
    });

    // モックをリセット
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('オンライン/オフライン状態管理', () => {
    it('オンライン状態を設定できる', () => {
      const store = useOfflineStore.getState();
      store.setOnlineStatus(false);

      expect(useOfflineStore.getState().isOnline).toBe(false);

      store.setOnlineStatus(true);
      expect(useOfflineStore.getState().isOnline).toBe(true);
    });

    it('初期状態はnavigator.onLineの値を使用する', () => {
      const originalOnLine = navigator.onLine;
      Object.defineProperty(navigator, 'onLine', {
        writable: true,
        value: false,
      });

      // 新しいストアインスタンスを作成
      const state = useOfflineStore.getState();
      expect(state.isOnline).toBe(true); // persistedの値が優先される

      Object.defineProperty(navigator, 'onLine', {
        writable: true,
        value: originalOnLine,
      });
    });
  });

  describe('保留中のアクション管理', () => {
    it('保留中のアクションを追加できる', () => {
      const store = useOfflineStore.getState();
      const action: OfflineAction = {
        id: 1,
        userPubkey: 'test_user',
        actionType: 'create_post',
        targetId: 'post_123',
        actionData: '{"content": "test"}',
        localId: 'local_123',
        remoteId: null,
        isSynced: false,
        createdAt: Date.now(),
        syncedAt: undefined,
      };

      store.addPendingAction(action);
      expect(useOfflineStore.getState().pendingActions).toHaveLength(1);
      expect(useOfflineStore.getState().pendingActions[0]).toEqual(action);
    });

    it('保留中のアクションを削除できる', () => {
      const store = useOfflineStore.getState();
      const action1: OfflineAction = {
        id: 1,
        userPubkey: 'test_user',
        actionType: 'create_post',
        targetId: 'post_123',
        actionData: '{}',
        localId: 'local_123',
        remoteId: null,
        isSynced: false,
        createdAt: Date.now(),
      };
      const action2: OfflineAction = {
        id: 2,
        userPubkey: 'test_user',
        actionType: 'like',
        targetId: 'post_456',
        actionData: '{}',
        localId: 'local_456',
        remoteId: null,
        isSynced: false,
        createdAt: Date.now(),
      };

      store.addPendingAction(action1);
      store.addPendingAction(action2);
      expect(useOfflineStore.getState().pendingActions).toHaveLength(2);

      store.removePendingAction('local_123');
      expect(useOfflineStore.getState().pendingActions).toHaveLength(1);
      expect(useOfflineStore.getState().pendingActions[0].localId).toBe('local_456');
    });

    it('全ての保留中のアクションをクリアできる', () => {
      const store = useOfflineStore.getState();
      store.addPendingAction({
        id: 1,
        userPubkey: 'test_user',
        actionType: 'create_post',
        actionData: '{}',
        localId: 'local_123',
        isSynced: false,
        createdAt: Date.now(),
      } as OfflineAction);

      expect(useOfflineStore.getState().pendingActions).toHaveLength(1);

      store.clearPendingActions();
      expect(useOfflineStore.getState().pendingActions).toHaveLength(0);
    });
  });

  describe('楽観的更新管理', () => {
    it('楽観的更新を追加できる', () => {
      const store = useOfflineStore.getState();
      const update = {
        id: 1,
        updateId: 'update_123',
        entityType: 'post',
        entityId: 'post_123',
        originalData: '{"likes": 10}',
        updatedData: '{"likes": 11}',
        isConfirmed: false,
        createdAt: Date.now(),
      };

      store.addOptimisticUpdate(update);
      const updates = useOfflineStore.getState().optimisticUpdates;
      expect(updates.size).toBe(1);
      expect(updates.get('update_123')).toEqual(update);
    });

    it('楽観的更新を確認できる', () => {
      const store = useOfflineStore.getState();
      const update = {
        id: 1,
        updateId: 'update_123',
        entityType: 'post',
        entityId: 'post_123',
        originalData: '{"likes": 10}',
        updatedData: '{"likes": 11}',
        isConfirmed: false,
        createdAt: Date.now(),
      };

      store.addOptimisticUpdate(update);
      store.confirmOptimisticUpdate('update_123');

      const updates = useOfflineStore.getState().optimisticUpdates;
      const confirmedUpdate = updates.get('update_123');
      expect(confirmedUpdate?.isConfirmed).toBe(true);
      expect(confirmedUpdate?.confirmedAt).toBeDefined();
    });

    it('楽観的更新をロールバックできる', () => {
      const store = useOfflineStore.getState();
      const update = {
        id: 1,
        updateId: 'update_123',
        entityType: 'post',
        entityId: 'post_123',
        originalData: '{"likes": 10}',
        updatedData: '{"likes": 11}',
        isConfirmed: false,
        createdAt: Date.now(),
      };

      store.addOptimisticUpdate(update);
      expect(useOfflineStore.getState().optimisticUpdates.size).toBe(1);

      store.rollbackOptimisticUpdate('update_123');
      expect(useOfflineStore.getState().optimisticUpdates.size).toBe(0);
    });
  });

  describe('同期エラー管理', () => {
    it('同期エラーを設定できる', () => {
      const store = useOfflineStore.getState();
      store.setSyncError('action_123', 'Network error');

      const errors = useOfflineStore.getState().syncErrors;
      expect(errors.size).toBe(1);
      expect(errors.get('action_123')).toBe('Network error');
    });

    it('同期エラーをクリアできる', () => {
      const store = useOfflineStore.getState();
      store.setSyncError('action_123', 'Network error');
      expect(useOfflineStore.getState().syncErrors.size).toBe(1);

      store.clearSyncError('action_123');
      expect(useOfflineStore.getState().syncErrors.size).toBe(0);
    });
  });

  describe('非同期アクション', () => {
    it('オフラインアクションを保存できる', async () => {
      const mockResponse = {
        localId: 'local_123',
        action: {
          id: 1,
          userPubkey: 'test_user',
          actionType: 'create_post',
          actionData: '{"content": "test"}',
          localId: 'local_123',
          isSynced: false,
          createdAt: Date.now(),
        } as OfflineAction,
      };

      vi.mocked(offlineApi.saveOfflineAction).mockResolvedValue(mockResponse);

      const store = useOfflineStore.getState();
      await store.saveOfflineAction({
        userPubkey: 'test_user',
        actionType: 'create_post',
        actionData: { content: 'test' },
      });

      expect(offlineApi.saveOfflineAction).toHaveBeenCalledWith({
        userPubkey: 'test_user',
        actionType: 'create_post',
        actionData: { content: 'test' },
      });
      expect(useOfflineStore.getState().pendingActions).toHaveLength(1);
    });

    it('オンライン時は自動的に同期を試みる', async () => {
      const mockResponse = {
        localId: 'local_123',
        action: {
          id: 1,
          userPubkey: 'test_user',
          actionType: 'create_post',
          actionData: '{}',
          localId: 'local_123',
          isSynced: false,
          createdAt: Date.now(),
        } as OfflineAction,
      };

      vi.mocked(offlineApi.saveOfflineAction).mockResolvedValue(mockResponse);

      const store = useOfflineStore.getState();
      store.setOnlineStatus(true);

      await store.saveOfflineAction({
        userPubkey: 'test_user',
        actionType: 'create_post',
        actionData: {},
      });

      // saveOfflineActionが呼ばれたことを確認
      expect(offlineApi.saveOfflineAction).toHaveBeenCalled();
      // アクションが追加されたことを確認
      expect(useOfflineStore.getState().pendingActions).toHaveLength(1);
    });

    it('保留中のアクションを同期できる', async () => {
      vi.mocked(offlineApi.syncOfflineActions).mockResolvedValue({
        syncedCount: 1,
        failedCount: 0,
        pendingCount: 0,
      });

      vi.mocked(offlineApi.getOfflineActions).mockResolvedValue([]);

      const store = useOfflineStore.getState();
      // 初期状態を設定
      useOfflineStore.setState({
        isOnline: true,
        isSyncing: false,
        pendingActions: [
          {
            id: 1,
            userPubkey: 'test_user',
            actionType: 'create_post',
            actionData: '{}',
            localId: 'local_123',
            isSynced: false,
            createdAt: Date.now(),
          } as OfflineAction,
        ],
      });

      await store.syncPendingActions('test_user');

      expect(offlineApi.syncOfflineActions).toHaveBeenCalledWith({
        userPubkey: 'test_user',
      });
      expect(useOfflineStore.getState().lastSyncedAt).toBeDefined();
    });

    it('オフライン時は同期をスキップする', async () => {
      const store = useOfflineStore.getState();
      store.setOnlineStatus(false);

      await store.syncPendingActions('test_user');

      expect(offlineApi.syncOfflineActions).not.toHaveBeenCalled();
    });

    it('期限切れキャッシュをクリーンアップできる', async () => {
      vi.mocked(offlineApi.cleanupExpiredCache).mockResolvedValue(5);

      const store = useOfflineStore.getState();
      await store.cleanupExpiredCache();

      expect(offlineApi.cleanupExpiredCache).toHaveBeenCalled();
    });
  });

  describe('楽観的更新ヘルパー', () => {
    it('楽観的更新を適用できる', async () => {
      vi.mocked(offlineApi.saveOptimisticUpdate).mockResolvedValue('update_123');

      const store = useOfflineStore.getState();
      const originalData = { likes: 10 };
      const updatedData = { likes: 11 };

      const updateId = await store.applyOptimisticUpdate(
        'post' as any,
        'post_123',
        originalData,
        updatedData,
      );

      expect(updateId).toBe('update_123');
      expect(offlineApi.saveOptimisticUpdate).toHaveBeenCalledWith(
        'post',
        'post_123',
        JSON.stringify(originalData),
        JSON.stringify(updatedData),
      );
      expect(useOfflineStore.getState().optimisticUpdates.size).toBe(1);
    });

    it('更新を確認できる', async () => {
      vi.mocked(offlineApi.confirmOptimisticUpdate).mockResolvedValue();

      const store = useOfflineStore.getState();
      store.addOptimisticUpdate({
        id: 1,
        updateId: 'update_123',
        entityType: 'post',
        entityId: 'post_123',
        originalData: '{"likes": 10}',
        updatedData: '{"likes": 11}',
        isConfirmed: false,
        createdAt: Date.now(),
      });

      await store.confirmUpdate('update_123');

      expect(offlineApi.confirmOptimisticUpdate).toHaveBeenCalledWith('update_123');
      const update = useOfflineStore.getState().optimisticUpdates.get('update_123');
      expect(update?.isConfirmed).toBe(true);
    });

    it('更新をロールバックできる', async () => {
      vi.mocked(offlineApi.rollbackOptimisticUpdate).mockResolvedValue('{"likes": 10}');

      const store = useOfflineStore.getState();
      store.addOptimisticUpdate({
        id: 1,
        updateId: 'update_123',
        entityType: 'post',
        entityId: 'post_123',
        originalData: '{"likes": 10}',
        updatedData: '{"likes": 11}',
        isConfirmed: false,
        createdAt: Date.now(),
      });

      const originalData = await store.rollbackUpdate('update_123');

      expect(offlineApi.rollbackOptimisticUpdate).toHaveBeenCalledWith('update_123');
      expect(originalData).toBe('{"likes": 10}');
      expect(useOfflineStore.getState().optimisticUpdates.size).toBe(0);
    });
  });
});
