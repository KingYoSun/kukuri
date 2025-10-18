import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useTopics, useTopic, useCreateTopic, useUpdateTopic, useDeleteTopic } from '../useTopics';
import { useTopicStore } from '@/stores';
import { ReactNode } from 'react';

// TauriAPIのモック
vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    getTopics: vi.fn().mockResolvedValue([
      {
        id: 'tech',
        name: 'technology',
        description: '技術全般について議論するトピック',
        created_at: Math.floor(Date.now() / 1000),
        updated_at: Math.floor(Date.now() / 1000),
      },
      {
        id: 'nostr',
        name: 'nostr',
        description: 'Nostrプロトコルについて',
        created_at: Math.floor(Date.now() / 1000),
        updated_at: Math.floor(Date.now() / 1000),
      },
    ]),
    getTopicStats: vi.fn().mockResolvedValue({
      topic_id: 'tech',
      member_count: 100,
      post_count: 500,
      active_users_24h: 80,
      trending_score: 420.0,
    }),
    createTopic: vi.fn().mockResolvedValue({
      id: 'new-topic',
      name: '新しいトピック',
      description: '新しいトピックの説明',
      created_at: Math.floor(Date.now() / 1000),
      updated_at: Math.floor(Date.now() / 1000),
    }),
    updateTopic: vi.fn().mockResolvedValue(undefined),
    deleteTopic: vi.fn().mockResolvedValue(undefined),
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

describe('useTopics hooks', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useTopicStore.setState({
      topics: new Map(),
      currentTopic: null,
      joinedTopics: [],
    });
  });

  describe('useTopics', () => {
    it('トピック取得成功時にtopicStoreが更新されること', async () => {
      const { result } = renderHook(() => useTopics(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      const state = useTopicStore.getState();
      expect(state.topics.size).toBe(2);
      expect(state.topics.has('tech')).toBe(true);
      expect(state.topics.has('nostr')).toBe(true);

      const techTopic = state.topics.get('tech');
      expect(techTopic?.name).toBe('technology');
      expect(techTopic?.description).toBe('技術全般について議論するトピック');
    });

    it('データがフロントエンドの型に正しく変換されること', async () => {
      const { result } = renderHook(() => useTopics(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(result.current.data).toBeDefined();
      });

      const topics = result.current.data!;
      expect(topics[0]).toMatchObject({
        id: 'tech',
        name: 'technology',
        description: '技術全般について議論するトピック',
        tags: [],
        memberCount: 100,
        postCount: 500,
        isActive: true,
      });
      expect(topics[0].createdAt).toBeInstanceOf(Date);
    });
  });

  describe('useTopic', () => {
    it('ストアにキャッシュがある場合はそれを返すこと', async () => {
      const cachedTopic = {
        id: 'cached',
        name: 'キャッシュされたトピック',
        description: 'キャッシュテスト',
        tags: [],
        memberCount: 10,
        postCount: 50,
        lastActive: Date.now() / 1000,
        isActive: true,
        createdAt: new Date(),
      };

      useTopicStore.setState({
        topics: new Map([['cached', cachedTopic]]),
      });

      const { result } = renderHook(() => useTopic('cached'), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      expect(result.current.data).toEqual(cachedTopic);
    });

    it('ストアにない場合はAPIから取得すること', async () => {
      const { result } = renderHook(() => useTopic('tech'), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      expect(result.current.data?.id).toBe('tech');
      expect(result.current.data?.name).toBe('technology');
    });
  });

  describe('useCreateTopic', () => {
    it('トピック作成が成功すること', async () => {
      const mockCreateTopic = vi.spyOn(useTopicStore.getState(), 'createTopic');

      const { result } = renderHook(() => useCreateTopic(), {
        wrapper: createWrapper(),
      });

      await result.current.mutateAsync({
        name: '新しいトピック',
        description: '新しいトピックの説明',
      });

      expect(mockCreateTopic).toHaveBeenCalledWith('新しいトピック', '新しいトピックの説明');
    });
  });

  describe('useUpdateTopic', () => {
    it('トピック更新が成功すること', async () => {
      const mockUpdateTopic = vi.spyOn(useTopicStore.getState(), 'updateTopicRemote');

      const { result } = renderHook(() => useUpdateTopic(), {
        wrapper: createWrapper(),
      });

      await result.current.mutateAsync({
        id: 'tech',
        name: '更新されたトピック',
        description: '更新された説明',
      });

      expect(mockUpdateTopic).toHaveBeenCalledWith('tech', '更新されたトピック', '更新された説明');
    });
  });

  describe('useDeleteTopic', () => {
    it('トピック削除が成功すること', async () => {
      const mockDeleteTopic = vi.spyOn(useTopicStore.getState(), 'deleteTopicRemote');

      const { result } = renderHook(() => useDeleteTopic(), {
        wrapper: createWrapper(),
      });

      await result.current.mutateAsync('tech');

      expect(mockDeleteTopic).toHaveBeenCalledWith('tech');
    });
  });
});
