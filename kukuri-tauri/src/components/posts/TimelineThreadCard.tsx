import { useTranslation } from 'react-i18next';
import { formatDistanceToNow } from 'date-fns';
import { getDateFnsLocale } from '@/i18n';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Clock3, MessageCircle } from 'lucide-react';
import { PostCard } from './PostCard';
import type { TopicTimelineEntry } from '@/hooks/usePosts';

interface TimelineThreadCardProps {
  entry: TopicTimelineEntry;
}

export function TimelineThreadCard({ entry }: TimelineThreadCardProps) {
  const { t } = useTranslation();
  const lastActivity = formatDistanceToNow(new Date(entry.lastActivityAt * 1000), {
    addSuffix: true,
    locale: getDateFnsLocale(),
  });

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
        <div data-testid={`timeline-thread-parent-${entry.threadUuid}`}>
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
      </CardContent>
    </Card>
  );
}
