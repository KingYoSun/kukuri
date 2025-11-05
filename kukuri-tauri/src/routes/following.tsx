import { useMemo } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { useFollowingFeedQuery } from '@/hooks/useTrendingFeeds';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Loader2 } from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';

export function FollowingPage() {
  const {
    data,
    isLoading,
    isError,
    error,
    refetch,
    hasNextPage,
    isFetchingNextPage,
    fetchNextPage,
  } = useFollowingFeedQuery({ limit: 10, includeReactions: true });

  const posts = useMemo(
    () => data?.pages.flatMap((page) => page.items) ?? [],
    [data?.pages],
  );

  const isInitialLoading = isLoading && !data;

  if (isInitialLoading) {
    return (
      <div className="container mx-auto px-4 py-8" data-testid="following-loading">
        <div className="flex items-center justify-center py-16">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      </div>
    );
  }

  if (isError) {
    return (
      <div className="container mx-auto px-4 py-8" data-testid="following-error">
        <Alert variant="destructive" className="max-w-2xl mx-auto">
          <AlertTitle>フォロー中フィードの取得に失敗しました</AlertTitle>
          <AlertDescription className="flex flex-col gap-4">
            <span>
              {error instanceof Error
                ? error.message
                : 'ネットワーク状況を確認し、再度お試しください。'}
            </span>
            <Button variant="outline" onClick={() => refetch()}>
              再試行
            </Button>
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  if (!isLoading && posts.length === 0) {
    return (
      <div className="container mx-auto px-4 py-8" data-testid="following-empty">
        <Card className="max-w-2xl mx-auto">
          <CardHeader>
            <CardTitle>フォロー中の投稿はまだありません</CardTitle>
          </CardHeader>
          <CardContent className="text-muted-foreground">
            新しいユーザーをフォローすると、ここに最新の投稿が表示されます。検索ページで興味のあるユーザーを探してみましょう。
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-8" data-testid="following-page">
      <div className="max-w-3xl mx-auto space-y-6">
        <header className="space-y-1">
          <h1 className="text-3xl font-bold">フォロー中</h1>
          <p className="text-sm text-muted-foreground">
            フォローしているユーザーの最新投稿をまとめて確認できます。
          </p>
        </header>

        <section className="space-y-3" data-testid="following-posts">
          {posts.map((post) => (
            <Card key={post.id} data-testid={`following-post-${post.id}`}>
              <CardHeader>
                <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm">
                  <span className="font-semibold">
                    {post.author.displayName || post.author.name || 'ユーザー'}
                  </span>
                  <span className="text-muted-foreground">
                    {formatDistanceToNow(new Date(post.created_at * 1000), {
                      addSuffix: true,
                      locale: ja,
                    })}
                  </span>
                </div>
              </CardHeader>
              <CardContent>
                <p className="text-sm leading-relaxed text-primary-foreground/90">
                  {post.content || '投稿本文は表示できません。'}
                </p>
              </CardContent>
            </Card>
          ))}
        </section>

        {hasNextPage && (
          <div className="flex justify-center pt-2">
            <Button
              onClick={() => fetchNextPage()}
              disabled={isFetchingNextPage}
              variant="secondary"
              data-testid="following-load-more"
            >
              {isFetchingNextPage ? '読み込み中...' : 'さらに読み込む'}
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}

export const Route = createFileRoute('/following')({
  component: FollowingPage,
});
