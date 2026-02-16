import { useTranslation } from 'react-i18next';
import { SummaryMetricCard } from '@/components/summary/SummaryMetricCard';
import type { TrendingPostsResult, TrendingTopicsResult } from '@/hooks/useTrendingFeeds';
import { SummaryDirectMessageCard } from '@/components/summary/SummaryDirectMessageCard';
import { formatLagLabel, formatRelativeTimeInfo } from '@/components/summary/summaryTime';

interface TrendingSummaryPanelProps {
  topics?: TrendingTopicsResult;
  posts?: TrendingPostsResult;
  isTopicsFetching?: boolean;
  isPostsFetching?: boolean;
}

export function TrendingSummaryPanel({
  topics,
  posts,
  isTopicsFetching = false,
  isPostsFetching = false,
}: TrendingSummaryPanelProps) {
  const { t } = useTranslation();
  const topicsCount =
    topics && topics.topics ? t('trending.summary.items', { count: topics.topics.length }) : null;

  const previewPostsCount =
    posts?.topics != null
      ? t('trending.summary.items', {
          count: posts.topics.reduce((total, topic) => total + topic.posts.length, 0),
        })
      : topics && topics.topics.length === 0
        ? t('trending.summary.zeroItems')
        : null;

  const averageScore =
    topics && topics.topics.length > 0
      ? t('trending.summary.points', {
          score: (
            topics.topics.reduce((total, topic) => total + topic.trendingScore, 0) /
            topics.topics.length
          ).toFixed(1),
        })
      : topics && topics.topics.length === 0
        ? t('trending.summary.zeroPoints')
        : null;

  const { display: updatedDisplay, helper: updatedHelper } = formatRelativeTimeInfo(
    topics?.generatedAt ?? null,
  );
  const topicsLagLabel = formatLagLabel(topics?.generatedAt ?? null);

  const { display: previewUpdatedDisplay, helper: previewUpdatedHelper } = formatRelativeTimeInfo(
    posts?.generatedAt ?? null,
  );
  const previewLagLabel = formatLagLabel(posts?.generatedAt ?? null);

  return (
    <section
      className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5"
      data-testid="trending-summary-panel"
    >
      <SummaryMetricCard
        label={t('trending.summary.trendTopics')}
        value={topicsCount}
        isLoading={isTopicsFetching && !topics}
        helperText={t('trending.summary.trendTopicsHelper')}
        testId="trending-summary-topics"
      />
      <SummaryMetricCard
        label={t('trending.summary.previewPosts')}
        value={previewPostsCount}
        isLoading={isPostsFetching && !posts}
        helperText={t('trending.summary.previewPostsHelper')}
        testId="trending-summary-posts"
      />
      <SummaryMetricCard
        label={t('trending.summary.averageScore')}
        value={averageScore}
        isLoading={isTopicsFetching && !topics}
        helperText={t('trending.summary.averageScoreHelper')}
        testId="trending-summary-score"
      />
      <SummaryMetricCard
        label={t('trending.summary.lastUpdated')}
        value={updatedDisplay}
        helperText={[updatedHelper, topicsLagLabel].filter(Boolean).join(' / ') || null}
        isLoading={isTopicsFetching && !topics}
        testId="trending-summary-updated"
      />
      <SummaryMetricCard
        label={t('trending.summary.previewUpdated')}
        value={previewUpdatedDisplay}
        helperText={[previewUpdatedHelper, previewLagLabel].filter(Boolean).join(' / ') || null}
        isLoading={isPostsFetching && !posts}
        testId="trending-summary-preview-updated"
      />
      <SummaryDirectMessageCard testIdPrefix="trending-summary" />
    </section>
  );
}
