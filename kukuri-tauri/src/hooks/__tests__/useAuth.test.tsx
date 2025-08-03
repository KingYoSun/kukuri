import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useAuth, useLogout } from '../useAuth';
import { useAuthStore, useTopicStore } from '@/stores';
import { ReactNode } from 'react';

// TauriApiをモック
vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    logout: vi.fn().mockResolvedValue(undefined),
    login: vi.fn().mockResolvedValue({
      public_key: 'pubkey123',
      npub: 'npub123',
    }),
    generateKeypair: vi.fn().mockResolvedValue({
      public_key: 'pubkey123',
      nsec: 'nsec123',
    }),
    getTopics: vi.fn().mockResolvedValue([]),
  },
}));

// SecureStorageApiをモック
vi.mock('@/lib/api/secureStorage', () => ({
  SecureStorageApi: {
    addAccount: vi.fn().mockResolvedValue(undefined),
    getAccounts: vi.fn().mockResolvedValue([]),
    getCurrentAccount: vi.fn().mockResolvedValue(null),
  },
}));

// topicStoreをモック
vi.mock('@/stores', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/stores')>();
  return {
    ...actual,
    useTopicStore: {
      getState: () => ({
        topics: new Map(),
        fetchTopics: vi.fn().mockResolvedValue(undefined),
        joinTopic: vi.fn().mockResolvedValue(undefined),
        setCurrentTopic: vi.fn(),
      }),
    },
  };
});

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
};

describe('useAuth hooks', () => {
  beforeEach(() => {
    useAuthStore.setState({
      isAuthenticated: false,
      currentUser: null,
      privateKey: null,
    });
  });

  describe('useAuth', () => {
    it('ログイン成功時にauthStoreが更新されること', async () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: createWrapper(),
      });

      await result.current.loginWithNsec('test-private-key');

      await waitFor(() => {
        const state = useAuthStore.getState();
        expect(state.isAuthenticated).toBe(true);
        expect(state.currentUser).not.toBeNull();
        expect(state.privateKey).toBe('test-private-key');
      });
    });
  });

  describe('useAuth - generateNewKeypair', () => {
    it('鍵ペア生成成功時にauthStoreが更新されること', async () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: createWrapper(),
      });

      await result.current.generateNewKeypair();

      await waitFor(() => {
        const state = useAuthStore.getState();
        expect(state.isAuthenticated).toBe(true);
        expect(state.currentUser).not.toBeNull();
        expect(state.privateKey).toMatch(/^nsec/);
      });
    });
  });

  describe('useLogout', () => {
    it('ログアウト時にauthStoreがクリアされること', async () => {
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

      const { result } = renderHook(() => useLogout(), {
        wrapper: createWrapper(),
      });

      result.current();

      await waitFor(() => {
        const state = useAuthStore.getState();
        expect(state.isAuthenticated).toBe(false);
        expect(state.currentUser).toBeNull();
        expect(state.privateKey).toBeNull();
      });
    });
  });
});
