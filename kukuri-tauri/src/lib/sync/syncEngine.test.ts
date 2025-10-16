import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { syncEngine } from './syncEngine';
import type { OfflineAction, SyncConflict } from '@/types/offline';
import { OfflineActionType } from '@/types/offline';

// モックの設定
vi.mock('@/lib/api/tauri');
vi.mock('@/lib/api/p2p');
vi.mock('@/lib/api/nostr');

describe('SyncEngine', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('performDifferentialSync', () => {
    it('空のアクションリストで同期を実行できる', async () => {
      const result = await syncEngine.performDifferentialSync([]);

      expect(result).toEqual({
        syncedActions: [],
        conflicts: [],
        failedActions: [],
        totalProcessed: 0,
      });
    });

    it('競合のないアクションを同期できる', async () => {
      const mockAction: OfflineAction = {
        id: 1,
        localId: 'local_123',
        userPubkey: 'user123',
        actionType: OfflineActionType.CREATE_POST,
        actionData: {
          content: 'Test post',
          topicId: 'topic1',
        },
        createdAt: new Date().toISOString(),
        isSynced: false,
      };

      vi.spyOn(syncEngine as any, 'detectConflict').mockResolvedValue(null);
      vi.spyOn(syncEngine as any, 'applyAction').mockResolvedValue(undefined);

      const result = await syncEngine.performDifferentialSync([mockAction]);

      expect(result.syncedActions).toHaveLength(1);
      expect(result.syncedActions[0]).toEqual(mockAction);
      expect(result.conflicts).toHaveLength(0);
      expect(result.failedActions).toHaveLength(0);
      expect(result.totalProcessed).toBe(1);
    });

    it('競合のあるアクションを検出して解決できる', async () => {
      const mockAction: OfflineAction = {
        id: 1,
        localId: 'local_123',
        userPubkey: 'user123',
        actionType: OfflineActionType.LIKE_POST,
        actionData: {
          postId: 'post1',
        },
        createdAt: '2024-01-01T00:00:00Z',
        isSynced: false,
      };

      const mockConflict: SyncConflict = {
        localAction: mockAction,
        conflictType: 'timestamp',
      };

      vi.spyOn(syncEngine as any, 'detectConflict').mockResolvedValue(mockConflict);
      vi.spyOn(syncEngine as any, 'resolveConflict').mockResolvedValue({
        ...mockConflict,
        resolution: 'local',
      });
      vi.spyOn(syncEngine as any, 'applyAction').mockResolvedValue(undefined);

      const result = await syncEngine.performDifferentialSync([mockAction]);

      expect(result.syncedActions).toHaveLength(1);
      expect(result.conflicts).toHaveLength(1);
      expect(result.conflicts[0].resolution).toBe('local');
      expect(result.failedActions).toHaveLength(0);
    });

    it('複数のトピックのアクションを並列で同期できる', async () => {
      const actions: OfflineAction[] = [
        {
          id: 1,
          localId: 'local_1',
          userPubkey: 'user123',
          actionType: OfflineActionType.CREATE_POST,
          actionData: { content: 'Post 1', topicId: 'topic1' },
          createdAt: new Date().toISOString(),
          isSynced: false,
        },
        {
          id: 2,
          localId: 'local_2',
          userPubkey: 'user123',
          actionType: OfflineActionType.CREATE_POST,
          actionData: { content: 'Post 2', topicId: 'topic2' },
          createdAt: new Date().toISOString(),
          isSynced: false,
        },
      ];

      vi.spyOn(syncEngine as any, 'detectConflict').mockResolvedValue(null);
      vi.spyOn(syncEngine as any, 'applyAction').mockResolvedValue(undefined);

      const result = await syncEngine.performDifferentialSync(actions);

      expect(result.syncedActions).toHaveLength(2);
      expect(result.totalProcessed).toBe(2);
    });

    it('同期中は重複実行を防ぐ', async () => {
      const mockAction: OfflineAction = {
        id: 1,
        localId: 'local_123',
        userPubkey: 'user123',
        actionType: OfflineActionType.CREATE_POST,
        actionData: { content: 'Test' },
        createdAt: new Date().toISOString(),
        isSynced: false,
      };

      // 長時間かかる同期をシミュレート
      vi.spyOn(syncEngine as any, 'applyAction').mockImplementation(
        () => new Promise((resolve) => setTimeout(resolve, 100)),
      );

      // 最初の同期を開始
      const firstSync = syncEngine.performDifferentialSync([mockAction]);

      // 2回目の同期を試みる
      await expect(syncEngine.performDifferentialSync([mockAction])).rejects.toThrow(
        '同期処理が既に実行中です',
      );

      // 最初の同期が完了するのを待つ
      await firstSync;
    });
  });

  describe('detectConflict', () => {
    it('エンティティの最終更新時刻がない場合は競合なし', async () => {
      const action: OfflineAction = {
        id: 1,
        localId: 'local_123',
        userPubkey: 'user123',
        actionType: OfflineActionType.CREATE_POST,
        actionData: {},
        createdAt: new Date().toISOString(),
        isSynced: false,
      };

      vi.spyOn(syncEngine as any, 'getEntityLastModified').mockResolvedValue(null);

      const conflict = await syncEngine['detectConflict'](action);
      expect(conflict).toBeNull();
    });

    it('アクションが新しい場合は競合なし', async () => {
      const now = new Date();
      const action: OfflineAction = {
        id: 1,
        localId: 'local_123',
        userPubkey: 'user123',
        actionType: OfflineActionType.LIKE_POST,
        actionData: {
          entityType: 'post',
          entityId: 'post1',
        },
        createdAt: now.toISOString(),
        isSynced: false,
      };

      const oldDate = new Date(now.getTime() - 1000);
      vi.spyOn(syncEngine as any, 'getEntityLastModified').mockResolvedValue(oldDate.toISOString());

      const conflict = await syncEngine['detectConflict'](action);
      expect(conflict).toBeNull();
    });

    it('エンティティが新しい場合は競合を検出', async () => {
      const oldDate = new Date('2024-01-01');
      const action: OfflineAction = {
        id: 1,
        localId: 'local_123',
        userPubkey: 'user123',
        actionType: OfflineActionType.LIKE_POST,
        actionData: {
          entityType: 'post',
          entityId: 'post1',
        },
        createdAt: oldDate.toISOString(),
        isSynced: false,
      };

      const newDate = new Date('2024-01-02');
      vi.spyOn(syncEngine as any, 'getEntityLastModified').mockResolvedValue(newDate.toISOString());

      const conflict = await syncEngine['detectConflict'](action);
      expect(conflict).not.toBeNull();
      expect(conflict?.conflictType).toBe('timestamp');
    });
  });

  describe('resolveConflict', () => {
    it('Last-Write-Wins戦略で競合を解決', () => {
      const localAction: OfflineAction = {
        id: 1,
        localId: 'local_123',
        userPubkey: 'user123',
        actionType: OfflineActionType.CREATE_POST,
        actionData: {},
        createdAt: '2024-01-02T00:00:00Z',
        isSynced: false,
      };

      const remoteAction: OfflineAction = {
        id: 2,
        localId: 'remote_123',
        userPubkey: 'user123',
        actionType: OfflineActionType.CREATE_POST,
        actionData: {},
        createdAt: '2024-01-01T00:00:00Z',
        isSynced: true,
      };

      const conflict: SyncConflict = {
        localAction,
        remoteAction,
        conflictType: 'timestamp',
      };

      const resolved = syncEngine['resolveLWW'](conflict);
      expect(resolved.resolution).toBe('local');
    });

    it('トピック参加アクションはLWWで解決', async () => {
      const localAction: OfflineAction = {
        id: 1,
        localId: 'local_123',
        userPubkey: 'user123',
        actionType: OfflineActionType.JOIN_TOPIC,
        actionData: { topicId: 'topic1' },
        createdAt: '2024-01-01T00:00:00Z',
        isSynced: false,
      };

      const conflict: SyncConflict = {
        localAction,
        conflictType: 'merge',
      };

      const resolved = await syncEngine['resolveConflict'](conflict);
      expect(resolved.resolution).toBe('local');
    });

    it('投稿作成はローカルを優先', async () => {
      const localAction: OfflineAction = {
        id: 1,
        localId: 'local_123',
        userPubkey: 'user123',
        actionType: OfflineActionType.CREATE_POST,
        actionData: { content: 'Test' },
        createdAt: '2024-01-01T00:00:00Z',
        isSynced: false,
      };

      const conflict: SyncConflict = {
        localAction,
        conflictType: 'merge',
      };

      const resolved = await syncEngine['resolveConflict'](conflict);
      expect(resolved.resolution).toBe('local');
    });
  });

  describe('generateDiffPatches', () => {
    it('追加されたフィールドを検出', () => {
      const oldData = { a: 1 };
      const newData = { a: 1, b: 2 };

      const patches = syncEngine.generateDiffPatches(oldData, newData);

      expect(patches).toHaveLength(1);
      expect(patches[0]).toEqual({
        type: 'add',
        path: 'b',
        value: 2,
      });
    });

    it('変更されたフィールドを検出', () => {
      const oldData = { a: 1, b: 2 };
      const newData = { a: 1, b: 3 };

      const patches = syncEngine.generateDiffPatches(oldData, newData);

      expect(patches).toHaveLength(1);
      expect(patches[0]).toEqual({
        type: 'modify',
        path: 'b',
        value: 3,
        oldValue: 2,
      });
    });

    it('削除されたフィールドを検出', () => {
      const oldData = { a: 1, b: 2 };
      const newData = { a: 1 };

      const patches = syncEngine.generateDiffPatches(oldData, newData);

      expect(patches).toHaveLength(1);
      expect(patches[0]).toEqual({
        type: 'delete',
        path: 'b',
        oldValue: 2,
      });
    });

    it('ネストされたオブジェクトの変更を検出', () => {
      const oldData = { a: { b: 1 } };
      const newData = { a: { b: 2 } };

      const patches = syncEngine.generateDiffPatches(oldData, newData);

      expect(patches).toHaveLength(1);
      expect(patches[0].path).toBe('a');
    });
  });

  describe('applyDiffPatches', () => {
    it('追加パッチを適用', () => {
      const data = { a: 1 };
      const patches = [{ type: 'add' as const, path: 'b', value: 2 }];

      const result = syncEngine.applyDiffPatches(data, patches);

      expect(result).toEqual({ a: 1, b: 2 });
    });

    it('変更パッチを適用', () => {
      const data = { a: 1, b: 2 };
      const patches = [{ type: 'modify' as const, path: 'b', value: 3 }];

      const result = syncEngine.applyDiffPatches(data, patches);

      expect(result).toEqual({ a: 1, b: 3 });
    });

    it('削除パッチを適用', () => {
      const data = { a: 1, b: 2 };
      const patches = [{ type: 'delete' as const, path: 'b', oldValue: 2 }];

      const result = syncEngine.applyDiffPatches(data, patches);

      expect(result).toEqual({ a: 1 });
    });

    it('複数のパッチを順番に適用', () => {
      const data = { a: 1 };
      const patches = [
        { type: 'add' as const, path: 'b', value: 2 },
        { type: 'modify' as const, path: 'a', value: 10 },
        { type: 'add' as const, path: 'c', value: 3 },
      ];

      const result = syncEngine.applyDiffPatches(data, patches);

      expect(result).toEqual({ a: 10, b: 2, c: 3 });
    });
  });
});
