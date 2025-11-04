import { useInfiniteQuery, useQuery } from '@tanstack/react-query';
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

export const useTrendingTopicsQuery = (limit = 10) =>
  useQuery({
    queryKey: ['trending', 'topics', limit],
    queryFn: async (): Promise<TrendingTopicsResult> => {
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
    },
    staleTime: 60_000,
    refetchInterval: 120_000,
    retry: 1,
    onError: (error) => {
      errorHandler.log('TrendingTopics.fetchFailed', error, {
        context: 'useTrendingTopicsQuery',
      });
    },
  });

export const useTrendingPostsQuery = (
  topicIds: string[],
  perTopic = 3,
  options?: TrendingPostsQueryOptions,
) =>
  useQuery({
    queryKey: ['trending', 'posts', { topicIds, perTopic }],
    enabled: options?.enabled ?? topicIds.length > 0,
    queryFn: async (): Promise<TrendingPostsResult> => {
      if (topicIds.length === 0) {
        return { generatedAt: Date.now(), topics: [] };
      }

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
    },
    staleTime: 60_000,
    refetchInterval: 120_000,
    retry: 1,
    onError: (error) => {
      errorHandler.log('TrendingPosts.fetchFailed', error, {
        context: 'useTrendingPostsQuery',
        topicIds,
      });
    },
  });

export const useFollowingFeedQuery = (options: FollowingFeedQueryOptions = {}) => {
  const limit = options.limit ?? 20;
  const includeReactions = options.includeReactions;

  return useInfiniteQuery<FollowingFeedPageResult>({
    queryKey: [
      'followingFeed',
      {
        limit,
        includeReactions: includeReactions ?? false,
      },
    ],
    initialPageParam: null as string | null,
    getNextPageParam: (lastPage) => lastPage.nextCursor ?? undefined,
    keepPreviousData: true,
    staleTime: 60_000,
    retry: 1,
    queryFn: async ({ pageParam }): Promise<FollowingFeedPageResult> => {
      const params: ListFollowingFeedParams = {
        cursor: pageParam ?? null,
        limit,
      };
      if (includeReactions !== undefined) {
        params.includeReactions = includeReactions;
      }

      const response: FollowingFeedPage = await TauriApi.listFollowingFeed(params);
      const posts = await Promise.all(response.items.map((item) => mapPostResponseToDomain(item)));

      return {
        cursor: pageParam ?? null,
        items: posts,
        nextCursor: response.next_cursor,
        hasMore: response.has_more,
        serverTime: response.server_time,
      };
    },
    onError: (error) => {
      errorHandler.log('FollowingFeed.fetchFailed', error, {
        context: 'useFollowingFeedQuery',
      });
    },
  });
};
