import { useTranslation } from 'react-i18next';
import { useMemo } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { useFollowingFeedQuery } from '@/hooks/useTrendingFeeds';
import { FollowingSummaryPanel } from '@/components/following/FollowingSummaryPanel';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Loader2 } from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { getDateFnsLocale } from '@/i18n';

export function FollowingPage() {
  const { t } = useTranslation();
  const {
    data,
    isLoading,
    isError,
    error,
    refetch,
    hasNextPage,
    isFetchingNextPage,
    fetchNextPage,
    isFetching,
  } = useFollowingFeedQuery({ limit: 10, includeReactions: true });

  const posts = useMemo(() => data?.pages.flatMap((page) => page.items) ?? [], [data?.pages]);

  const isInitialLoading = isLoading && !data;
  const dateFnsLocale = getDateFnsLocale();

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
          <AlertTitle>{t('following.errorTitle')}</AlertTitle>
          <AlertDescription className="flex flex-col gap-4">
            <span>{error instanceof Error ? error.message : t('common.loading')}</span>
            <Button variant="outline" onClick={() => refetch()}>
              {t('common.retry')}
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
            <CardTitle>{t('following.empty')}</CardTitle>
          </CardHeader>
          <CardContent className="text-muted-foreground">{t('following.description')}</CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-8" data-testid="following-page">
      <div className="max-w-3xl mx-auto space-y-6">
        <FollowingSummaryPanel
          data={data}
          isLoading={isLoading && !data}
          isFetching={isFetching}
          hasNextPage={Boolean(hasNextPage)}
        />
        <header className="space-y-1">
          <h1 className="text-3xl font-bold">{t('following.title')}</h1>
          <p className="text-sm text-muted-foreground">{t('following.description')}</p>
        </header>

        <section className="space-y-3" data-testid="following-posts">
          {posts.map((post) => (
            <Card key={post.id} data-testid={`following-post-${post.id}`}>
              <CardHeader>
                <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm">
                  <span className="font-semibold">
                    {post.author.displayName || post.author.name || t('following.user')}
                  </span>
                  <span className="text-muted-foreground">
                    {formatDistanceToNow(new Date(post.created_at * 1000), {
                      addSuffix: true,
                      locale: dateFnsLocale,
                    })}
                  </span>
                </div>
              </CardHeader>
              <CardContent>
                <p className="text-sm leading-relaxed text-primary-foreground/90">
                  {post.content || t('following.postContentNotAvailable')}
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
              {isFetchingNextPage ? t('common.loading') : t('common.loadMore')}
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
