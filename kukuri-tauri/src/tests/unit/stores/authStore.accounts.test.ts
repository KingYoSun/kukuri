import { waitFor } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  clearFallbackAccounts,
  listFallbackAccountMetadata,
  useAuthStore,
} from '@/stores/authStore';
import { TauriApi } from '@/lib/api/tauri';
import { SecureStorageApi } from '@/lib/api/secureStorage';
import * as nostrApi from '@/lib/api/nostr';
import { DEFAULT_PUBLIC_TOPIC_ID } from '@/constants/topics';
import i18n from '@/i18n';

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    generateKeypair: vi.fn(),
    login: vi.fn(),
    logout: vi.fn(),
    fetchProfileAvatar: vi.fn(),
    getTopics: vi.fn(),
    joinTopic: vi.fn(),
  },
}));
vi.mock('@/lib/api/secureStorage');
vi.mock('@/lib/api/nostr');

const topicStoreState = {
  topics: new Map<string, any>(),
  joinedTopics: [] as string[],
  topicUnreadCounts: new Map<string, number>(),
  topicLastReadAt: new Map<string, number>(),
  currentTopic: null as any,
  fetchTopics: vi.fn(async () => {
    topicStoreState.topics = new Map([
      [
        DEFAULT_PUBLIC_TOPIC_ID,
        {
          id: DEFAULT_PUBLIC_TOPIC_ID,
          name: '#public',
          description: '',
          tags: [],
          memberCount: 0,
          postCount: 0,
          isActive: true,
          createdAt: new Date(),
        },
      ],
    ]);
  }),
  joinTopic: vi.fn(async () => {}),
  setCurrentTopic: vi.fn((topic: any) => {
    topicStoreState.currentTopic = topic;
  }),
};

vi.mock('@/stores/topicStore', () => ({
  useTopicStore: {
    getState: () => topicStoreState,
  },
}));

const mockTauriApi = TauriApi as unknown as {
  generateKeypair: ReturnType<typeof vi.fn>;
  login: ReturnType<typeof vi.fn>;
  logout: ReturnType<typeof vi.fn>;
  fetchProfileAvatar: ReturnType<typeof vi.fn>;
  getTopics: ReturnType<typeof vi.fn>;
  joinTopic: ReturnType<typeof vi.fn>;
};

const mockSecureStorageApi = SecureStorageApi as unknown as {
  addAccount: ReturnType<typeof vi.fn>;
  listAccounts: ReturnType<typeof vi.fn>;
  switchAccount: ReturnType<typeof vi.fn>;
  removeAccount: ReturnType<typeof vi.fn>;
  getCurrentAccount: ReturnType<typeof vi.fn>;
  secureLogin: ReturnType<typeof vi.fn>;
};

const mockNostrApi = nostrApi as unknown as {
  initializeNostr: ReturnType<typeof vi.fn>;
  disconnectNostr: ReturnType<typeof vi.fn>;
  getRelayStatus: ReturnType<typeof vi.fn>;
};

describe('authStore - Multiple Account Management', () => {
  beforeEach(() => {
    useAuthStore.setState({
      isAuthenticated: false,
      currentUser: null,
      privateKey: null,
      relayStatus: [],
      accounts: [],
    });
    clearFallbackAccounts();

    vi.clearAllMocks();

    mockNostrApi.initializeNostr.mockResolvedValue(undefined);
    mockNostrApi.disconnectNostr.mockResolvedValue(undefined);
    mockNostrApi.getRelayStatus.mockResolvedValue([]);
    mockTauriApi.fetchProfileAvatar.mockRejectedValue(new Error('Profile avatar not found'));
    mockTauriApi.getTopics.mockResolvedValue([]);
    mockTauriApi.joinTopic.mockResolvedValue(undefined);

    topicStoreState.topics = new Map();
    topicStoreState.joinedTopics = [];
    topicStoreState.topicUnreadCounts = new Map();
    topicStoreState.topicLastReadAt = new Map();
    topicStoreState.currentTopic = null;
    topicStoreState.fetchTopics = vi.fn(async () => {
      topicStoreState.topics = new Map([
        [
          DEFAULT_PUBLIC_TOPIC_ID,
          {
            id: DEFAULT_PUBLIC_TOPIC_ID,
            name: '#public',
            description: '',
            tags: [],
            memberCount: 0,
            postCount: 0,
            isActive: true,
            createdAt: new Date(),
          },
        ],
      ]);
    });
    topicStoreState.joinTopic = vi.fn(async () => {});
    topicStoreState.setCurrentTopic = vi.fn((topic: any) => {
      topicStoreState.currentTopic = topic;
    });
  });

  describe('initialize with auto-login', () => {
    it('should auto-login when current account exists in secure storage', async () => {
      const mockCurrentAccount = {
        npub: 'npub1current',
        nsec: 'nsec1current',
        pubkey: 'pubkey_current',
        metadata: {
          npub: 'npub1current',
          pubkey: 'pubkey_current',
          name: 'Current User',
          display_name: 'Current User Display',
          picture: 'https://example.com/avatar.png',
          last_used: '2024-01-01T00:00:00Z',
        },
      };

      const mockAccounts = [
        {
          npub: 'npub1current',
          pubkey: 'pubkey_current',
          name: 'Current User',
          display_name: 'Current User Display',
          picture: 'https://example.com/avatar.png',
          last_used: '2024-01-01T00:00:00Z',
        },
        {
          npub: 'npub1other',
          pubkey: 'pubkey_other',
          name: 'Other User',
          display_name: 'Other User Display',
          last_used: '2024-01-02T00:00:00Z',
        },
      ];

      mockSecureStorageApi.getCurrentAccount = vi.fn().mockResolvedValue(mockCurrentAccount);
      mockSecureStorageApi.listAccounts = vi.fn().mockResolvedValue(mockAccounts);

      await useAuthStore.getState().initialize();

      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(true);
      expect(state.currentUser).toEqual({
        id: 'pubkey_current',
        pubkey: 'pubkey_current',
        npub: 'npub1current',
        name: 'Current User',
        displayName: 'Current User Display',
        about: '',
        picture: 'https://example.com/avatar.png',
        nip05: '',
        avatar: null,
        publicProfile: true,
        showOnlineStatus: false,
      });
      expect(state.privateKey).toBe('nsec1current');
      expect(state.accounts).toEqual(mockAccounts);
      expect(mockNostrApi.initializeNostr).toHaveBeenCalled();
    });

    it('should populate avatar metadata when fetchProfileAvatar succeeds', async () => {
      const mockCurrentAccount = {
        npub: 'npub1current',
        nsec: 'nsec1current',
        pubkey: 'pubkey_current',
        metadata: {
          npub: 'npub1current',
          pubkey: 'pubkey_current',
          name: 'Current User',
          display_name: 'Current User Display',
          picture: '',
          last_used: '2024-01-01T00:00:00Z',
        },
      };

      mockSecureStorageApi.getCurrentAccount = vi.fn().mockResolvedValue(mockCurrentAccount);
      mockSecureStorageApi.listAccounts = vi.fn().mockResolvedValue([]);
      mockTauriApi.fetchProfileAvatar.mockResolvedValue({
        npub: 'npub1current',
        blob_hash: 'abc123',
        format: 'image/png',
        size_bytes: 3,
        access_level: 'public',
        share_ticket: 'ticket-1',
        doc_version: 5,
        updated_at: '2025-11-02T00:00:00Z',
        content_sha256: 'deadbeef',
        data_base64: 'AQID',
      });

      await useAuthStore.getState().initialize();

      await waitFor(() => {
        expect(mockTauriApi.fetchProfileAvatar).toHaveBeenCalledWith('npub1current');
      });
    });

    it('should not auto-login when no current account exists', async () => {
      mockSecureStorageApi.getCurrentAccount = vi.fn().mockResolvedValue(null);
      mockSecureStorageApi.listAccounts = vi.fn().mockResolvedValue([]);

      await useAuthStore.getState().initialize();

      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(false);
      expect(state.currentUser).toBeNull();
      expect(state.privateKey).toBeNull();
      expect(state.accounts).toEqual([]);
      expect(mockNostrApi.initializeNostr).not.toHaveBeenCalled();
    });

    it('should handle errors during initialization', async () => {
      mockSecureStorageApi.getCurrentAccount = vi
        .fn()
        .mockRejectedValue(new Error('Storage error'));
      mockSecureStorageApi.listAccounts = vi.fn().mockResolvedValue([]);

      await useAuthStore.getState().initialize();

      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(false);
      expect(state.currentUser).toBeNull();
      expect(state.privateKey).toBeNull();
      expect(state.accounts).toEqual([]);
    });
  });

  describe('loginWithNsec with secure storage', () => {
    it('should save to secure storage when saveToSecureStorage is true', async () => {
      const mockLoginResponse = {
        public_key: 'pubkey123',
        npub: 'npub1example',
      };
      mockTauriApi.login = vi.fn().mockResolvedValue(mockLoginResponse);
      mockSecureStorageApi.addAccount = vi.fn().mockResolvedValue({
        success: true,
      });
      mockSecureStorageApi.listAccounts = vi.fn().mockResolvedValue([]);

      await useAuthStore.getState().loginWithNsec('nsec123', true);

      expect(mockSecureStorageApi.addAccount).toHaveBeenCalledWith({
        nsec: 'nsec123',
        name: i18n.t('auth.newUser'),
        display_name: i18n.t('auth.newUser'),
        picture: '',
      });
      expect(mockTauriApi.login).toHaveBeenCalledWith({ nsec: 'nsec123' });
      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(true);
      expect(state.currentUser?.avatar).toBeNull();
    });

    it('should not save to secure storage when saveToSecureStorage is false', async () => {
      const mockLoginResponse = {
        public_key: 'pubkey123',
        npub: 'npub1example',
      };
      mockTauriApi.login = vi.fn().mockResolvedValue(mockLoginResponse);

      await useAuthStore.getState().loginWithNsec('nsec123', false);

      expect(mockSecureStorageApi.addAccount).not.toHaveBeenCalled();
    });
  });

  describe('generateNewKeypair with secure storage', () => {
    it('should save to secure storage by default', async () => {
      const mockKeypairResponse = {
        public_key: 'pubkey123',
        npub: 'npub1example',
        nsec: 'nsec1example',
      };
      mockTauriApi.generateKeypair = vi.fn().mockResolvedValue(mockKeypairResponse);
      mockSecureStorageApi.addAccount = vi.fn().mockResolvedValue({
        success: true,
      });
      mockSecureStorageApi.listAccounts = vi.fn().mockResolvedValue([]);

      await useAuthStore.getState().generateNewKeypair();

      expect(mockSecureStorageApi.addAccount).toHaveBeenCalledWith({
        nsec: 'nsec1example',
        name: '新規ユーザー',
        display_name: '新規ユーザー',
        picture: '',
      });
      const state = useAuthStore.getState();
      expect(state.currentUser?.avatar).toBeNull();
    });

    it('should not save to secure storage when saveToSecureStorage is false', async () => {
      const mockKeypairResponse = {
        public_key: 'pubkey123',
        npub: 'npub1example',
        nsec: 'nsec1example',
      };
      mockTauriApi.generateKeypair = vi.fn().mockResolvedValue(mockKeypairResponse);

      await useAuthStore.getState().generateNewKeypair(false);

      expect(mockSecureStorageApi.addAccount).not.toHaveBeenCalled();
    });
  });

  describe('switchAccount', () => {
    it('should switch to different account successfully', async () => {
      const npub = 'npub1alice';
      const mockLoginResponse = {
        public_key: 'pubkey_alice',
        npub: 'npub1alice',
      };
      const mockAccounts = [
        {
          npub: 'npub1alice',
          pubkey: 'pubkey_alice',
          name: 'Alice',
          display_name: 'Alice Smith',
          picture: 'https://example.com/alice.png',
          last_used: '2024-01-01T00:00:00Z',
        },
      ];

      mockSecureStorageApi.secureLogin = vi.fn().mockResolvedValue(mockLoginResponse);
      mockSecureStorageApi.listAccounts = vi.fn().mockResolvedValue(mockAccounts);

      await useAuthStore.getState().switchAccount(npub);

      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(true);
      expect(state.currentUser).toEqual({
        id: 'pubkey_alice',
        pubkey: 'pubkey_alice',
        npub: 'npub1alice',
        name: 'Alice',
        displayName: 'Alice Smith',
        about: '',
        picture: 'https://example.com/alice.png',
        nip05: '',
        avatar: null,
        publicProfile: true,
        showOnlineStatus: false,
      });
      expect(state.privateKey).toBeNull();
      expect(mockNostrApi.initializeNostr).toHaveBeenCalled();
    });

    it('should handle errors when switching to non-existent account', async () => {
      mockSecureStorageApi.secureLogin = vi.fn().mockRejectedValue(new Error('Account not found'));

      await expect(useAuthStore.getState().switchAccount('npub_not_exist')).rejects.toThrow();
    });
  });

  describe('removeAccount', () => {
    it('should remove account and logout if current account', async () => {
      useAuthStore.setState({
        isAuthenticated: true,
        currentUser: {
          id: 'pubkey123',
          pubkey: 'pubkey123',
          npub: 'npub1current',
          name: 'Current User',
          displayName: 'Current User',
          about: '',
          picture: '',
          nip05: '',
          avatar: null,
        },
        privateKey: 'nsec1current',
      });

      mockSecureStorageApi.removeAccount = vi.fn().mockResolvedValue(undefined);
      mockSecureStorageApi.listAccounts = vi.fn().mockResolvedValue([]);
      mockTauriApi.logout = vi.fn().mockResolvedValue(undefined);

      await useAuthStore.getState().removeAccount('npub1current');

      expect(mockSecureStorageApi.removeAccount).toHaveBeenCalledWith('npub1current');
      expect(mockTauriApi.logout).toHaveBeenCalled();

      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(false);
      expect(state.currentUser).toBeNull();
    });

    it('should remove account without logout if not current account', async () => {
      useAuthStore.setState({
        isAuthenticated: true,
        currentUser: {
          id: 'pubkey_other',
          pubkey: 'pubkey_other',
          npub: 'npub1other',
          name: 'Other User',
          displayName: 'Other User',
          about: '',
          picture: '',
          nip05: '',
          avatar: null,
        },
      });

      mockSecureStorageApi.removeAccount = vi.fn().mockResolvedValue(undefined);
      mockSecureStorageApi.listAccounts = vi.fn().mockResolvedValue([]);

      await useAuthStore.getState().removeAccount('npub1alice');

      expect(mockSecureStorageApi.removeAccount).toHaveBeenCalledWith('npub1alice');
      expect(mockTauriApi.logout).not.toHaveBeenCalled();

      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(true);
      expect(state.currentUser?.npub).toBe('npub1other');
    });
  });

  describe('loadAccounts', () => {
    it('should load accounts from secure storage', async () => {
      const mockAccounts = [
        {
          npub: 'npub1alice',
          pubkey: 'pubkey_alice',
          name: 'Alice',
          display_name: 'Alice Smith',
          last_used: '2024-01-01T00:00:00Z',
        },
        {
          npub: 'npub1bob',
          pubkey: 'pubkey_bob',
          name: 'Bob',
          display_name: 'Bob Johnson',
          last_used: '2024-01-02T00:00:00Z',
        },
      ];

      mockSecureStorageApi.listAccounts = vi.fn().mockResolvedValue(mockAccounts);

      await useAuthStore.getState().loadAccounts();

      const state = useAuthStore.getState();
      expect(state.accounts).toEqual(mockAccounts);
    });

    it('should handle errors when loading accounts', async () => {
      mockSecureStorageApi.listAccounts = vi.fn().mockRejectedValue(new Error('Storage error'));

      await useAuthStore.getState().loadAccounts();

      const state = useAuthStore.getState();
      expect(state.accounts).toEqual(listFallbackAccountMetadata());
    });
  });
});
