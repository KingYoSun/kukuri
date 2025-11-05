import { describe, it, expect, beforeEach, vi } from 'vitest';
import { act } from '@testing-library/react';
import { QueryClient, QueryClientProvider, type InfiniteData } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import {
  useTrendingTopicsQuery,
  useTrendingPostsQuery,
  useFollowingFeedQuery,
  prefetchTrendingCategory,
  prefetchFollowingCategory,
  trendingTopicsQueryKey,
  trendingPostsQueryKey,
  followingFeedQueryKey,
  type FollowingFeedPageResult,
} from '@/hooks/useTrendingFeeds';

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    listTrendingTopics: vi.fn(),
    listTrendingPosts: vi.fn(),
    listFollowingFeed: vi.fn(),
  },
}));

vi.mock('@/lib/posts/postMapper', () => ({
  mapPostResponseToDomain: vi.fn(async (post: any) => ({
    id: post.id,
    content: post.content,
    author: {
      id: post.author_pubkey,
      pubkey: post.author_pubkey,
      npub: post.author_npub,
      name: '',
      displayName: '',
      picture: '',
      about: '',
      nip05: '',
      avatar: null,
    },
    topicId: post.topic_id,
    created_at: post.created_at,
    tags: [],
    likes: post.likes ?? 0,
    boosts: post.boosts ?? 0,
    replies: [],
    isSynced: post.is_synced ?? true,
  })),
}));

vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
  },
}));

describe('useTrendingFeeds hooks', () => {
  const createQueryClient = () =>
    new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });

  const createWrapper = () => {
    const queryClient = createQueryClient();

    return ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
  };

  beforeEach(async () => {
    vi.clearAllMocks();
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.listTrendingTopics).mockReset();
    vi.mocked(TauriApi.listTrendingPosts).mockReset();
    vi.mocked(TauriApi.listFollowingFeed).mockReset();
  });

  it('fetches trending topics successfully', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.listTrendingTopics).mockResolvedValue({
      generated_at: 1_700_000_000,
      topics: [
        {
          topic_id: 'topic-1',
          name: 'Topic One',
          description: 'desc',
          member_count: 12,
          post_count: 30,
          trending_score: 24.0,
          rank: 1,
          score_change: null,
        },
      ],
    });

    const wrapper = createWrapper();
    const { result } = renderHook(() => useTrendingTopicsQuery(5), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(TauriApi.listTrendingTopics).toHaveBeenCalledWith(5);
    expect(result.current.data?.topics[0].topicId).toBe('topic-1');
    expect(result.current.data?.topics[0].trendingScore).toBe(24);
  });

  it('fetches trending posts for topics', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.listTrendingPosts).mockResolvedValue({
      generated_at: 1_700_000_100,
      topics: [
        {
          topic_id: 'topic-1',
          topic_name: 'Topic One',
          relative_rank: 1,
          posts: [
            {
              id: 'post-1',
              content: 'hello',
              author_pubkey: 'author-1',
              author_npub: 'npub1author',
              topic_id: 'topic-1',
              created_at: 1_700_000_000,
              likes: 10,
              boosts: 0,
              replies: 0,
              is_synced: true,
            },
          ],
        },
      ],
    });

    const wrapper = createWrapper();
    const { result } = renderHook(() => useTrendingPostsQuery(['topic-1'], 3), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(TauriApi.listTrendingPosts).toHaveBeenCalledWith({
      topicIds: ['topic-1'],
      perTopic: 3,
    });
    expect(result.current.data?.topics[0].posts[0].id).toBe('post-1');
  });

  it('paginates following feed', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.listFollowingFeed)
      .mockResolvedValueOnce({
        items: [
          {
            id: 'post-1',
            content: 'hello',
            author_pubkey: 'author-1',
            author_npub: 'npub1author',
            topic_id: 'topic-1',
            created_at: 1_700_000_000,
            likes: 2,
            boosts: 0,
            replies: 0,
            is_synced: true,
          },
        ],
        next_cursor: 'cursor-1',
        has_more: true,
        server_time: 1_700_000_000,
      })
      .mockResolvedValueOnce({
        items: [
          {
            id: 'post-2',
            content: 'second',
            author_pubkey: 'author-2',
            author_npub: 'npub1author2',
            topic_id: 'topic-2',
            created_at: 1_700_000_500,
            likes: 5,
            boosts: 0,
            replies: 0,
            is_synced: true,
          },
        ],
        next_cursor: null,
        has_more: false,
        server_time: 1_700_000_500,
      });

    const wrapper = createWrapper();
    const { result } = renderHook(() => useFollowingFeedQuery({ limit: 1 }), {
      wrapper,
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
      expect(result.current.data?.pages[0].items.length).toBe(1);
    });

    expect(TauriApi.listFollowingFeed).toHaveBeenLastCalledWith({
      cursor: null,
      limit: 1,
      includeReactions: false,
    });

    await act(async () => {
      await result.current.fetchNextPage();
    });

    await waitFor(() => {
      expect(result.current.data?.pages.length).toBe(2);
      expect(result.current.data?.pages[1].items[0].id).toBe('post-2');
    });

    expect(TauriApi.listFollowingFeed).toHaveBeenLastCalledWith({
      cursor: 'cursor-1',
      limit: 1,
      includeReactions: false,
    });
  });

  it('prefetches trending category data', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    const queryClient = createQueryClient();

    vi.mocked(TauriApi.listTrendingTopics).mockResolvedValue({
      generated_at: 1_700_000_000,
      topics: [
        {
          topic_id: 'topic-1',
          name: 'Topic One',
          description: 'desc',
          member_count: 12,
          post_count: 30,
          trending_score: 24.0,
          rank: 1,
          score_change: null,
        },
      ],
    });
    vi.mocked(TauriApi.listTrendingPosts).mockResolvedValue({
      generated_at: 1_700_000_100,
      topics: [
        {
          topic_id: 'topic-1',
          topic_name: 'Topic One',
          relative_rank: 1,
          posts: [
            {
              id: 'post-1',
              content: 'prefetch',
              author_pubkey: 'author-1',
              author_npub: 'npub1author',
              topic_id: 'topic-1',
              created_at: 1_700_000_050,
              likes: 3,
              boosts: 0,
              replies: 0,
              is_synced: true,
            },
          ],
        },
      ],
    });

    await prefetchTrendingCategory(queryClient, { topicsLimit: 2, postsPerTopic: 2 });

    expect(TauriApi.listTrendingTopics).toHaveBeenCalledWith(2);
    const topicsCache = queryClient.getQueryData(trendingTopicsQueryKey(2));
    expect(topicsCache).toBeDefined();

    const postsCache = queryClient.getQueryData(trendingPostsQueryKey(['topic-1'], 2));
    expect(postsCache).toBeDefined();
  });

  it('prefetches following category data', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    const queryClient = createQueryClient();

    vi.mocked(TauriApi.listFollowingFeed).mockResolvedValue({
      items: [
        {
          id: 'post-1',
          content: 'hello',
          author_pubkey: 'author-1',
          author_npub: 'npub1author',
          topic_id: 'topic-1',
          created_at: 1_700_000_000,
          likes: 2,
          boosts: 0,
          replies: 0,
          is_synced: true,
        },
      ],
      next_cursor: null,
      has_more: false,
      server_time: 1_700_000_000,
    });

    await prefetchFollowingCategory(queryClient, { limit: 5, includeReactions: false });

    expect(TauriApi.listFollowingFeed).toHaveBeenCalledWith({
      cursor: null,
      limit: 5,
      includeReactions: false,
    });

    const cached = queryClient.getQueryData<InfiniteData<FollowingFeedPageResult>>(
      followingFeedQueryKey(5, false),
    );
    expect(cached).toBeDefined();
    expect(cached?.pages[0].items[0].id).toBe('post-1');
  });
});
