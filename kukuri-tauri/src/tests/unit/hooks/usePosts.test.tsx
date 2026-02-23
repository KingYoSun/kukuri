import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import {
  usePostsByTopic,
  useCreatePost,
  useThreadPosts,
  useTimelinePosts,
  useTopicThreads,
  useTopicTimeline,
} from '@/hooks/usePosts';
import { usePostStore, useAuthStore } from '@/stores';
import { ReactNode } from 'react';
import type { User } from '@/stores';

// Mock Tauri API
vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    getPosts: vi.fn(),
    getTopicTimeline: vi.fn(),
    getThreadPosts: vi.fn(),
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
      avatar: null,
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
          author_npub: 'npub1pubkey1',
          topic_id: 'tech',
          created_at: Math.floor(Date.now() / 1000),
          likes: 5,
          replies: 0,
          boosts: 0,
          is_synced: true,
        },
      ]);

      const { result } = renderHook(() => usePostsByTopic('tech'), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      expect(TauriApi.getPosts).toHaveBeenCalledWith({
        topic_id: 'tech',
        pagination: { limit: 50 },
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
        content: 'created-post',
        author_pubkey: 'pubkey1',
        author_npub: 'npub1pubkey1',
        topic_id: 'tech',
        created_at: Math.floor(Date.now() / 1000),
        likes: 0,
        boosts: 0,
        replies: 0,
        is_synced: true,
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
        const createdPost = state.posts.get('new-post-id');
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
          author_npub: 'npub1pubkey1',
          topic_id: 'general',
          created_at: Math.floor(Date.now() / 1000),
          likes: 10,
          boosts: 0,
          replies: 2,
          is_synced: true,
        },
      ]);

      const { result } = renderHook(() => useTimelinePosts(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      expect(TauriApi.getPosts).toHaveBeenCalledWith({
        pagination: { limit: 50 },
      });

      expect(result.current.data).toHaveLength(1);
      expect(result.current.data?.[0].content).toBe('Timeline post');
    });
  });

  describe('useTopicTimeline', () => {
    it('トピックタイムライン集約を取得できること', async () => {
      const { TauriApi } = await import('@/lib/api/tauri');
      vi.mocked(TauriApi.getTopicTimeline).mockResolvedValue([
        {
          thread_uuid: 'thread-1',
          parent_post: {
            id: 'parent-1',
            content: 'Parent post',
            author_pubkey: 'pubkey1',
            author_npub: 'npub1pubkey1',
            topic_id: 'tech',
            created_at: Math.floor(Date.now() / 1000),
            likes: 3,
            boosts: 0,
            replies: 1,
            is_synced: true,
          },
          first_reply: {
            id: 'reply-1',
            content: 'First reply',
            author_pubkey: 'pubkey2',
            author_npub: 'npub1pubkey2',
            topic_id: 'tech',
            created_at: Math.floor(Date.now() / 1000),
            likes: 1,
            boosts: 0,
            replies: 0,
            is_synced: true,
          },
          reply_count: 1,
          last_activity_at: Math.floor(Date.now() / 1000),
        },
      ]);

      const { result } = renderHook(() => useTopicTimeline('tech'), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      expect(TauriApi.getTopicTimeline).toHaveBeenCalledWith({
        topic_id: 'tech',
        pagination: { limit: 50 },
      });

      expect(result.current.data).toHaveLength(1);
      expect(result.current.data?.[0].threadUuid).toBe('thread-1');
      expect(result.current.data?.[0].parentPost.id).toBe('parent-1');
      expect(result.current.data?.[0].firstReply?.id).toBe('reply-1');
      expect(result.current.data?.[0].replyCount).toBe(1);
      expect(result.current.data?.[0].lastActivityAt).toBeGreaterThan(0);

      const state = usePostStore.getState();
      expect(state.posts.get('parent-1')).toBeDefined();
      expect(state.posts.get('reply-1')).toBeDefined();
    });
  });

  describe('useTopicThreads', () => {
    it('スレッド一覧用フックがタイムライン集約を取得できること', async () => {
      const { TauriApi } = await import('@/lib/api/tauri');
      vi.mocked(TauriApi.getTopicTimeline).mockResolvedValue([
        {
          thread_uuid: 'thread-99',
          parent_post: {
            id: 'parent-99',
            content: 'Root',
            author_pubkey: 'pubkey9',
            author_npub: 'npub1pubkey9',
            topic_id: 'rust',
            created_at: Math.floor(Date.now() / 1000),
            likes: 0,
            boosts: 0,
            replies: 0,
            is_synced: true,
          },
          first_reply: null,
          reply_count: 0,
          last_activity_at: Math.floor(Date.now() / 1000),
        },
      ]);

      const { result } = renderHook(() => useTopicThreads('rust'), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      expect(TauriApi.getTopicTimeline).toHaveBeenCalledWith({
        topic_id: 'rust',
        pagination: { limit: 50 },
      });
      expect(result.current.data?.[0].threadUuid).toBe('thread-99');
    });
  });

  describe('useThreadPosts', () => {
    it('threadUuid を指定してスレッド投稿一覧を取得できること', async () => {
      const { TauriApi } = await import('@/lib/api/tauri');
      vi.mocked(TauriApi.getThreadPosts).mockResolvedValue([
        {
          id: 'thread-post-1',
          content: 'Thread root post',
          author_pubkey: 'pubkey-thread',
          author_npub: 'npub1thread',
          topic_id: 'topic-thread',
          thread_uuid: 'thread-abc',
          thread_root_event_id: 'thread-post-1',
          thread_parent_event_id: null,
          created_at: Math.floor(Date.now() / 1000),
          likes: 0,
          boosts: 0,
          replies: 2,
          is_synced: true,
        },
      ]);

      const { result } = renderHook(() => useThreadPosts('topic-thread', 'thread-abc'), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      expect(TauriApi.getThreadPosts).toHaveBeenCalledWith({
        topic_id: 'topic-thread',
        thread_uuid: 'thread-abc',
        pagination: { limit: 200 },
      });
      expect(result.current.data?.[0].id).toBe('thread-post-1');
      expect(result.current.data?.[0].threadUuid).toBe('thread-abc');
    });
  });
});
