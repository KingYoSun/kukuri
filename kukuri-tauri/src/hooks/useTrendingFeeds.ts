import {
  QueryClient,
  useInfiniteQuery,
  useQuery,
  useQueryClient,
  type InfiniteData,
} from '@tanstack/react-query';
import { TauriApi, type FollowingFeedPage, type ListFollowingFeedParams } from '@/lib/api/tauri';
import { mapPostResponseToDomain } from '@/lib/posts/postMapper';
import type { Post } from '@/stores';
import { errorHandler } from '@/lib/errorHandler';

export interface TrendingTopicSummary {
  topicId: string;
  name: string;
  description: string | null;
  memberCount: number;
  postCount: number;
  trendingScore: number;
  rank: number;
  scoreChange: number | null;
}

export interface TrendingTopicsResult {
  generatedAt: number;
  topics: TrendingTopicSummary[];
}

export interface TrendingPostsTopic {
  topicId: string;
  topicName: string;
  relativeRank: number;
  posts: Post[];
}

export interface TrendingPostsResult {
  generatedAt: number;
  topics: TrendingPostsTopic[];
}

export interface FollowingFeedPageResult {
  cursor: string | null;
  items: Post[];
  nextCursor: string | null;
  hasMore: boolean;
  serverTime: number;
}

interface TrendingPostsQueryOptions {
  enabled?: boolean;
}

interface FollowingFeedQueryOptions {
  limit?: number;
  includeReactions?: boolean;
}

const TRENDING_STALE_TIME = 60_000;
const TRENDING_REFETCH_INTERVAL = 120_000;
const FOLLOWING_FEED_STALE_TIME = 60_000;

export const trendingTopicsQueryKey = (limit = 10) => ['trending', 'topics', limit] as const;

export const trendingPostsQueryKey = (topicIds: string[], perTopic: number) =>
  ['trending', 'posts', { topicIds, perTopic }] as const;

export const followingFeedQueryKey = (limit: number, includeReactions: boolean) =>
  ['followingFeed', { limit, includeReactions }] as const;

async function fetchTrendingTopics(limit = 10): Promise<TrendingTopicsResult> {
  try {
    const response = await TauriApi.listTrendingTopics(limit);
    return {
      generatedAt: response.generated_at,
      topics: response.topics.map((topic) => ({
        topicId: topic.topic_id,
        name: topic.name,
        description: topic.description,
        memberCount: topic.member_count,
        postCount: topic.post_count,
        trendingScore: topic.trending_score,
        rank: topic.rank,
        scoreChange: topic.score_change ?? null,
      })),
    };
  } catch (error) {
    errorHandler.log('TrendingTopics.fetchFailed', error, {
      context: 'fetchTrendingTopics',
      metadata: { limit },
    });
    throw error;
  }
}

async function fetchTrendingPosts(topicIds: string[], perTopic = 3): Promise<TrendingPostsResult> {
  if (topicIds.length === 0) {
    return { generatedAt: Date.now(), topics: [] };
  }

  try {
    const response = await TauriApi.listTrendingPosts({
      topicIds,
      perTopic,
    });

    const topics = await Promise.all(
      response.topics.map(async (topic) => ({
        topicId: topic.topic_id,
        topicName: topic.topic_name,
        relativeRank: topic.relative_rank,
        posts: await Promise.all(topic.posts.map((post) => mapPostResponseToDomain(post))),
      })),
    );

    return {
      generatedAt: response.generated_at,
      topics,
    };
  } catch (error) {
    errorHandler.log('TrendingPosts.fetchFailed', error, {
      context: 'fetchTrendingPosts',
      metadata: { topicIds, perTopic },
    });
    throw error;
  }
}

async function fetchFollowingFeedPage(
  params: ListFollowingFeedParams,
): Promise<FollowingFeedPageResult> {
  try {
    const response: FollowingFeedPage = await TauriApi.listFollowingFeed(params);
    const posts = await Promise.all(response.items.map((item) => mapPostResponseToDomain(item)));

    return {
      cursor: params.cursor ?? null,
      items: posts,
      nextCursor: response.next_cursor,
      hasMore: response.has_more,
      serverTime: response.server_time,
    };
  } catch (error) {
    errorHandler.log('FollowingFeed.fetchFailed', error, {
      context: 'fetchFollowingFeedPage',
      metadata: { params },
    });
    throw error;
  }
}

export const useTrendingTopicsQuery = (limit = 10) =>
  useQuery<TrendingTopicsResult, Error>({
    queryKey: trendingTopicsQueryKey(limit),
    queryFn: () => fetchTrendingTopics(limit),
    staleTime: TRENDING_STALE_TIME,
    refetchInterval: TRENDING_REFETCH_INTERVAL,
    retry: 1,
  });

export const useTrendingPostsQuery = (
  topicIds: string[],
  perTopic = 3,
  options?: TrendingPostsQueryOptions,
) =>
  useQuery<TrendingPostsResult, Error>({
    queryKey: trendingPostsQueryKey(topicIds, perTopic),
    enabled: options?.enabled ?? topicIds.length > 0,
    queryFn: () => fetchTrendingPosts(topicIds, perTopic),
    staleTime: TRENDING_STALE_TIME,
    refetchInterval: TRENDING_REFETCH_INTERVAL,
    retry: 1,
  });

export const useFollowingFeedQuery = (options: FollowingFeedQueryOptions = {}) => {
  const queryClient = useQueryClient();
  const isE2E =
    typeof window !== 'undefined' &&
    Boolean((window as unknown as { __KUKURI_E2E__?: boolean }).__KUKURI_E2E__);
  const limit = options.limit ?? 20;
  const includeReactions = options.includeReactions ?? false;
  const queryKey = followingFeedQueryKey(limit, includeReactions);

  if (isE2E) {
    return useInfiniteQuery<
      FollowingFeedPageResult,
      Error,
      InfiniteData<FollowingFeedPageResult>,
      ReturnType<typeof followingFeedQueryKey>,
      string | null
    >({
      queryKey,
      initialPageParam: null,
      staleTime: 10 * 60 * 1000,
      gcTime: 15 * 60 * 1000,
      retry: false,
      refetchOnMount: false,
      refetchOnReconnect: false,
      refetchInterval: false,
      networkMode: 'offlineFirst',
      getNextPageParam: () => undefined,
      queryFn: async () => {
        const cached = queryClient.getQueryData<InfiniteData<FollowingFeedPageResult>>(queryKey);
        const fallbackPages =
          queryClient
            .getQueriesData<InfiniteData<FollowingFeedPageResult>>({ queryKey: ['followingFeed'] })
            .map(([, data]) => data?.pages ?? [])
            .reduce<FollowingFeedPageResult[]>((acc, pages) => acc.concat(pages), []) ?? [];
        const resolved = cached?.pages?.[0] ?? fallbackPages.find(Boolean);
        if (resolved) {
          return resolved;
        }
        return {
          cursor: null,
          items: [],
          nextCursor: null,
          hasMore: false,
          serverTime: Date.now(),
        };
      },
    });
  }

  return useInfiniteQuery<
    FollowingFeedPageResult,
    Error,
    InfiniteData<FollowingFeedPageResult>,
    ReturnType<typeof followingFeedQueryKey>,
    string | null
  >({
    queryKey,
    initialPageParam: null,
    getNextPageParam: (lastPage) => lastPage.nextCursor ?? undefined,
    staleTime: FOLLOWING_FEED_STALE_TIME,
    retry: 1,
    queryFn: ({ pageParam }) =>
      fetchFollowingFeedPage({
        cursor: pageParam ?? null,
        limit,
        includeReactions,
      }),
  });
};

export interface PrefetchTrendingCategoryOptions {
  topicsLimit?: number;
  postsPerTopic?: number;
}

export const prefetchTrendingCategory = async (
  queryClient: QueryClient,
  options: PrefetchTrendingCategoryOptions = {},
) => {
  const topicsLimit = options.topicsLimit ?? 10;
  const postsPerTopic = options.postsPerTopic ?? 3;

  try {
    const topics = await queryClient.fetchQuery({
      queryKey: trendingTopicsQueryKey(topicsLimit),
      queryFn: () => fetchTrendingTopics(topicsLimit),
      staleTime: TRENDING_STALE_TIME,
    });

    const topicIds = topics.topics.map((topic) => topic.topicId);
    if (topicIds.length === 0) {
      return;
    }

    await queryClient.prefetchQuery({
      queryKey: trendingPostsQueryKey(topicIds, postsPerTopic),
      queryFn: () => fetchTrendingPosts(topicIds, postsPerTopic),
      staleTime: TRENDING_STALE_TIME,
    });
  } catch (error) {
    errorHandler.log('Sidebar.prefetchFailed', error, {
      context: 'Sidebar.prefetchTrendingCategory',
      metadata: { category: 'trending', topicsLimit, postsPerTopic },
    });
  }
};

export interface PrefetchFollowingCategoryOptions {
  limit?: number;
  includeReactions?: boolean;
}

export const prefetchFollowingCategory = async (
  queryClient: QueryClient,
  options: PrefetchFollowingCategoryOptions = {},
) => {
  const limit = options.limit ?? 10;
  const includeReactions = options.includeReactions ?? true;

  try {
    await queryClient.prefetchInfiniteQuery<
      FollowingFeedPageResult,
      Error,
      InfiniteData<FollowingFeedPageResult>,
      ReturnType<typeof followingFeedQueryKey>,
      string | null
    >({
      queryKey: followingFeedQueryKey(limit, includeReactions),
      initialPageParam: null,
      staleTime: FOLLOWING_FEED_STALE_TIME,
      queryFn: ({ pageParam }) =>
        fetchFollowingFeedPage({
          cursor: pageParam ?? null,
          limit,
          includeReactions,
        }),
    });
  } catch (error) {
    errorHandler.log('Sidebar.prefetchFailed', error, {
      context: 'Sidebar.prefetchFollowingCategory',
      metadata: { category: 'following', limit, includeReactions },
    });
  }
};
