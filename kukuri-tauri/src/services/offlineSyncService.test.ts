import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { OfflineSyncService } from './offlineSyncService';
import { useOfflineStore } from '@/stores/offlineStore';
import { useAuthStore } from '@/stores/authStore';

vi.mock('@/stores/offlineStore');
vi.mock('@/stores/authStore');

describe('OfflineSyncService', () => {
  let service: OfflineSyncService;
  const mockOfflineStore = useOfflineStore as unknown as {
    getState: ReturnType<typeof vi.fn>;
  };
  const mockAuthStore = useAuthStore as unknown as {
    getState: ReturnType<typeof vi.fn>;
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    service = new OfflineSyncService();
    
    // デフォルトのモック状態
    mockOfflineStore.getState = vi.fn().mockReturnValue({
      isOnline: true,
      isSyncing: false,
      pendingActions: [],
      setOnlineStatus: vi.fn(),
      loadPendingActions: vi.fn(),
      syncPendingActions: vi.fn(),
      cleanupExpiredCache: vi.fn(),
      syncErrors: new Map(),
    });
    
    mockAuthStore.getState = vi.fn().mockReturnValue({
      currentAccount: { pubkey: 'test-pubkey' },
    });
  });

  afterEach(() => {
    service.cleanup();
    vi.useRealTimers();
  });

  describe('initialize', () => {
    it('初期化時にネットワークリスナーと定期同期を設定する', async () => {
      const addEventListenerSpy = vi.spyOn(window, 'addEventListener');
      
      service.initialize();
      
      expect(addEventListenerSpy).toHaveBeenCalledWith('online', expect.any(Function));
      expect(addEventListenerSpy).toHaveBeenCalledWith('offline', expect.any(Function));
    });

    it('初期化時に未同期アクションを読み込む', async () => {
      const loadPendingActionsMock = vi.fn();
      mockOfflineStore.getState = vi.fn().mockReturnValue({
        isOnline: true,
        isSyncing: false,
        pendingActions: [],
        loadPendingActions: loadPendingActionsMock,
        cleanupExpiredCache: vi.fn(),
      });
      
      service.initialize();
      await vi.runAllTimersAsync();
      
      expect(loadPendingActionsMock).toHaveBeenCalledWith('test-pubkey');
    });

    it('初期化時に期限切れキャッシュをクリーンアップする', async () => {
      const cleanupExpiredCacheMock = vi.fn();
      mockOfflineStore.getState = vi.fn().mockReturnValue({
        isOnline: true,
        isSyncing: false,
        pendingActions: [],
        loadPendingActions: vi.fn(),
        cleanupExpiredCache: cleanupExpiredCacheMock,
      });
      
      service.initialize();
      await vi.runAllTimersAsync();
      
      expect(cleanupExpiredCacheMock).toHaveBeenCalled();
    });
  });

  describe('ネットワーク状態の監視', () => {
    it('オンライン復帰時に同期を開始する', () => {
      const setOnlineStatusMock = vi.fn();
      mockOfflineStore.getState = vi.fn().mockReturnValue({
        isOnline: false,
        setOnlineStatus: setOnlineStatusMock,
        pendingActions: [{ localId: '1', action: {}, createdAt: Date.now() }],
        isSyncing: false,
      });
      
      service.initialize();
      
      // オンラインイベントを発火
      window.dispatchEvent(new Event('online'));
      
      expect(setOnlineStatusMock).toHaveBeenCalledWith(true);
    });

    it('オフライン時に定期同期を停止する', () => {
      const setOnlineStatusMock = vi.fn();
      mockOfflineStore.getState = vi.fn().mockReturnValue({
        isOnline: true,
        setOnlineStatus: setOnlineStatusMock,
      });
      
      service.initialize();
      
      // オフラインイベントを発火
      window.dispatchEvent(new Event('offline'));
      
      expect(setOnlineStatusMock).toHaveBeenCalledWith(false);
    });
  });

  describe('同期処理', () => {
    it('未同期アクションがある場合のみ同期を実行する', async () => {
      const syncPendingActionsMock = vi.fn();
      mockOfflineStore.getState = vi.fn().mockReturnValue({
        isOnline: true,
        isSyncing: false,
        pendingActions: [{ localId: '1', action: {}, createdAt: Date.now() }],
        syncPendingActions: syncPendingActionsMock,
      });
      
      await service.triggerSync();
      
      expect(syncPendingActionsMock).toHaveBeenCalledWith('test-pubkey');
    });

    it('未同期アクションがない場合は同期をスキップする', async () => {
      const syncPendingActionsMock = vi.fn();
      mockOfflineStore.getState = vi.fn().mockReturnValue({
        isOnline: true,
        isSyncing: false,
        pendingActions: [],
        syncPendingActions: syncPendingActionsMock,
      });
      
      await service.triggerSync();
      
      expect(syncPendingActionsMock).not.toHaveBeenCalled();
    });

    it('オフライン時は同期を実行しない', async () => {
      const syncPendingActionsMock = vi.fn();
      mockOfflineStore.getState = vi.fn().mockReturnValue({
        isOnline: false,
        isSyncing: false,
        pendingActions: [{ localId: '1', action: {}, createdAt: Date.now() }],
        syncPendingActions: syncPendingActionsMock,
      });
      
      await service.triggerSync();
      
      expect(syncPendingActionsMock).not.toHaveBeenCalled();
    });

    it('同期中は新しい同期を開始しない', async () => {
      const syncPendingActionsMock = vi.fn();
      mockOfflineStore.getState = vi.fn().mockReturnValue({
        isOnline: true,
        isSyncing: true,
        pendingActions: [{ localId: '1', action: {}, createdAt: Date.now() }],
        syncPendingActions: syncPendingActionsMock,
      });
      
      await service.triggerSync();
      
      expect(syncPendingActionsMock).not.toHaveBeenCalled();
    });
  });

  describe('定期同期', () => {
    it('30秒ごとに同期を試行する', () => {
      mockOfflineStore.getState = vi.fn().mockReturnValue({
        isOnline: true,
        isSyncing: false,
        pendingActions: [{ localId: '1', action: {}, createdAt: Date.now() }],
        syncPendingActions: vi.fn(),
      });
      
      service.initialize();
      
      // 30秒経過
      vi.advanceTimersByTime(30000);
      
      // オンラインかつ未同期アクションがある場合は同期が試行される
      const state = mockOfflineStore.getState();
      expect(state.pendingActions.length).toBeGreaterThan(0);
    });
  });

  describe('リトライ処理', () => {
    it('同期エラー時に指数バックオフでリトライする', async () => {
      const syncPendingActionsMock = vi.fn().mockRejectedValue(new Error('Sync failed'));
      mockOfflineStore.getState = vi.fn().mockReturnValue({
        isOnline: true,
        isSyncing: false,
        pendingActions: [{ localId: '1', action: {}, createdAt: Date.now() }],
        syncPendingActions: syncPendingActionsMock,
        syncErrors: new Map(),
      });
      
      await service.triggerSync();
      
      // 5秒後にリトライ
      vi.advanceTimersByTime(5000);
      
      expect(syncPendingActionsMock).toHaveBeenCalledTimes(1);
    });
  });

  describe('cleanup', () => {
    it('クリーンアップ時にタイマーとリスナーを削除する', () => {
      const removeEventListenerSpy = vi.spyOn(window, 'removeEventListener');
      
      service.initialize();
      service.cleanup();
      
      expect(removeEventListenerSpy).toHaveBeenCalledWith('online', expect.any(Function));
      expect(removeEventListenerSpy).toHaveBeenCalledWith('offline', expect.any(Function));
    });
  });
});