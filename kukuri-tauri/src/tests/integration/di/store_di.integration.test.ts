import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { setupIntegrationTest } from '../setup';
import { useP2PStore } from '@/stores/p2pStore';
import { p2pApi } from '@/lib/api/p2p';
import { errorHandler } from '@/lib/errorHandler';
import { offlineSyncService } from '@/services/offlineSyncService';
import { useOfflineStore } from '@/stores/offlineStore';
import { useAuthStore } from '@/stores/authStore';
import type { OfflineAction } from '@/types/offline';

type OfflineStoreState = ReturnType<typeof useOfflineStore.getState>;

vi.mock('@/lib/api/p2p', () => ({
  p2pApi: {
    initialize: vi.fn(),
    getNodeAddress: vi.fn(),
    getStatus: vi.fn(),
    joinTopic: vi.fn(),
    leaveTopic: vi.fn(),
    broadcast: vi.fn(),
    getMetrics: vi.fn(),
    getBootstrapConfig: vi.fn(),
    setBootstrapNodes: vi.fn(),
    clearBootstrapNodes: vi.fn(),
  },
}));

vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
    warn: vi.fn(),
    info: vi.fn(),
  },
}));

const createOfflineAction = (overrides: Partial<OfflineAction> = {}): OfflineAction => ({
  id: 1,
  userPubkey: 'pubkey123',
  actionType: 'CREATE_POST',
  actionData: '{}',
  localId: 'local-1',
  isSynced: false,
  createdAt: Date.now(),
  ...overrides,
});

describe('Dependency integration across stores and services', () => {
  let cleanup: () => void;
  let originalSyncPendingActions: OfflineStoreState['syncPendingActions'];

  beforeEach(() => {
    cleanup = setupIntegrationTest();
    useP2PStore.getState().reset();

    useOfflineStore.setState({
      isOnline: true,
      lastSyncedAt: undefined,
      pendingActions: [],
      syncQueue: [],
      optimisticUpdates: new Map(),
      isSyncing: false,
      syncErrors: new Map(),
    });

    useAuthStore.setState({
      isAuthenticated: false,
      currentUser: null,
      privateKey: null,
      relayStatus: [],
      accounts: [],
    });

    originalSyncPendingActions = useOfflineStore.getState().syncPendingActions;
    vi.clearAllMocks();
    offlineSyncService.cleanup();
  });

  afterEach(() => {
    useP2PStore.getState().reset();
    useOfflineStore.setState({
      syncPendingActions: originalSyncPendingActions,
      pendingActions: [],
      isSyncing: false,
      syncErrors: new Map(),
    });
    useAuthStore.setState({
      isAuthenticated: false,
      currentUser: null,
      privateKey: null,
      relayStatus: [],
      accounts: [],
    });
    offlineSyncService.cleanup();
    cleanup();
  });

  describe('P2P store initialization', () => {
    it('initializes node metadata via injected p2pApi', async () => {
      vi.mocked(p2pApi.initialize).mockResolvedValue(undefined);
      vi.mocked(p2pApi.getNodeAddress).mockResolvedValue([
        '/ip4/127.0.0.1/tcp/4001',
        '/ip6/::1/tcp/4001',
      ]);
      vi.mocked(p2pApi.getStatus).mockResolvedValue({
        connected: true,
        endpoint_id: 'node-123',
        active_topics: [],
        peer_count: 2,
        metrics_summary: {
          joins: 1,
          leaves: 0,
          broadcasts_sent: 2,
          messages_received: 3,
        },
      });

      await useP2PStore.getState().initialize();

      const state = useP2PStore.getState();
      expect(state.initialized).toBe(true);
      expect(state.connectionStatus).toBe('connected');
      expect(state.nodeId).toBe('node-123');
      expect(state.nodeAddr).toContain('/ip4/127.0.0.1/tcp/4001');
      expect(state.metricsSummary?.broadcasts_sent).toBe(2);
      expect(p2pApi.initialize).toHaveBeenCalledTimes(1);
      expect(p2pApi.getNodeAddress).toHaveBeenCalledTimes(1);
      expect(p2pApi.getStatus).toHaveBeenCalledTimes(1);
      expect(errorHandler.info).not.toHaveBeenCalled();
    });
  });

  describe('Offline sync coordination', () => {
    it('triggers offline synchronization when dependencies are satisfied', async () => {
      const syncPendingActionsMock = vi.fn().mockResolvedValue(undefined);
      useOfflineStore.setState({
        isOnline: true,
        isSyncing: false,
        pendingActions: [createOfflineAction()],
        syncPendingActions: syncPendingActionsMock as typeof originalSyncPendingActions,
      });

      useAuthStore.setState({
        isAuthenticated: true,
        currentUser: {
          id: 'user-1',
          pubkey: 'pubkey123',
          npub: 'npub123',
          name: 'Test User',
          displayName: 'Test User',
          about: '',
          picture: '',
          nip05: '',
        },
        privateKey: 'nsec123',
      });

      await offlineSyncService.triggerSync();

      expect(syncPendingActionsMock).toHaveBeenCalledWith('pubkey123');
      expect(errorHandler.info).toHaveBeenCalledWith(
        'Starting sync for 1 pending actions',
        'OfflineSyncService',
      );
    });
  });
});
