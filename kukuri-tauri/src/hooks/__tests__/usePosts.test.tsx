import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { usePostsByTopic, useCreatePost, useTimelinePosts } from '../usePosts';
import { usePostStore, useAuthStore } from '@/stores';
import { ReactNode } from 'react';
import type { User } from '@/stores';

// Mock Tauri API
vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    getPosts: vi.fn(),
    createPost: vi.fn(),
  },
}));

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

describe('usePosts hooks', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    usePostStore.setState({
      posts: new Map(),
      postsByTopic: new Map(),
    });
    // デフォルトで認証済みユーザーを設定
    const mockUser: User = {
      id: 'user1',
      pubkey: 'pubkey1',
      npub: 'npub1test',
      name: 'Test User',
      displayName: 'Test User',
      picture: '',
      about: '',
      nip05: '',
    };
    useAuthStore.setState({
      isAuthenticated: true,
      currentUser: mockUser,
      privateKey: 'test-private-key',
    });
  });

  describe('usePostsByTopic', () => {
    it('投稿取得成功時にpostStoreが更新されること', async () => {
      const { TauriApi } = await import('@/lib/api/tauri');
      vi.mocked(TauriApi.getPosts).mockResolvedValue([
        {
          id: '1',
          content: 'Test post',
          author_pubkey: 'pubkey1',
          topic_id: 'tech',
          created_at: Math.floor(Date.now() / 1000),
          likes: 5,
          replies: 0,
        },
      ]);

      const { result } = renderHook(() => usePostsByTopic('tech'), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      const state = usePostStore.getState();
      expect(state.posts.size).toBeGreaterThan(0);
      expect(state.postsByTopic.get('tech')).toBeDefined();
    });

    it('topicIdが空の場合はクエリが実行されないこと', () => {
      const { result } = renderHook(() => usePostsByTopic(''), {
        wrapper: createWrapper(),
      });

      // React Query v5では、enabledがfalseでもisPendingはtrueになることがある
      expect(result.current.fetchStatus).toBe('idle');
      expect(result.current.data).toBeUndefined();
      expect(result.current.isSuccess).toBe(false);
    });
  });

  describe('useCreatePost', () => {
    it('投稿作成成功時にpostStoreに追加されること', async () => {
      const { TauriApi } = await import('@/lib/api/tauri');
      vi.mocked(TauriApi.createPost).mockResolvedValue({
        id: 'new-post-id',
        content: '新しい投稿',
        author_pubkey: 'pubkey1',
        topic_id: 'tech',
        created_at: Math.floor(Date.now() / 1000),
        likes: 0,
        replies: 0,
      });

      const { result } = renderHook(() => useCreatePost(), {
        wrapper: createWrapper(),
      });

      const newPost = {
        content: '新しい投稿',
        topicId: 'tech',
      };

      await result.current.mutateAsync(newPost);

      await waitFor(() => {
        const state = usePostStore.getState();
        const posts = Array.from(state.posts.values());
        const createdPost = posts.find((p) => p.content === '新しい投稿');
        expect(createdPost).toBeDefined();
        expect(createdPost?.author).toBeDefined();
        expect(createdPost?.topicId).toBe('tech');
      });
    });
  });

  describe('useTimelinePosts', () => {
    it('タイムライン投稿が取得できること', async () => {
      const { TauriApi } = await import('@/lib/api/tauri');
      vi.mocked(TauriApi.getPosts).mockResolvedValue([
        {
          id: '1',
          content: 'Timeline post',
          author_pubkey: 'pubkey1',
          topic_id: 'general',
          created_at: Math.floor(Date.now() / 1000),
          likes: 10,
          replies: 2,
        },
      ]);

      const { result } = renderHook(() => useTimelinePosts(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      expect(result.current.data).toHaveLength(1);
      expect(result.current.data?.[0].content).toBe('Timeline post');
    });
  });
});
