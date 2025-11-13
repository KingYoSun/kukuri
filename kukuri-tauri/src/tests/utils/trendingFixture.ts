import { readFileSync } from 'node:fs';
import path from 'node:path';
import type {
  ListTrendingPostsResult,
  ListTrendingTopicsResult,
  FollowingFeedPage,
} from '@/lib/api/tauri';

export interface TrendingScenarioFixture {
  trendingTopics: ListTrendingTopicsResult;
  trendingPosts: ListTrendingPostsResult;
  followingFeed: FollowingFeedPage;
}

let cachedFixture: TrendingScenarioFixture | null | undefined;

const resolveFixturePath = (fixturePath: string) =>
  path.isAbsolute(fixturePath) ? fixturePath : path.join(process.cwd(), fixturePath);

export const getTrendingScenarioFixture = (): TrendingScenarioFixture | null => {
  if (cachedFixture !== undefined) {
    return cachedFixture;
  }

  const rawPath = process.env.VITE_TRENDING_FIXTURE_PATH;
  if (!rawPath) {
    cachedFixture = null;
    return cachedFixture;
  }

  try {
    const resolvedPath = resolveFixturePath(rawPath);
    const buffer = readFileSync(resolvedPath, 'utf-8');
    const parsed = JSON.parse(buffer) as {
      trending_topics: ListTrendingTopicsResult;
      trending_posts: ListTrendingPostsResult;
      following_feed: FollowingFeedPage;
    };

    cachedFixture = {
      trendingTopics: parsed.trending_topics,
      trendingPosts: parsed.trending_posts,
      followingFeed: parsed.following_feed,
    };
  } catch (error) {
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    cachedFixture = null;
  }

  return cachedFixture;
};

export const cloneFixture = <T>(value: T): T => JSON.parse(JSON.stringify(value));
