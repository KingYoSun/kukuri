import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { SyncStatusIndicator } from '@/components/SyncStatusIndicator';
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
  const mockRefreshCacheStatus = vi.fn();
  const mockEnqueueSyncRequest = vi.fn().mockResolvedValue(undefined);

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
    cacheStatus: null,
    cacheStatusError: null,
    isCacheStatusLoading: false,
    refreshCacheStatus: mockRefreshCacheStatus,
    queueItems: [],
    queueItemsError: null,
    isQueueItemsLoading: false,
    refreshQueueItems: vi.fn(),
    lastQueuedItemId: null,
    queueingType: null,
    enqueueSyncRequest: mockEnqueueSyncRequest,
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
        expect(screen.getByText('create_post')).toBeInTheDocument();
        expect(screen.getByText('like_post')).toBeInTheDocument();
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

    it('キャッシュ状態と操作を表示', async () => {
      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        cacheStatus: {
          total_items: 5,
          stale_items: 2,
          cache_types: [
            {
              cache_type: 'sync_queue',
              item_count: 3,
              last_synced_at: 1_700_000_000,
              is_stale: true,
            },
          ],
        },
      });

      render(<SyncStatusIndicator />);

      const button = screen.getByRole('button');
      fireEvent.click(button);

      await waitFor(() => {
        expect(screen.getByText('キャッシュ状態')).toBeInTheDocument();
        expect(screen.getByText('同期キュー')).toBeInTheDocument();
      });

      const refreshButton = screen.getByLabelText('キャッシュ情報を更新');
      fireEvent.click(refreshButton);
      expect(mockRefreshCacheStatus).toHaveBeenCalled();
    });

    it('再送キューを追加できる', async () => {
      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        cacheStatus: {
          total_items: 1,
          stale_items: 1,
          cache_types: [
            { cache_type: 'sync_queue', item_count: 1, last_synced_at: null, is_stale: true },
          ],
        },
      });

      render(<SyncStatusIndicator />);

      const button = screen.getByRole('button');
      fireEvent.click(button);

      await waitFor(() => {
        const queueButton = screen.getByText('再送キュー');
        fireEvent.click(queueButton);
      });

      await waitFor(() => {
        expect(mockEnqueueSyncRequest).toHaveBeenCalledWith('sync_queue');
      });
    });

    it('キャッシュメタデータを整形して表示する', () => {
      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        cacheStatus: {
          total_items: 1,
          stale_items: 1,
          cache_types: [
            {
              cache_type: 'sync_queue',
              item_count: 2,
              last_synced_at: 1_730_000_000,
              is_stale: true,
              metadata: {
                cacheType: 'offline_actions',
                requestedBy: 'npub1exampleexampleexample',
                requestedAt: '2025-11-09T00:00:00Z',
                queueItemId: 42,
                source: 'sync_status_indicator',
              },
            },
          ],
        },
      });

      render(<SyncStatusIndicator />);

      const trigger = screen.getByRole('button');
      fireEvent.click(trigger);

      expect(screen.getByText('2件 / 要再同期')).toBeInTheDocument();
      const metadataSection = screen.getByTestId('cache-metadata-sync_queue');
      expect(within(metadataSection).getByText('対象キャッシュ')).toBeInTheDocument();
      expect(within(metadataSection).getByText('offline_actions')).toBeInTheDocument();
      expect(within(metadataSection).getByText('最終要求者')).toBeInTheDocument();
      expect(within(metadataSection).getByText('キュー ID')).toBeInTheDocument();
      expect(within(metadataSection).getByText('#42')).toBeInTheDocument();
      expect(within(metadataSection).getByText('発行元')).toBeInTheDocument();
      expect(within(metadataSection).getByTitle('2025-11-09T00:00:00Z')).toBeInTheDocument();
    });
  });

  describe('再送キュー履歴', () => {
    it('履歴を表示し、最新IDをハイライトする', async () => {
      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        queueItems: [
          {
            id: 1,
            action_type: 'manual_sync_refresh',
            status: 'pending',
            retry_count: 0,
            max_retries: 3,
            created_at: 1_700_000_000,
            updated_at: 1_700_000_100,
            payload: {
              cacheType: 'offline_actions',
              requestedBy: 'npub1exampleexampleexample',
              source: 'sync_status_indicator',
              requestedAt: '2025-11-09T00:00:00Z',
            },
          },
          {
            id: 2,
            action_type: 'manual_sync_refresh',
            status: 'failed',
            retry_count: 1,
            max_retries: 3,
            created_at: 1_700_000_010,
            updated_at: 1_700_000_200,
            payload: { cacheType: 'cache_metadata' },
            error_message: 'timeout',
          },
        ],
        lastQueuedItemId: 1,
      });

      render(<SyncStatusIndicator />);
      fireEvent.click(screen.getByRole('button'));

      await waitFor(() => {
        expect(screen.getByPlaceholderText('Queue ID / cacheType を検索')).toBeInTheDocument();
      });

      const highlighted = screen.getByTestId('queue-item-1');
      expect(highlighted.className).toContain('border-primary');
      expect(within(highlighted).getByText('#1')).toBeInTheDocument();
      expect(screen.getByText(/最新 #/)).toBeInTheDocument();
      expect(screen.getByText('timeout')).toBeInTheDocument();
      expect(screen.getByText('再送キュー履歴')).toBeInTheDocument();
    });

    it('フィルタ入力で履歴を絞り込む', async () => {
      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        queueItems: [
          {
            id: 1,
            action_type: 'manual_sync_refresh',
            status: 'pending',
            retry_count: 0,
            max_retries: 3,
            created_at: 0,
            updated_at: 0,
            payload: { cacheType: 'offline_actions' },
          },
          {
            id: 2,
            action_type: 'manual_sync_refresh',
            status: 'pending',
            retry_count: 0,
            max_retries: 3,
            created_at: 0,
            updated_at: 0,
            payload: { cacheType: 'cache_metadata' },
          },
        ],
      });

      render(<SyncStatusIndicator />);
      fireEvent.click(screen.getByRole('button'));

      const input = await screen.findByPlaceholderText('Queue ID / cacheType を検索');
      fireEvent.change(input, { target: { value: 'metadata' } });

      await waitFor(() => {
        expect(screen.queryByTestId('queue-item-1')).not.toBeInTheDocument();
        expect(screen.getByTestId('queue-item-2')).toBeInTheDocument();
      });
    });

    it('更新ボタンで再送キューを再取得する', async () => {
      const refreshQueueItems = vi.fn();
      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        refreshQueueItems,
      });

      render(<SyncStatusIndicator />);
      fireEvent.click(screen.getByRole('button'));

      const refreshButton = await screen.findByLabelText('再送キューを更新');
      fireEvent.click(refreshButton);
      expect(refreshQueueItems).toHaveBeenCalled();
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
        const conflictItem = screen.getByText('create_post');
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
        const conflictItem = screen.getByText('create_post');
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
        const conflictItem = screen.getByText('create_post');
        fireEvent.click(conflictItem);
      });

      // リモートを適用ボタンをクリック
      await waitFor(() => {
        const applyRemoteButton = screen.getByText('リモートを適用');
        fireEvent.click(applyRemoteButton);
      });

      expect(mockResolveConflict).toHaveBeenCalledWith(mockConflict, 'remote');
    });

    it('Doc/Blob 情報をタブで比較表示', async () => {
      const user = userEvent.setup();
      const docConflict: SyncConflict = {
        localAction: {
          id: 1,
          localId: 'local_doc',
          userPubkey: 'user123',
          actionType: OfflineActionType.PROFILE_UPDATE,
          actionData: JSON.stringify({
            docVersion: 2,
            blobHash: 'bao1abcdefghijklmno1234567890',
            payloadBytes: 2048,
            format: 'image/png',
          }),
          createdAt: '2024-01-01T00:00:00Z',
          isSynced: false,
        },
        remoteAction: {
          id: 2,
          localId: 'remote_doc',
          userPubkey: 'user999',
          actionType: OfflineActionType.PROFILE_UPDATE,
          actionData: JSON.stringify({
            docVersion: 3,
            blobHash: 'bao1zyxwvutsrqponm9876543210',
            payloadBytes: 4096,
            format: 'image/png',
          }),
          createdAt: '2024-01-01T00:00:10Z',
          isSynced: true,
        },
        conflictType: 'version',
      };

      vi.mocked(useSyncManager).mockReturnValue({
        ...defaultManagerState,
        syncStatus: {
          ...defaultSyncStatus,
          conflicts: [docConflict],
        },
      });

      render(<SyncStatusIndicator />);

      const indicatorButton = screen.getByRole('button');
      fireEvent.click(indicatorButton);

      await waitFor(() => {
        expect(screen.getByText('profile_update')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText('profile_update'));

      await waitFor(() => {
        expect(screen.getByText('Doc/Blob')).toBeInTheDocument();
      });

      await user.click(screen.getByRole('tab', { name: 'Doc/Blob' }));

      await waitFor(() => {
        expect(screen.getByRole('tab', { name: 'Doc/Blob' })).toHaveAttribute('data-state', 'active');
      });

      expect(screen.getAllByText('ローカル').length).toBeGreaterThan(0);
      expect(screen.getAllByText('リモート').length).toBeGreaterThan(0);
      expect(screen.getByText('2')).toBeInTheDocument();
      expect(screen.getByText('3')).toBeInTheDocument();
      expect(screen.getByText(/Payload Size/i)).toBeInTheDocument();
    });
  });
});
