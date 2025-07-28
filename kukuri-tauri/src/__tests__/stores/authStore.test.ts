import { vi, describe, it, expect, beforeEach } from 'vitest';
import { useAuthStore } from '@/stores/authStore';
import { TauriApi } from '@/lib/api/tauri';
import { SecureStorageApi } from '@/lib/api/secureStorage';
import * as nostrApi from '@/lib/api/nostr';

// モック設定
vi.mock('@/lib/api/tauri');
vi.mock('@/lib/api/secureStorage');
vi.mock('@/lib/api/nostr');

const mockTauriApi = TauriApi as unknown as {
  generateKeypair: ReturnType<typeof vi.fn>;
  login: ReturnType<typeof vi.fn>;
  logout: ReturnType<typeof vi.fn>;
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
    // ストアをリセット
    useAuthStore.setState({
      isAuthenticated: false,
      currentUser: null,
      privateKey: null,
      relayStatus: [],
      accounts: [],
    });
    
    // モックをクリア
    vi.clearAllMocks();
    
    // デフォルトのモック実装
    mockNostrApi.initializeNostr = vi.fn().mockResolvedValue(undefined);
    mockNostrApi.disconnectNostr = vi.fn().mockResolvedValue(undefined);
    mockNostrApi.getRelayStatus = vi.fn().mockResolvedValue([]);
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
      });
      expect(state.privateKey).toBe('nsec1current');
      expect(state.accounts).toEqual(mockAccounts);
      expect(mockNostrApi.initializeNostr).toHaveBeenCalled();
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
      mockSecureStorageApi.getCurrentAccount = vi.fn().mockRejectedValue(new Error('Storage error'));
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
      const nsec = 'nsec1test123';
      const mockLoginResponse = {
        public_key: 'pubkey123',
        npub: 'npub1test123',
      };

      mockTauriApi.login = vi.fn().mockResolvedValue(mockLoginResponse);
      mockSecureStorageApi.addAccount = vi.fn().mockResolvedValue({
        npub: 'npub1test123',
        pubkey: 'pubkey123',
      });
      mockSecureStorageApi.listAccounts = vi.fn().mockResolvedValue([]);

      await useAuthStore.getState().loginWithNsec(nsec, true);

      expect(mockSecureStorageApi.addAccount).toHaveBeenCalledWith({
        nsec,
        name: 'ユーザー',
        display_name: 'ユーザー',
        picture: '',
      });

      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(true);
      expect(state.currentUser?.npub).toBe('npub1test123');
    });

    it('should not save to secure storage when saveToSecureStorage is false', async () => {
      const nsec = 'nsec1test123';
      const mockLoginResponse = {
        public_key: 'pubkey123',
        npub: 'npub1test123',
      };

      mockTauriApi.login = vi.fn().mockResolvedValue(mockLoginResponse);
      mockSecureStorageApi.listAccounts = vi.fn().mockResolvedValue([]);

      await useAuthStore.getState().loginWithNsec(nsec, false);

      expect(mockSecureStorageApi.addAccount).not.toHaveBeenCalled();
    });
  });

  describe('generateNewKeypair with secure storage', () => {
    it('should save to secure storage by default', async () => {
      const mockKeypairResponse = {
        public_key: 'pubkey123',
        nsec: 'nsec1generated',
      };

      mockTauriApi.generateKeypair = vi.fn().mockResolvedValue(mockKeypairResponse);
      mockSecureStorageApi.addAccount = vi.fn().mockResolvedValue({
        npub: 'pubkey123',
        pubkey: 'pubkey123',
      });
      mockSecureStorageApi.listAccounts = vi.fn().mockResolvedValue([]);

      const result = await useAuthStore.getState().generateNewKeypair();

      expect(mockSecureStorageApi.addAccount).toHaveBeenCalledWith({
        nsec: 'nsec1generated',
        name: '新規ユーザー',
        display_name: '新規ユーザー',
        picture: '',
      });

      expect(result.nsec).toBe('nsec1generated');
      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(true);
    });

    it('should not save to secure storage when saveToSecureStorage is false', async () => {
      const mockKeypairResponse = {
        public_key: 'pubkey123',
        nsec: 'nsec1generated',
      };

      mockTauriApi.generateKeypair = vi.fn().mockResolvedValue(mockKeypairResponse);
      mockSecureStorageApi.listAccounts = vi.fn().mockResolvedValue([]);

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

      const mockAccounts = [{
        npub: 'npub1alice',
        pubkey: 'pubkey_alice',
        name: 'Alice',
        display_name: 'Alice Smith',
        picture: 'https://example.com/alice.png',
        last_used: '2024-01-01T00:00:00Z',
      }];

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
      });
      expect(state.privateKey).toBeNull(); // セキュアストレージから取得したものは保持しない
      expect(mockNostrApi.initializeNostr).toHaveBeenCalled();
    });

    it('should handle errors when switching to non-existent account', async () => {
      mockSecureStorageApi.secureLogin = vi.fn().mockRejectedValue(new Error('Account not found'));

      await expect(useAuthStore.getState().switchAccount('npub_not_exist')).rejects.toThrow();
    });
  });

  describe('removeAccount', () => {
    it('should remove account and logout if current account', async () => {
      // 現在のアカウントを設定
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
      // 別のアカウントを現在のアカウントとして設定
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
      expect(state.accounts).toEqual([]);
    });
  });
});