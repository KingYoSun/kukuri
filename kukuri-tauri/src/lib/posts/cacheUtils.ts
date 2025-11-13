import { InfiniteData, QueryClient } from '@tanstack/react-query';
import type { Post } from '@/stores';
import type { FollowingFeedPageResult, TrendingPostsResult } from '@/hooks/useTrendingFeeds';

const TRENDING_POSTS_QUERY_PREFIX = ['trending', 'posts'] as const;
const FOLLOWING_FEED_QUERY_PREFIX = ['followingFeed'] as const;

export type PostCacheInvalidationTarget = Pick<Post, 'id'> & {
  topicId?: string | null;
  authorPubkey?: string | null;
};

export function removePostFromTrendingCache(queryClient: QueryClient, postId: string) {
  const queries = queryClient.getQueriesData<TrendingPostsResult>({
    queryKey: TRENDING_POSTS_QUERY_PREFIX,
  });

  queries.forEach(([queryKey, data]) => {
    if (!data) {
      return;
    }

    let changed = false;
    const topics = data.topics.map((topic) => {
      const filtered = topic.posts.filter((post) => post.id !== postId);
      if (filtered.length !== topic.posts.length) {
        changed = true;
        return {
          ...topic,
          posts: filtered,
        };
      }
      return topic;
    });

    if (changed) {
      queryClient.setQueryData<TrendingPostsResult>(queryKey, {
        ...data,
        topics,
      });
    }
  });
}

export function removePostFromFollowingCache(queryClient: QueryClient, postId: string) {
  const queries = queryClient.getQueriesData<InfiniteData<FollowingFeedPageResult>>({
    queryKey: FOLLOWING_FEED_QUERY_PREFIX,
  });

  queries.forEach(([queryKey, data]) => {
    if (!data) {
      return;
    }

    let changed = false;
    const pages = data.pages.map((page) => {
      const filteredItems = page.items.filter((item) => item.id !== postId);
      if (filteredItems.length !== page.items.length) {
        changed = true;
        return {
          ...page,
          items: filteredItems,
        };
      }
      return page;
    });

    if (changed) {
      queryClient.setQueryData<InfiniteData<FollowingFeedPageResult>>(queryKey, {
        pageParams: data.pageParams,
        pages,
      });
    }
  });
}

export function invalidatePostCaches(
  queryClient: QueryClient,
  target: PostCacheInvalidationTarget,
) {
  queryClient.invalidateQueries({ queryKey: ['timeline'] });
  queryClient.invalidateQueries({ queryKey: ['posts', 'all'] });
  if (target.topicId) {
    queryClient.invalidateQueries({ queryKey: ['posts', target.topicId] });
  }
  if (target.authorPubkey) {
    queryClient.invalidateQueries({ queryKey: ['userPosts', target.authorPubkey] });
  }
  removePostFromTrendingCache(queryClient, target.id);
  removePostFromFollowingCache(queryClient, target.id);
}
