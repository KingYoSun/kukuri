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
  const posts = data?.pages.flatMap((page) => page.items) ?? [];
  const postsCount = posts.length > 0 || data ? `${posts.length.toLocaleString()}件` : null;

  const uniqueAuthors =
    posts.length > 0
      ? `${new Set(posts.map((post) => post.author.npub || post.author.pubkey)).size.toLocaleString()}人`
      : data
        ? '0人'
        : null;

  const latestServerTime = data
    ? data.pages.reduce((latest, page) => Math.max(latest, page.serverTime ?? 0), 0) || null
    : null;
  const { display: updatedDisplay, helper: updatedHelper } = formatRelativeTimeInfo(latestServerTime);
  const updatedLagLabel = formatLagLabel(latestServerTime);

  const remainingPages = data || hasNextPage ? (hasNextPage ? 'あり' : 'なし') : null;

  const showLoadingState = (condition: boolean) => (isLoading || isFetching) && condition;

  return (
    <section
      className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5"
      data-testid="following-summary-panel"
    >
      <SummaryMetricCard
        label="取得済み投稿"
        value={postsCount}
        isLoading={showLoadingState(!postsCount)}
        helperText="現在の表示件数"
        testId="following-summary-posts"
      />
      <SummaryMetricCard
        label="ユニーク投稿者"
        value={uniqueAuthors}
        isLoading={showLoadingState(!uniqueAuthors)}
        helperText="表示中の投稿者数"
        testId="following-summary-authors"
      />
      <SummaryMetricCard
        label="最終更新"
        value={updatedDisplay}
        helperText={[updatedHelper, updatedLagLabel].filter(Boolean).join(' / ') || null}
        isLoading={showLoadingState(!updatedDisplay)}
        testId="following-summary-updated"
      />
      <SummaryMetricCard
        label="残ページ"
        value={remainingPages}
        isLoading={isFetching && !data}
        helperText="追加ロードの必要有無"
        testId="following-summary-remaining"
      />
      <SummaryDirectMessageCard testIdPrefix="following-summary" />
    </section>
  );
}
