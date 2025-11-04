import { describe, it, expect, beforeEach, vi } from 'vitest';
import { act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import {
  useTrendingTopicsQuery,
  useTrendingPostsQuery,
  useFollowingFeedQuery,
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

  beforeEach(() => {
    vi.clearAllMocks();
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
    });
  });
});
