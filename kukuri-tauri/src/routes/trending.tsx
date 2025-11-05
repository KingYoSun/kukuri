import { useMemo } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import {
  useTrendingTopicsQuery,
  useTrendingPostsQuery,
  type TrendingTopicSummary,
} from '@/hooks/useTrendingFeeds';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Loader2, ArrowUpRight } from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';

interface TopicWithPosts extends TrendingTopicSummary {
  posts: {
    id: string;
    content: string;
    created_at: number;
    author: {
      displayName: string;
      name: string;
    };
  }[];
}

export function TrendingPage() {
  const {
    data: topicsData,
    isLoading: topicsLoading,
    isError: topicsIsError,
    error: topicsError,
    refetch: refetchTopics,
  } = useTrendingTopicsQuery();

  const topicIds = topicsData?.topics.map((topic) => topic.topicId) ?? [];

  const {
    data: postsData,
    isLoading: postsLoading,
    isError: postsIsError,
    error: postsError,
    refetch: refetchPosts,
  } = useTrendingPostsQuery(topicIds, 3, { enabled: topicIds.length > 0 });

  const topicsWithPosts = useMemo<TopicWithPosts[]>(() => {
    if (!topicsData) {
      return [];
    }
    const postsByTopic = new Map<string, TopicWithPosts['posts']>();

    postsData?.topics.forEach((topic) => {
      postsByTopic.set(
        topic.topicId,
        topic.posts.map((post) => ({
          id: post.id,
          content: post.content,
          created_at: post.created_at,
          author: {
            displayName: post.author.displayName,
            name: post.author.name,
          },
        })),
      );
    });

    return topicsData.topics.map((topic) => ({
      ...topic,
      posts: postsByTopic.get(topic.topicId) ?? [],
    }));
  }, [postsData, topicsData]);

  const isInitialLoading =
    topicsLoading || (topicIds.length > 0 && postsLoading && !postsData && !postsIsError);

  if (isInitialLoading) {
    return (
      <div className="container mx-auto px-4 py-8" data-testid="trending-loading">
        <div className="flex items-center justify-center py-16">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      </div>
    );
  }

  if (topicsIsError) {
    return (
      <div className="container mx-auto px-4 py-8" data-testid="trending-error">
        <Alert variant="destructive" className="max-w-2xl mx-auto">
          <AlertTitle>トレンド情報の取得に失敗しました</AlertTitle>
          <AlertDescription className="flex flex-col gap-4">
            <span>
              {topicsError instanceof Error
                ? topicsError.message
                : 'しばらく時間をおいてから再度お試しください。'}
            </span>
            <Button variant="outline" onClick={() => refetchTopics()}>
              再試行
            </Button>
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  if (!topicsLoading && topicsData && topicsData.topics.length === 0) {
    return (
      <div className="container mx-auto px-4 py-8" data-testid="trending-empty">
        <Card className="max-w-2xl mx-auto">
          <CardHeader>
            <CardTitle>トレンドはまだありません</CardTitle>
          </CardHeader>
          <CardContent className="text-muted-foreground">
            新しくトピックに参加すると、トレンド一覧が充実します。気になるトピックを探してみましょう。
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-8" data-testid="trending-page">
      <div className="max-w-5xl mx-auto space-y-6">
        <header className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
          <div>
            <h1 className="text-3xl font-bold">トレンド</h1>
            {topicsData?.generatedAt && (
              <p className="text-sm text-muted-foreground">
                更新:
                {formatDistanceToNow(new Date(topicsData.generatedAt), {
                  addSuffix: true,
                  locale: ja,
                })}
              </p>
            )}
          </div>
          {postsIsError && (
            <Alert variant="destructive" className="max-w-md" data-testid="trending-posts-error">
              <AlertTitle>投稿プレビューの取得に失敗しました</AlertTitle>
              <AlertDescription className="flex flex-col gap-2">
                <span>
                  {postsError instanceof Error
                    ? postsError.message
                    : '投稿プレビューを再取得してください。'}
                </span>
                <Button variant="outline" size="sm" onClick={() => refetchPosts()}>
                  再試行
                </Button>
              </AlertDescription>
            </Alert>
          )}
        </header>

        <section className="grid gap-4 md:grid-cols-2" data-testid="trending-topics-list">
          {topicsWithPosts.map((topic) => (
            <Card key={topic.topicId} data-testid={`trending-topic-${topic.topicId}`}>
              <CardHeader className="flex flex-col gap-3">
                <div className="flex items-center justify-between gap-4">
                  <div className="flex items-center gap-3">
                    <Badge variant="secondary" data-testid={`trending-rank-${topic.rank}`}>
                      #{topic.rank}
                    </Badge>
                    <CardTitle className="text-xl">{topic.name}</CardTitle>
                  </div>
                  <div className="text-right">
                    <div className="text-sm text-muted-foreground">スコア</div>
                    <div className="text-lg font-semibold">{topic.trendingScore.toFixed(1)}</div>
                  </div>
                </div>
                {topic.description && (
                  <p className="text-sm text-muted-foreground line-clamp-2">{topic.description}</p>
                )}
                <div className="flex flex-wrap items-center gap-3 text-sm text-muted-foreground">
                  <span>参加者 {topic.memberCount}</span>
                  <span>投稿 {topic.postCount}</span>
                  {topic.scoreChange !== null && (
                    <span
                      data-testid={`trending-score-change-${topic.topicId}`}
                      className={topic.scoreChange >= 0 ? 'text-emerald-600' : 'text-red-600'}
                    >
                      {topic.scoreChange >= 0 ? '+' : ''}
                      {topic.scoreChange.toFixed(1)}pt
                    </span>
                  )}
                </div>
              </CardHeader>
              <CardContent className="space-y-3">
                <h3 className="text-sm font-semibold text-muted-foreground">最新の投稿</h3>
                {topic.posts.length > 0 ? (
                  <ul className="space-y-2" data-testid={`trending-topic-${topic.topicId}-posts`}>
                    {topic.posts.map((post) => (
                      <li
                        key={post.id}
                        className="rounded-md border border-border bg-muted/40 p-3 text-sm"
                      >
                        <div className="flex items-center gap-2 text-xs text-muted-foreground">
                          <ArrowUpRight className="h-3 w-3" />
                          <span>{post.author.displayName || post.author.name || 'ユーザー'}</span>
                          <span aria-hidden>·</span>
                          <span>
                            {formatDistanceToNow(new Date(post.created_at * 1000), {
                              addSuffix: true,
                              locale: ja,
                            })}
                          </span>
                        </div>
                        <p className="mt-2 line-clamp-3 text-sm text-primary-foreground/90">
                          {post.content || '投稿本文は省略されています。'}
                        </p>
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p
                    className="rounded-md border border-dashed border-border bg-muted/20 p-3 text-sm text-muted-foreground"
                    data-testid={`trending-topic-${topic.topicId}-empty`}
                  >
                    このトピックの投稿はまだありません。最初の投稿をしてみましょう。
                  </p>
                )}
              </CardContent>
            </Card>
          ))}
        </section>
      </div>
    </div>
  );
}

export const Route = createFileRoute('/trending')({
  component: TrendingPage,
});
