import { describe, it, expect, beforeEach, vi, MockedFunction } from 'vitest';
import { clearFallbackAccounts, useAuthStore } from '@/stores/authStore';
import type { User } from '@/stores/types';
import { errorHandler } from '@/lib/errorHandler';

// errorHandlerをモック
vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
    warn: vi.fn(),
    info: vi.fn(),
  },
}));

// TauriApiをモック
vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    logout: vi.fn().mockResolvedValue(undefined),
    login: vi.fn(),
    generateKeypair: vi.fn(),
  },
}));

// Nostr APIをモック
vi.mock('@/lib/api/nostr', () => ({
  initializeNostr: vi.fn().mockResolvedValue(undefined),
  disconnectNostr: vi.fn().mockResolvedValue(undefined),
  getRelayStatus: vi.fn().mockResolvedValue([]),
}));

// SecureStorage APIをモック
vi.mock('@/lib/api/secureStorage', () => ({
  SecureStorageApi: {
    getCurrentAccount: vi.fn().mockResolvedValue(null),
    listAccounts: vi.fn().mockResolvedValue([]),
  },
}));

import { initializeNostr, disconnectNostr, getRelayStatus } from '@/lib/api/nostr';

const mockInitializeNostr = initializeNostr as MockedFunction<typeof initializeNostr>;
const mockDisconnectNostr = disconnectNostr as MockedFunction<typeof disconnectNostr>;
const mockGetRelayStatus = getRelayStatus as MockedFunction<typeof getRelayStatus>;

type AuthStoreState = ReturnType<typeof useAuthStore.getState>;

const createTestUser = (overrides: Partial<User> = {}): User => ({
  id: 'test123',
  pubkey: 'pubkey123',
  npub: 'npub123',
  name: 'テストユーザー',
  displayName: 'テストユーザー',
  picture: '',
  about: '',
  nip05: '',
  avatar: null,
  ...overrides,
});

const setAuthenticatedState = (
  stateOverrides: Partial<AuthStoreState> = {},
  userOverrides: Partial<User> = {},
): User => {
  const user = createTestUser(userOverrides);
  useAuthStore.setState({
    isAuthenticated: true,
    currentUser: user,
    privateKey: 'nsec123',
    relayStatus: [],
    relayStatusError: null,
    relayStatusBackoffMs: 30_000,
    lastRelayStatusFetchedAt: null,
    isFetchingRelayStatus: false,
    accounts: [],
    ...stateOverrides,
  });
  return user;
};

const expectLoggedOutState = () => {
  const state = useAuthStore.getState();
  expect(state.isAuthenticated).toBe(false);
  expect(state.currentUser).toBeNull();
  expect(state.privateKey).toBeNull();
  expect(state.relayStatus).toEqual([]);
  expect(state.relayStatusError).toBeNull();
  expect(state.relayStatusBackoffMs).toBe(30_000);
  expect(state.lastRelayStatusFetchedAt).toBeNull();
  expect(state.isFetchingRelayStatus).toBe(false);
};

describe('authStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({
      isAuthenticated: false,
      currentUser: null,
      privateKey: null,
      relayStatus: [],
      relayStatusError: null,
      relayStatusBackoffMs: 30_000,
      lastRelayStatusFetchedAt: null,
      isFetchingRelayStatus: false,
      accounts: [],
    });
    clearFallbackAccounts();
  });

  it('初期状態が正しく設定されていること', () => {
    const state = useAuthStore.getState();
    expect(state.isAuthenticated).toBe(false);
    expect(state.currentUser).toBeNull();
    expect(state.privateKey).toBeNull();
    expect(state.relayStatus).toEqual([]);
    expect(state.relayStatusError).toBeNull();
    expect(state.relayStatusBackoffMs).toBe(30_000);
    expect(state.lastRelayStatusFetchedAt).toBeNull();
    expect(state.isFetchingRelayStatus).toBe(false);
  });

  it('loginメソッドが正しく動作すること', () => {
    const testUser = createTestUser();
    const testPrivateKey = 'nsec123';

    useAuthStore.getState().login(testPrivateKey, testUser);

    const state = useAuthStore.getState();
    expect(state.isAuthenticated).toBe(true);
    expect(state.currentUser).toEqual(testUser);
    expect(state.privateKey).toBe(testPrivateKey);
  });

  it('logoutメソッドが正しく動作すること', async () => {
    setAuthenticatedState({
      relayStatus: [{ url: 'wss://relay.example', status: 'connected' }],
      relayStatusError: 'error',
      relayStatusBackoffMs: 120_000,
      lastRelayStatusFetchedAt: Date.now(),
      isFetchingRelayStatus: true,
    });

    await useAuthStore.getState().logout();

    expectLoggedOutState();
  });

  it('updateRelayStatusが成功すると状態とバックオフがリセットされる', async () => {
    mockGetRelayStatus.mockResolvedValueOnce([{ url: 'wss://relay.example', status: 'connected' }]);

    await useAuthStore.getState().updateRelayStatus();

    const state = useAuthStore.getState();
    expect(state.relayStatus).toHaveLength(1);
    expect(state.relayStatusError).toBeNull();
    expect(state.relayStatusBackoffMs).toBe(30_000);
    expect(state.isFetchingRelayStatus).toBe(false);
    expect(state.lastRelayStatusFetchedAt).not.toBeNull();
  });

  it('updateRelayStatusが失敗するとバックオフが増加しエラーが保持される', async () => {
    mockGetRelayStatus.mockRejectedValueOnce(new Error('network error'));

    const previousBackoff = useAuthStore.getState().relayStatusBackoffMs;

    await useAuthStore.getState().updateRelayStatus();

    const state = useAuthStore.getState();
    expect(state.relayStatusError).toBe('network error');
    expect(state.relayStatusBackoffMs).toBeGreaterThan(previousBackoff);
    expect(state.isFetchingRelayStatus).toBe(false);
    expect(state.lastRelayStatusFetchedAt).not.toBeNull();
  });

  it('updateUserメソッドが正しく動作すること', () => {
    const testUser = setAuthenticatedState();

    const updates = {
      name: '更新されたユーザー',
      about: '新しい自己紹介',
    };
    useAuthStore.getState().updateUser(updates);

    const state = useAuthStore.getState();
    expect(state.currentUser?.name).toBe('更新されたユーザー');
    expect(state.currentUser?.about).toBe('新しい自己紹介');
    expect(state.currentUser?.pubkey).toBe(testUser.pubkey);
  });

  it('currentUserがnullの場合updateUserが何もしないこと', () => {
    useAuthStore.getState().updateUser({ name: '新しい名前' });

    const state = useAuthStore.getState();
    expect(state.currentUser).toBeNull();
  });

  it('loginメソッドがNostrを初期化すること', async () => {
    const testUser = createTestUser();
    const testPrivateKey = 'nsec123';

    await useAuthStore.getState().login(testPrivateKey, testUser);

    expect(initializeNostr).toHaveBeenCalled();
  });

  it('logoutメソッドがNostrを切断すること', async () => {
    setAuthenticatedState();

    await useAuthStore.getState().logout();

    expect(disconnectNostr).toHaveBeenCalled();
  });

  it('setRelayStatusメソッドが正しく動作すること', () => {
    const relayStatus = [
      { url: 'wss://relay1.test', status: 'connected' },
      { url: 'wss://relay2.test', status: 'disconnected' },
    ];

    useAuthStore.getState().setRelayStatus(relayStatus);

    const state = useAuthStore.getState();
    expect(state.relayStatus).toEqual(relayStatus);
  });

  it('isLoggedInが正しく動作すること', () => {
    // 初期状態では false
    expect(useAuthStore.getState().isAuthenticated).toBe(false);

    setAuthenticatedState();

    // 認証後は true - isAuthenticatedを直接確認
    expect(useAuthStore.getState().isAuthenticated).toBe(true);
  });

  it('Nostr初期化エラーが処理されること', async () => {
    mockInitializeNostr.mockRejectedValueOnce(new Error('Nostr init failed'));

    const testUser = createTestUser();

    await useAuthStore.getState().login('nsec123', testUser);

    expect(errorHandler.log).toHaveBeenCalledWith(
      'Failed to initialize Nostr',
      expect.any(Error),
      expect.objectContaining({
        context: 'AuthStore.login',
      }),
    );
    // ログイン自体は成功する
    expect(useAuthStore.getState().isAuthenticated).toBe(true);
  });

  it('Nostr切断エラーが処理されること', async () => {
    mockDisconnectNostr.mockRejectedValueOnce(new Error('Disconnect failed'));

    setAuthenticatedState();

    await useAuthStore.getState().logout();

    expect(errorHandler.log).toHaveBeenCalledWith(
      'Failed to disconnect Nostr',
      expect.any(Error),
      expect.objectContaining({
        context: 'AuthStore.logout',
      }),
    );
    // ログアウト自体は成功する
    expect(useAuthStore.getState().isAuthenticated).toBe(false);
  });

  describe('initialize', () => {
    beforeEach(() => {
      localStorage.clear();
      vi.clearAllMocks();
    });

    it('初期化時に常に未認証状態になること', async () => {
      // 既存の認証状態を設定
      setAuthenticatedState();

      // initialize実行
      await useAuthStore.getState().initialize();

      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(false);
      expect(state.currentUser).toBeNull();
      expect(state.privateKey).toBeNull();
      expect(state.relayStatus).toEqual([]);
    });

    it('localStorageに保存された状態があっても未認証状態になること', async () => {
      // localStorageに認証状態を保存
      const savedState = {
        state: {
          isAuthenticated: true,
          currentUser: {
            id: 'test123',
            pubkey: 'pubkey123',
            npub: 'npub123',
            name: '保存されたユーザー',
            displayName: '保存されたユーザー',
            picture: 'https://example.com/saved.jpg',
            about: '保存された自己紹介',
            nip05: 'saved@example.com',
          },
        },
      };
      localStorage.setItem('auth-storage', JSON.stringify(savedState));

      // initialize実行
      await useAuthStore.getState().initialize();

      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(false);
      expect(state.currentUser).toBeNull();
      expect(state.privateKey).toBeNull();
    });

    it('SecureStorageのエラーが処理されること', async () => {
      // SecureStorageApiにエラーを発生させる
      const { SecureStorageApi } = await import('@/lib/api/secureStorage');
      (SecureStorageApi.getCurrentAccount as vi.Mock).mockRejectedValueOnce(
        new Error('Storage error'),
      );

      // initialize実行
      await useAuthStore.getState().initialize();

      expect(errorHandler.log).toHaveBeenCalledWith(
        'Failed to initialize auth store',
        expect.any(Error),
        expect.objectContaining({
          context: 'AuthStore.initialize',
        }),
      );

      // エラーがあっても初期状態になること
      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(false);
      expect(state.currentUser).toBeNull();
      expect(state.privateKey).toBeNull();
    });

    it('SecureStorageにアカウントがある場合、自動ログインすること', async () => {
      const mockAccount = {
        npub: 'npub123',
        nsec: 'nsec123',
        pubkey: 'pubkey123',
        metadata: {
          name: 'テストユーザー',
          display_name: 'テストユーザー',
          picture: 'https://example.com/avatar.png',
        },
      };

      const { SecureStorageApi } = await import('@/lib/api/secureStorage');
      (SecureStorageApi.getCurrentAccount as vi.Mock).mockResolvedValueOnce(mockAccount);

      // initialize実行
      await useAuthStore.getState().initialize();

      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(true);
      expect(state.currentUser).not.toBeNull();
      expect(state.currentUser?.npub).toBe('npub123');
      expect(mockInitializeNostr).toHaveBeenCalled();
    });

    it('非クリティカル初期化が停止していても初期化ゲートは短時間で完了すること', async () => {
      const mockAccount = {
        npub: 'npub123',
        nsec: 'nsec123',
        pubkey: 'pubkey123',
        metadata: {
          name: 'テストユーザー',
          display_name: 'テストユーザー',
          picture: 'https://example.com/avatar.png',
        },
      };

      const { SecureStorageApi } = await import('@/lib/api/secureStorage');
      (SecureStorageApi.getCurrentAccount as vi.Mock).mockResolvedValueOnce(mockAccount);
      (SecureStorageApi.listAccounts as vi.Mock).mockImplementationOnce(
        () => new Promise(() => {}),
      );
      mockInitializeNostr.mockImplementationOnce(() => new Promise(() => {}));

      const gateResult = (await Promise.race([
        useAuthStore
          .getState()
          .initialize()
          .then(() => 'resolved' as const),
        new Promise<'timeout'>((resolve) => {
          setTimeout(() => resolve('timeout'), 500);
        }),
      ])) as 'resolved' | 'timeout';

      expect(gateResult).toBe('resolved');
      expect(useAuthStore.getState().isAuthenticated).toBe(true);
      expect(mockInitializeNostr).toHaveBeenCalledTimes(1);
    });
  });
});
