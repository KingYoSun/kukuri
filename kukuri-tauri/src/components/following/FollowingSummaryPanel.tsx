import { useTranslation } from 'react-i18next';
import type { InfiniteData } from '@tanstack/react-query';

import { SummaryMetricCard } from '@/components/summary/SummaryMetricCard';
import type { FollowingFeedPageResult } from '@/hooks/useTrendingFeeds';
import { SummaryDirectMessageCard } from '@/components/summary/SummaryDirectMessageCard';
import { formatLagLabel, formatRelativeTimeInfo } from '@/components/summary/summaryTime';

interface FollowingSummaryPanelProps {
  data?: InfiniteData<FollowingFeedPageResult>;
  isLoading?: boolean;
  isFetching?: boolean;
  hasNextPage?: boolean;
}

export function FollowingSummaryPanel({
  data,
  isLoading = false,
  isFetching = false,
  hasNextPage = false,
}: FollowingSummaryPanelProps) {
  const { t } = useTranslation();
  const posts = data?.pages.flatMap((page) => page.items) ?? [];
  const postsCount = posts.length > 0 || data ? t('following.summary.items', { count: posts.length }) : null;

  const uniqueAuthors =
    posts.length > 0
      ? t('following.summary.people', { count: new Set(posts.map((post) => post.author.npub || post.author.pubkey)).size })
      : data
        ? t('following.summary.zeroPeople')
        : null;

  const latestServerTime = data
    ? data.pages.reduce((latest, page) => Math.max(latest, page.serverTime ?? 0), 0) || null
    : null;
  const { display: updatedDisplay, helper: updatedHelper } =
    formatRelativeTimeInfo(latestServerTime);
  const updatedLagLabel = formatLagLabel(latestServerTime);

  const remainingPages = data || hasNextPage ? (hasNextPage ? t('following.summary.hasRemaining') : t('following.summary.noRemaining')) : null;

  const showLoadingState = (condition: boolean) => (isLoading || isFetching) && condition;

  return (
    <section
      className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5"
      data-testid="following-summary-panel"
    >
      <SummaryMetricCard
        label={t('following.summary.fetchedPosts')}
        value={postsCount}
        isLoading={showLoadingState(!postsCount)}
        helperText={t('following.summary.fetchedPostsHelper')}
        testId="following-summary-posts"
      />
      <SummaryMetricCard
        label={t('following.summary.uniqueAuthors')}
        value={uniqueAuthors}
        isLoading={showLoadingState(!uniqueAuthors)}
        helperText={t('following.summary.uniqueAuthorsHelper')}
        testId="following-summary-authors"
      />
      <SummaryMetricCard
        label={t('following.summary.lastUpdated')}
        value={updatedDisplay}
        helperText={[updatedHelper, updatedLagLabel].filter(Boolean).join(' / ') || null}
        isLoading={showLoadingState(!updatedDisplay)}
        testId="following-summary-updated"
      />
      <SummaryMetricCard
        label={t('following.summary.remainingPages')}
        value={remainingPages}
        isLoading={isFetching && !data}
        helperText={t('following.summary.remainingPagesHelper')}
        testId="following-summary-remaining"
      />
      <SummaryDirectMessageCard testIdPrefix="following-summary" />
    </section>
  );
}
