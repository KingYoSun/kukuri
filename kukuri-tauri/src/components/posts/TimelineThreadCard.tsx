import { useTranslation } from 'react-i18next';
import { Link } from '@tanstack/react-router';
import { formatDistanceToNow } from 'date-fns';
import { getDateFnsLocale } from '@/i18n';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { ArrowRight, Clock3, MessageCircle } from 'lucide-react';
import { PostCard } from './PostCard';
import { cn } from '@/lib/utils';
import type { TopicTimelineEntry } from '@/hooks/usePosts';

interface TimelineThreadCardProps {
  entry: TopicTimelineEntry;
  topicId?: string;
  onParentPostClick?: (threadUuid: string) => void;
}

const isInteractiveElement = (target: EventTarget | null): boolean => {
  if (!(target instanceof Element)) {
    return false;
  }

  return target.closest('button, a, input, textarea, select') !== null;
};

export function TimelineThreadCard({ entry, topicId, onParentPostClick }: TimelineThreadCardProps) {
  const { t } = useTranslation();
  const lastActivity = formatDistanceToNow(new Date(entry.lastActivityAt * 1000), {
    addSuffix: true,
    locale: getDateFnsLocale(),
  });

  const canOpenPreview = typeof onParentPostClick === 'function';

  const handleParentPostClick = (event: React.MouseEvent<HTMLDivElement>) => {
    if (!canOpenPreview || isInteractiveElement(event.target)) {
      return;
    }
    onParentPostClick(entry.threadUuid);
  };

  const handleParentPostKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (!canOpenPreview) {
      return;
    }

    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      onParentPostClick(entry.threadUuid);
    }
  };

  return (
    <Card data-testid={`timeline-thread-card-${entry.threadUuid}`}>
      <CardHeader className="pb-3">
        <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
          <Badge
            variant="secondary"
            className="flex items-center gap-1"
            data-testid={`timeline-thread-replies-${entry.threadUuid}`}
          >
            <MessageCircle className="h-3 w-3" />
            <span>{t('topics.timelineReplies', { count: entry.replyCount })}</span>
          </Badge>
          <span
            className="flex items-center gap-1"
            data-testid={`timeline-thread-last-activity-${entry.threadUuid}`}
          >
            <Clock3 className="h-3 w-3" />
            <span>{t('topics.timelineLastActivity')}</span>
            <span aria-hidden>·</span>
            <span>{lastActivity}</span>
          </span>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <div
          data-testid={`timeline-thread-parent-${entry.threadUuid}`}
          role={canOpenPreview ? 'button' : undefined}
          tabIndex={canOpenPreview ? 0 : undefined}
          aria-label={canOpenPreview ? t('topics.openThreadPreview') : undefined}
          className={cn(
            canOpenPreview &&
              'cursor-pointer rounded-md transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
          )}
          onClick={handleParentPostClick}
          onKeyDown={handleParentPostKeyDown}
        >
          <PostCard post={entry.parentPost} />
        </div>
        {entry.firstReply && (
          <section
            className="rounded-md border border-dashed border-border bg-muted/20 p-3"
            data-testid={`timeline-thread-first-reply-${entry.threadUuid}`}
          >
            <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              {t('topics.timelineFirstReply')}
            </p>
            <PostCard post={entry.firstReply} />
          </section>
        )}

        {topicId && (
          <div className="flex justify-end">
            <Button asChild variant="outline" size="sm">
              <Link
                to="/topics/$topicId/threads/$threadUuid"
                params={{ topicId, threadUuid: entry.threadUuid }}
                data-testid={`timeline-thread-open-${entry.threadUuid}`}
              >
                <span>{t('topics.openThread')}</span>
                <ArrowRight className="h-4 w-4" />
              </Link>
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
