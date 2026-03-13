import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import {
  useTopics,
  useTopic,
  useCreateTopic,
  useUpdateTopic,
  useDeleteTopic,
} from '@/hooks/useTopics';
import { useTopicStore } from '@/stores';
import { ReactNode } from 'react';
import { DEFAULT_PUBLIC_TOPIC_ID } from '@/constants/topics';

// TauriAPIのモチE�E��E�
vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    getTopics: vi.fn().mockResolvedValue([
      {
        id: 'tech',
        name: 'technology',
        description: '技術�E般につぁE�E��E�議論するトピック',
        created_at: Math.floor(Date.now() / 1000),
        updated_at: Math.floor(Date.now() / 1000),
      },
      {
        id: 'nostr',
        name: 'nostr',
        description: 'NostrプロトコルにつぁE�E��E�',
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
      name: 'new topic',
      description: 'new topic description',
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
    useTopicStore.setState((state) => {
      state.topics = new Map();
      state.currentTopic = null;
      state.joinedTopics = [];
      state.refreshPendingTopics = vi.fn().mockResolvedValue(undefined);
      return state;
    });
  });

  describe('useTopics', () => {
    it('トピチE�E��E�取得�E功時にtopicStoreが更新されること', async () => {
      const { result } = renderHook(() => useTopics(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      const state = useTopicStore.getState();
      expect(state.topics.size).toBe(3);
      expect(state.topics.has('tech')).toBe(true);
      expect(state.topics.has('nostr')).toBe(true);
      expect(state.topics.has(DEFAULT_PUBLIC_TOPIC_ID)).toBe(true);

      const techTopic = state.topics.get('tech');
      expect(techTopic?.name).toBe('technology');
      expect(techTopic?.description).toBe('技術�E般につぁE�E��E�議論するトピック');
    });

    it('チE�E�Eタがフロントエンド�E型に正しく変換されること', async () => {
      const { result } = renderHook(() => useTopics(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
        expect(result.current.data?.length).toBeGreaterThan(0);
      });

      const topics = result.current.data!;
      const techTopic = topics.find((topic) => topic.id === 'tech');
      expect(techTopic).toBeDefined();
      expect(techTopic).toMatchObject({
        id: 'tech',
        name: 'technology',
        tags: [],
        memberCount: 100,
        postCount: 500,
        isActive: true,
      });
      expect(techTopic?.createdAt).toBeInstanceOf(Date);
    });
  });

  describe('useTopic', () => {
    it('ストアにキャチE�E��E�ュがある場合�Eそれを返すこと', async () => {
      const cachedTopic = {
        id: 'cached',
        name: 'cached topic',
        description: 'cached topic description',
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

    it('ストアになぁE�E��E�合�EAPIから取得すること', async () => {
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
    it('トピチE�E��E�作�Eが�E功すること', async () => {
      const mockCreateTopic = vi.spyOn(useTopicStore.getState(), 'createTopic');

      const { result } = renderHook(() => useCreateTopic(), {
        wrapper: createWrapper(),
      });

      await result.current.mutateAsync({
        name: 'new topic',
        description: 'new topic description',
      });

      expect(mockCreateTopic).toHaveBeenCalledWith('new topic', 'new topic description');
    });
  });

  describe('useUpdateTopic', () => {
    it('トピチE�E��E�更新が�E功すること', async () => {
      const mockUpdateTopic = vi.spyOn(useTopicStore.getState(), 'updateTopicRemote');

      const { result } = renderHook(() => useUpdateTopic(), {
        wrapper: createWrapper(),
      });

      await result.current.mutateAsync({
        id: 'tech',
        name: 'updated topic',
        description: 'updated description',
      });

      expect(mockUpdateTopic).toHaveBeenCalledWith('tech', 'updated topic', 'updated description');
    });
  });

  describe('useDeleteTopic', () => {
    it('トピチE�E��E�削除が�E功すること', async () => {
      const mockDeleteTopic = vi.spyOn(useTopicStore.getState(), 'deleteTopicRemote');

      const { result } = renderHook(() => useDeleteTopic(), {
        wrapper: createWrapper(),
      });

      await result.current.mutateAsync('tech');

      expect(mockDeleteTopic).toHaveBeenCalledWith('tech');
    });
  });
});
