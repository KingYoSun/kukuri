import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { SyncStatusIndicator } from './SyncStatusIndicator';
import { useSyncManager } from '@/hooks/useSyncManager';
import type { SyncStatus } from '@/hooks/useSyncManager';
import type { SyncConflict } from '@/lib/sync/syncEngine';
import { OfflineActionType } from '@/types/offline';

// モックの設定
vi.mock('@/hooks/useSyncManager');

describe('SyncStatusIndicator', () => {
  const mockTriggerManualSync = vi.fn();
  const mockResolveConflict = vi.fn();
  const mockUpdateProgress = vi.fn();

  const defaultSyncStatus: SyncStatus = {
    isSyncing: false,
    progress: 0,
    totalItems: 0,
    syncedItems: 0,
    conflicts: [],
    lastSyncTime: undefined,
    error: undefined,
  };

  const defaultManagerState = {
    syncStatus: defaultSyncStatus,
    triggerManualSync: mockTriggerManualSync,
    resolveConflict: mockResolveConflict,
    updateProgress: mockUpdateProgress,
    pendingActionsCount: 0,
    isOnline: true,
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSyncManager).mockReturnValue(defaultManagerState);
  });

  describe('状態表示', () => {
    it('オフライン状態を表示', () => {
      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        isOnline: false,
      });

      render(<SyncStatusIndicator />);
      
      expect(screen.getByText('オフライン')).toBeInTheDocument();
    });

    it('同期済み状態を表示', () => {
      render(<SyncStatusIndicator />);
      
      expect(screen.getByText('同期済み')).toBeInTheDocument();
    });

    it('未同期アクションがある場合の表示', () => {
      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        pendingActionsCount: 5,
      });

      render(<SyncStatusIndicator />);
      
      expect(screen.getByText('未同期: 5件')).toBeInTheDocument();
      expect(screen.getByText('5')).toHaveClass('ml-1'); // バッジ
    });

    it('同期中の状態を表示', () => {
      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        syncStatus: {
          ...defaultSyncStatus,
          isSyncing: true,
          syncedItems: 3,
          totalItems: 10,
        },
      });

      render(<SyncStatusIndicator />);
      
      expect(screen.getByText('同期中... (3/10)')).toBeInTheDocument();
    });

    it('競合がある場合の表示', () => {
      const mockConflict: SyncConflict = {
        localAction: {
          id: 1,
          localId: 'local_1',
          userPubkey: 'user123',
          actionType: OfflineActionType.CREATE_POST,
          actionData: {},
          createdAt: new Date().toISOString(),
          isSynced: false,
        },
        conflictType: 'timestamp',
      };

      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        syncStatus: {
          ...defaultSyncStatus,
          conflicts: [mockConflict],
        },
      });

      render(<SyncStatusIndicator />);
      
      expect(screen.getByText('競合: 1件')).toBeInTheDocument();
    });

    it('同期エラーの表示', () => {
      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        syncStatus: {
          ...defaultSyncStatus,
          error: 'Network error',
        },
      });

      render(<SyncStatusIndicator />);
      
      expect(screen.getByText('同期エラー')).toBeInTheDocument();
    });
  });

  describe('ポップオーバー', () => {
    it('クリックでポップオーバーを開く', async () => {
      render(<SyncStatusIndicator />);
      
      const button = screen.getByRole('button');
      fireEvent.click(button);
      
      await waitFor(() => {
        expect(screen.getByText('接続状態')).toBeInTheDocument();
      });
    });

    it('同期進捗を表示', async () => {
      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        syncStatus: {
          ...defaultSyncStatus,
          isSyncing: true,
          progress: 50,
          syncedItems: 5,
          totalItems: 10,
        },
      });

      render(<SyncStatusIndicator />);
      
      const button = screen.getByRole('button');
      fireEvent.click(button);
      
      await waitFor(() => {
        expect(screen.getByText('同期進捗')).toBeInTheDocument();
        expect(screen.getByText('5 / 10 件を同期中')).toBeInTheDocument();
      });
    });

    it('未同期アクション数を表示', async () => {
      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        pendingActionsCount: 3,
      });

      render(<SyncStatusIndicator />);
      
      const button = screen.getByRole('button');
      fireEvent.click(button);
      
      await waitFor(() => {
        expect(screen.getByText('未同期アクション')).toBeInTheDocument();
        expect(screen.getByText('3件のアクションが同期待ちです')).toBeInTheDocument();
      });
    });

    it('競合リストを表示', async () => {
      const mockConflicts: SyncConflict[] = [
        {
          localAction: {
            id: 1,
            localId: 'local_1',
            userPubkey: 'user123',
            actionType: OfflineActionType.CREATE_POST,
            actionData: {},
            createdAt: new Date().toISOString(),
            isSynced: false,
          },
          conflictType: 'timestamp',
        },
        {
          localAction: {
            id: 2,
            localId: 'local_2',
            userPubkey: 'user123',
            actionType: OfflineActionType.LIKE_POST,
            actionData: {},
            createdAt: new Date().toISOString(),
            isSynced: false,
          },
          conflictType: 'version',
        },
      ];

      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        syncStatus: {
          ...defaultSyncStatus,
          conflicts: mockConflicts,
        },
      });

      render(<SyncStatusIndicator />);
      
      const button = screen.getByRole('button');
      fireEvent.click(button);
      
      await waitFor(() => {
        expect(screen.getByText('競合検出')).toBeInTheDocument();
        expect(screen.getByText('CREATE_POST')).toBeInTheDocument();
        expect(screen.getByText('LIKE_POST')).toBeInTheDocument();
      });
    });

    it('最終同期時刻を表示', async () => {
      const lastSyncTime = new Date('2024-01-01T12:00:00Z');
      
      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        syncStatus: {
          ...defaultSyncStatus,
          lastSyncTime,
        },
      });

      render(<SyncStatusIndicator />);
      
      const button = screen.getByRole('button');
      fireEvent.click(button);
      
      await waitFor(() => {
        expect(screen.getByText('最終同期')).toBeInTheDocument();
      });
    });
  });

  describe('手動同期', () => {
    it('同期ボタンをクリックで同期を実行', async () => {
      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        pendingActionsCount: 1,
      });

      render(<SyncStatusIndicator />);
      
      const button = screen.getByRole('button');
      fireEvent.click(button);
      
      await waitFor(() => {
        const syncButton = screen.getByText('今すぐ同期');
        fireEvent.click(syncButton);
      });
      
      expect(mockTriggerManualSync).toHaveBeenCalled();
    });

    it('オフライン時は同期ボタンを無効化', async () => {
      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        isOnline: false,
        pendingActionsCount: 1,
      });

      render(<SyncStatusIndicator />);
      
      const button = screen.getByRole('button');
      fireEvent.click(button);
      
      await waitFor(() => {
        const syncButton = screen.getByText('今すぐ同期');
        expect(syncButton).toBeDisabled();
      });
    });

    it('同期中は同期ボタンを無効化', async () => {
      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        syncStatus: {
          ...defaultSyncStatus,
          isSyncing: true,
        },
        pendingActionsCount: 1,
      });

      render(<SyncStatusIndicator />);
      
      const button = screen.getByRole('button');
      fireEvent.click(button);
      
      await waitFor(() => {
        const syncButton = screen.getByText('今すぐ同期');
        expect(syncButton).toBeDisabled();
      });
    });

    it('未同期アクションがない場合は同期ボタンを無効化', async () => {
      render(<SyncStatusIndicator />);
      
      const button = screen.getByRole('button');
      fireEvent.click(button);
      
      await waitFor(() => {
        const syncButton = screen.getByText('今すぐ同期');
        expect(syncButton).toBeDisabled();
      });
    });
  });

  describe('競合解決ダイアログ', () => {
    it('競合をクリックでダイアログを開く', async () => {
      const mockConflict: SyncConflict = {
        localAction: {
          id: 1,
          localId: 'local_1',
          userPubkey: 'user123',
          actionType: OfflineActionType.CREATE_POST,
          actionData: {},
          createdAt: '2024-01-01T00:00:00Z',
          isSynced: false,
        },
        conflictType: 'timestamp',
      };

      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        syncStatus: {
          ...defaultSyncStatus,
          conflicts: [mockConflict],
        },
      });

      render(<SyncStatusIndicator />);
      
      // ポップオーバーを開く
      const button = screen.getByRole('button');
      fireEvent.click(button);
      
      // 競合をクリック
      await waitFor(() => {
        const conflictItem = screen.getByText('CREATE_POST');
        fireEvent.click(conflictItem);
      });
      
      // ダイアログが表示される
      await waitFor(() => {
        expect(screen.getByText('競合の解決')).toBeInTheDocument();
        expect(screen.getByText('ローカルの変更')).toBeInTheDocument();
      });
    });

    it('ローカルの変更を適用', async () => {
      const mockConflict: SyncConflict = {
        localAction: {
          id: 1,
          localId: 'local_1',
          userPubkey: 'user123',
          actionType: OfflineActionType.CREATE_POST,
          actionData: {},
          createdAt: '2024-01-01T00:00:00Z',
          isSynced: false,
        },
        conflictType: 'timestamp',
      };

      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        syncStatus: {
          ...defaultSyncStatus,
          conflicts: [mockConflict],
        },
      });

      render(<SyncStatusIndicator />);
      
      // ポップオーバーを開く
      const button = screen.getByRole('button');
      fireEvent.click(button);
      
      // 競合をクリック
      await waitFor(() => {
        const conflictItem = screen.getByText('CREATE_POST');
        fireEvent.click(conflictItem);
      });
      
      // ローカルを適用ボタンをクリック
      await waitFor(() => {
        const applyLocalButton = screen.getByText('ローカルを適用');
        fireEvent.click(applyLocalButton);
      });
      
      expect(mockResolveConflict).toHaveBeenCalledWith(mockConflict, 'local');
    });

    it('リモートの変更を適用', async () => {
      const mockConflict: SyncConflict = {
        localAction: {
          id: 1,
          localId: 'local_1',
          userPubkey: 'user123',
          actionType: OfflineActionType.CREATE_POST,
          actionData: {},
          createdAt: '2024-01-01T00:00:00Z',
          isSynced: false,
        },
        remoteAction: {
          id: 2,
          localId: 'remote_1',
          userPubkey: 'user123',
          actionType: OfflineActionType.CREATE_POST,
          actionData: {},
          createdAt: '2024-01-02T00:00:00Z',
          isSynced: true,
        },
        conflictType: 'timestamp',
      };

      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        syncStatus: {
          ...defaultSyncStatus,
          conflicts: [mockConflict],
        },
      });

      render(<SyncStatusIndicator />);
      
      // ポップオーバーを開く
      const button = screen.getByRole('button');
      fireEvent.click(button);
      
      // 競合をクリック
      await waitFor(() => {
        const conflictItem = screen.getByText('CREATE_POST');
        fireEvent.click(conflictItem);
      });
      
      // リモートを適用ボタンをクリック
      await waitFor(() => {
        const applyRemoteButton = screen.getByText('リモートを適用');
        fireEvent.click(applyRemoteButton);
      });
      
      expect(mockResolveConflict).toHaveBeenCalledWith(mockConflict, 'remote');
    });
  });
});