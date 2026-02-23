import { useTranslation } from 'react-i18next';
import { Link, createFileRoute } from '@tanstack/react-router';
import { Loader2, ListTree } from 'lucide-react';
import { useTopicThreads } from '@/hooks';
import { useTopicStore } from '@/stores';
import { TimelineThreadCard } from '@/components/posts/TimelineThreadCard';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';

export const Route = createFileRoute('/topics/$topicId/threads')({
  component: TopicThreadsRoute,
});

function TopicThreadsRoute() {
  const { topicId } = Route.useParams();
  return <TopicThreadsPage topicId={topicId} />;
}

interface TopicThreadsPageProps {
  topicId: string;
}

export function TopicThreadsPage({ topicId }: TopicThreadsPageProps) {
  const { t } = useTranslation();
  const topicName = useTopicStore((state) => state.topics.get(topicId)?.name ?? topicId);
  const { data: threadEntries, isLoading } = useTopicThreads(topicId);

  return (
    <div className="space-y-6">
      <header className="rounded-lg border bg-card p-6">
        <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
          <div>
            <div className="mb-2 flex items-center gap-2 text-muted-foreground">
              <ListTree className="h-4 w-4" />
              <span className="text-sm">{t('topics.openThreads')}</span>
            </div>
            <h1 className="text-2xl font-bold" data-testid="thread-list-title">
              {t('topics.threadListTitle', { topic: topicName })}
            </h1>
            <p className="mt-2 text-sm text-muted-foreground">
              {t('topics.threadListDescription')}
            </p>
          </div>

          <Button asChild variant="outline" data-testid="thread-list-back-to-topic">
            <Link to="/topics/$topicId" params={{ topicId }}>
              {t('topics.backToTimeline')}
            </Link>
          </Button>
        </div>
      </header>

      {isLoading ? (
        <div className="flex justify-center py-12" data-testid="thread-list-loading">
          <Loader2 className="h-8 w-8 animate-spin" />
        </div>
      ) : !threadEntries || threadEntries.length === 0 ? (
        <Alert data-testid="thread-list-empty">
          <AlertDescription>{t('topics.threadListEmpty')}</AlertDescription>
        </Alert>
      ) : (
        <section className="space-y-4" data-testid="thread-list-items">
          {threadEntries.map((entry) => (
            <TimelineThreadCard key={entry.threadUuid} entry={entry} topicId={topicId} />
          ))}
        </section>
      )}
    </div>
  );
}
