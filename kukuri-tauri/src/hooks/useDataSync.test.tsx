import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ReactNode } from 'react';
import { useDataSync } from './useDataSync';
import { usePostStore } from '@/stores/postStore';
import { useTopicStore } from '@/stores/topicStore';
import { useAuthStore } from '@/stores/authStore';

// ストアのモック
vi.mock('@/stores/postStore');
vi.mock('@/stores/topicStore');
vi.mock('@/stores/authStore');

describe('useDataSync', () => {
  let queryClient: QueryClient;
  let mockPostStoreSubscribe: vi.Mock;
  let mockTopicStoreSubscribe: vi.Mock;

  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  beforeEach(() => {
    vi.useFakeTimers();
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });

    // ストアのモックを設定
    mockPostStoreSubscribe = vi.fn().mockReturnValue(vi.fn());
    mockTopicStoreSubscribe = vi.fn().mockReturnValue(vi.fn());

    (usePostStore as any).subscribe = mockPostStoreSubscribe;
    (useTopicStore as any).subscribe = mockTopicStoreSubscribe;
    (useAuthStore as any).mockReturnValue({ isAuthenticated: true });
  });

  afterEach(() => {
    vi.clearAllMocks();
    vi.useRealTimers();
  });

  it('should not setup subscriptions when not authenticated', () => {
    (useAuthStore as any).mockReturnValue({ isAuthenticated: false });

    renderHook(() => useDataSync(), { wrapper });

    expect(mockPostStoreSubscribe).not.toHaveBeenCalled();
    expect(mockTopicStoreSubscribe).not.toHaveBeenCalled();
  });

  it('should setup store subscriptions when authenticated', () => {
    renderHook(() => useDataSync(), { wrapper });

    expect(mockPostStoreSubscribe).toHaveBeenCalledWith(expect.any(Function));
    expect(mockTopicStoreSubscribe).toHaveBeenCalledWith(expect.any(Function));
  });

  it('should update query cache when post store changes', () => {
    renderHook(() => useDataSync(), { wrapper });

    const setQueryDataSpy = vi.spyOn(queryClient, 'setQueryData');
    const postsCallback = mockPostStoreSubscribe.mock.calls[0][0];

    const mockPosts = new Map([
      ['1', { id: '1', content: 'Post 1', created_at: 2 }],
      ['2', { id: '2', content: 'Post 2', created_at: 1 }],
    ]);

    // Zustandのsubscribeコールバックにstateを渡す
    postsCallback({ posts: mockPosts });

    expect(setQueryDataSpy).toHaveBeenCalledWith(['posts'], expect.any(Function));
    const result = setQueryDataSpy.mock.calls[0][1]();
    expect(result.pages[0].posts).toHaveLength(2);
    expect(result.pages[0].posts[0].id).toBe('1'); // より新しい投稿が先
  });

  it('should update query cache when topic store changes', () => {
    renderHook(() => useDataSync(), { wrapper });

    const setQueryDataSpy = vi.spyOn(queryClient, 'setQueryData');
    const topicsCallback = mockTopicStoreSubscribe.mock.calls[0][0];

    const mockTopics = new Map([
      ['topic1', { id: 'topic1', name: 'Topic 1' }],
      ['topic2', { id: 'topic2', name: 'Topic 2' }],
    ]);

    // Zustandのsubscribeコールバックにstateを渡す
    topicsCallback({ topics: mockTopics });

    expect(setQueryDataSpy).toHaveBeenCalledWith(['topics'], expect.any(Function));
    const result = setQueryDataSpy.mock.calls[0][1]();
    expect(result).toHaveLength(2);
  });

  it('should refetch stale queries every 5 minutes', () => {
    renderHook(() => useDataSync(), { wrapper });

    const refetchQueriesSpy = vi.spyOn(queryClient, 'refetchQueries');

    // 5分経過
    vi.advanceTimersByTime(5 * 60 * 1000);

    expect(refetchQueriesSpy).toHaveBeenCalledWith({
      queryKey: ['posts'],
      type: 'active',
      stale: true,
    });
    expect(refetchQueriesSpy).toHaveBeenCalledWith({
      queryKey: ['topics'],
      type: 'active',
      stale: true,
    });
  });

  it('should cleanup subscriptions and intervals on unmount', () => {
    const mockUnsubscribePosts = vi.fn();
    const mockUnsubscribeTopics = vi.fn();
    mockPostStoreSubscribe.mockReturnValue(mockUnsubscribePosts);
    mockTopicStoreSubscribe.mockReturnValue(mockUnsubscribeTopics);

    const { unmount } = renderHook(() => useDataSync(), { wrapper });

    unmount();

    expect(mockUnsubscribePosts).toHaveBeenCalled();
    expect(mockUnsubscribeTopics).toHaveBeenCalled();
  });

  it('should refetch all queries when coming online', () => {
    renderHook(() => useDataSync(), { wrapper });

    const refetchQueriesSpy = vi.spyOn(queryClient, 'refetchQueries');

    // オンラインイベントを発火
    window.dispatchEvent(new Event('online'));

    expect(refetchQueriesSpy).toHaveBeenCalledWith();
  });

  it('should cleanup online event listener on unmount', () => {
    const removeEventListenerSpy = vi.spyOn(window, 'removeEventListener');

    const { unmount } = renderHook(() => useDataSync(), { wrapper });

    unmount();

    expect(removeEventListenerSpy).toHaveBeenCalledWith('online', expect.any(Function));
  });
});
