import { useTranslation } from 'react-i18next';
import { Link, createFileRoute } from '@tanstack/react-router';
import { Loader2, MessagesSquare } from 'lucide-react';
import { useThreadPosts } from '@/hooks';
import { useTopicStore } from '@/stores';
import { ForumThreadView } from '@/components/posts/ForumThreadView';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';

export const Route = createFileRoute('/topics/$topicId/threads/$threadUuid')({
  component: TopicThreadDetailRoute,
});

function TopicThreadDetailRoute() {
  const { topicId, threadUuid } = Route.useParams();
  return <TopicThreadDetailPage topicId={topicId} threadUuid={threadUuid} />;
}

interface TopicThreadDetailPageProps {
  topicId: string;
  threadUuid: string;
}

export function TopicThreadDetailPage({ topicId, threadUuid }: TopicThreadDetailPageProps) {
  const { t } = useTranslation();
  const topicName = useTopicStore((state) => state.topics.get(topicId)?.name ?? topicId);
  const { data: threadPosts, isLoading, error, refetch } = useThreadPosts(topicId, threadUuid);

  return (
    <div className="space-y-6">
      <header className="rounded-lg border bg-card p-6">
        <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
          <div>
            <div className="mb-2 flex items-center gap-2 text-muted-foreground">
              <MessagesSquare className="h-4 w-4" />
              <span className="text-sm">{t('topics.threadDetailTitle')}</span>
            </div>
            <h1 className="text-2xl font-bold" data-testid="thread-detail-title">
              {t('topics.threadDetailTopicTitle', { topic: topicName })}
            </h1>
            <p className="mt-2 text-sm text-muted-foreground" data-testid="thread-detail-uuid">
              {t('topics.threadDetailUuid', { uuid: threadUuid })}
            </p>
          </div>

          <div className="flex gap-2">
            <Button asChild variant="outline" data-testid="thread-detail-back-to-list">
              <Link to="/topics/$topicId/threads" params={{ topicId }}>
                {t('topics.backToThreads')}
              </Link>
            </Button>
            <Button asChild variant="ghost" data-testid="thread-detail-back-to-topic">
              <Link to="/topics/$topicId" params={{ topicId }}>
                {t('topics.backToTimeline')}
              </Link>
            </Button>
          </div>
        </div>
      </header>

      {isLoading ? (
        <div className="flex justify-center py-12" data-testid="thread-detail-loading">
          <Loader2 className="h-8 w-8 animate-spin" />
        </div>
      ) : error ? (
        <Alert variant="destructive" data-testid="thread-detail-error">
          <AlertDescription className="space-y-2">
            <p>{t('topics.threadLoadFailed')}</p>
            <Button type="button" variant="outline" size="sm" onClick={() => refetch()}>
              {t('common.retry')}
            </Button>
          </AlertDescription>
        </Alert>
      ) : !threadPosts || threadPosts.length === 0 ? (
        <Alert data-testid="thread-detail-empty">
          <AlertDescription>{t('topics.threadNotFound')}</AlertDescription>
        </Alert>
      ) : (
        <ForumThreadView threadUuid={threadUuid} posts={threadPosts} />
      )}
    </div>
  );
}
