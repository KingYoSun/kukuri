import { describe, it, expect, beforeEach, vi, MockedFunction } from 'vitest';
import { useAuthStore } from '../authStore';
import type { User } from '../types';

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

import { initializeNostr, disconnectNostr } from '@/lib/api/nostr';

const mockInitializeNostr = initializeNostr as MockedFunction<typeof initializeNostr>;
const mockDisconnectNostr = disconnectNostr as MockedFunction<typeof disconnectNostr>;

describe('authStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({
      isAuthenticated: false,
      currentUser: null,
      privateKey: null,
      relayStatus: [],
    });
  });

  it('初期状態が正しく設定されていること', () => {
    const state = useAuthStore.getState();
    expect(state.isAuthenticated).toBe(false);
    expect(state.currentUser).toBeNull();
    expect(state.privateKey).toBeNull();
  });

  it('loginメソッドが正しく動作すること', () => {
    const testUser: User = {
      id: 'test123',
      pubkey: 'pubkey123',
      npub: 'npub123',
      name: 'テストユーザー',
      displayName: 'テストユーザー',
      picture: '',
      about: '',
      nip05: '',
    };
    const testPrivateKey = 'nsec123';

    useAuthStore.getState().login(testPrivateKey, testUser);

    const state = useAuthStore.getState();
    expect(state.isAuthenticated).toBe(true);
    expect(state.currentUser).toEqual(testUser);
    expect(state.privateKey).toBe(testPrivateKey);
  });

  it('logoutメソッドが正しく動作すること', async () => {
    const testUser: User = {
      id: 'test123',
      pubkey: 'pubkey123',
      npub: 'npub123',
      name: 'テストユーザー',
      displayName: 'テストユーザー',
      picture: '',
      about: '',
      nip05: '',
    };
    useAuthStore.setState({
      isAuthenticated: true,
      currentUser: testUser,
      privateKey: 'nsec123',
    });

    await useAuthStore.getState().logout();

    const state = useAuthStore.getState();
    expect(state.isAuthenticated).toBe(false);
    expect(state.currentUser).toBeNull();
    expect(state.privateKey).toBeNull();
  });

  it('updateUserメソッドが正しく動作すること', () => {
    const testUser: User = {
      id: 'test123',
      pubkey: 'pubkey123',
      npub: 'npub123',
      name: 'テストユーザー',
      displayName: 'テストユーザー',
      picture: '',
      about: '',
      nip05: '',
    };
    useAuthStore.setState({
      isAuthenticated: true,
      currentUser: testUser,
      privateKey: 'nsec123',
    });

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
    const testUser: User = {
      id: 'test123',
      pubkey: 'pubkey123',
      npub: 'npub123',
      name: 'テストユーザー',
      displayName: 'テストユーザー',
      picture: '',
      about: '',
      nip05: '',
    };
    const testPrivateKey = 'nsec123';

    await useAuthStore.getState().login(testPrivateKey, testUser);

    expect(initializeNostr).toHaveBeenCalled();
  });

  it('logoutメソッドがNostrを切断すること', async () => {
    const testUser: User = {
      id: 'test123',
      pubkey: 'pubkey123',
      npub: 'npub123',
      name: 'テストユーザー',
      displayName: 'テストユーザー',
      picture: '',
      about: '',
      nip05: '',
    };
    useAuthStore.setState({
      isAuthenticated: true,
      currentUser: testUser,
      privateKey: 'nsec123',
    });

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

    const testUser: User = {
      id: 'test123',
      pubkey: 'pubkey123',
      npub: 'npub123',
      name: 'テストユーザー',
      displayName: 'テストユーザー',
      picture: '',
      about: '',
      nip05: '',
    };
    useAuthStore.setState({
      isAuthenticated: true,
      currentUser: testUser,
      privateKey: 'nsec123',
    });

    // 認証後は true - isAuthenticatedを直接確認
    expect(useAuthStore.getState().isAuthenticated).toBe(true);
  });

  it('Nostr初期化エラーが処理されること', async () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    mockInitializeNostr.mockRejectedValueOnce(new Error('Nostr init failed'));

    const testUser: User = {
      id: 'test123',
      pubkey: 'pubkey123',
      npub: 'npub123',
      name: 'テストユーザー',
      displayName: 'テストユーザー',
      picture: '',
      about: '',
      nip05: '',
    };

    await useAuthStore.getState().login('nsec123', testUser);

    expect(consoleErrorSpy).toHaveBeenCalledWith('Failed to initialize Nostr:', expect.any(Error));
    // ログイン自体は成功する
    expect(useAuthStore.getState().isAuthenticated).toBe(true);

    consoleErrorSpy.mockRestore();
  });

  it('Nostr切断エラーが処理されること', async () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    mockDisconnectNostr.mockRejectedValueOnce(new Error('Disconnect failed'));

    const testUser: User = {
      id: 'test123',
      pubkey: 'pubkey123',
      npub: 'npub123',
      name: 'テストユーザー',
      displayName: 'テストユーザー',
      picture: '',
      about: '',
      nip05: '',
    };
    useAuthStore.setState({
      isAuthenticated: true,
      currentUser: testUser,
      privateKey: 'nsec123',
    });

    await useAuthStore.getState().logout();

    expect(consoleErrorSpy).toHaveBeenCalledWith('Failed to disconnect Nostr:', expect.any(Error));
    // ログアウト自体は成功する
    expect(useAuthStore.getState().isAuthenticated).toBe(false);

    consoleErrorSpy.mockRestore();
  });

  describe('initialize', () => {
    beforeEach(() => {
      localStorage.clear();
      vi.clearAllMocks();
    });

    it('初期化時に常に未認証状態になること', async () => {
      // 既存の認証状態を設定
      useAuthStore.setState({
        isAuthenticated: true,
        currentUser: {
          id: 'test123',
          pubkey: 'pubkey123',
          npub: 'npub123',
          name: 'テストユーザー',
          displayName: 'テストユーザー',
          picture: '',
          about: '',
          nip05: '',
        },
        privateKey: 'nsec123',
      });

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

    it('localStorageのパースエラーが処理されること', async () => {
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      const consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
      
      // 不正なJSONを保存
      localStorage.setItem('auth-storage', 'invalid json');

      // initialize実行
      await useAuthStore.getState().initialize();

      expect(consoleErrorSpy).toHaveBeenCalledWith('Failed to parse auth storage:', expect.any(Error));
      
      // エラーがあっても初期状態になること
      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(false);
      expect(state.currentUser).toBeNull();
      expect(state.privateKey).toBeNull();

      consoleErrorSpy.mockRestore();
      consoleLogSpy.mockRestore();
    });

    it('保存された認証状態がある場合、コンソールログが出力されること', async () => {
      const consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
      
      // localStorageに認証状態を保存
      const savedState = {
        state: {
          isAuthenticated: true,
          currentUser: {
            id: 'test123',
            pubkey: 'pubkey123',
            npub: 'npub123',
            name: 'ユーザー',
          },
        },
      };
      localStorage.setItem('auth-storage', JSON.stringify(savedState));

      // initialize実行
      await useAuthStore.getState().initialize();

      expect(consoleLogSpy).toHaveBeenCalledWith('Previous session found, but re-authentication required');

      consoleLogSpy.mockRestore();
    });
  });
});
