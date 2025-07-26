import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { usePostsByTopic, useCreatePost } from '../usePosts';
import { usePostStore } from '@/stores';
import { ReactNode } from 'react';

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
    usePostStore.setState({
      posts: new Map(),
      postsByTopic: new Map(),
    });
  });

  describe('usePostsByTopic', () => {
    it('投稿取得成功時にpostStoreが更新されること', async () => {
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
});
