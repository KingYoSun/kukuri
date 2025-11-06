import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';

import { SummaryMetricCard } from '@/components/summary/SummaryMetricCard';
import type { TrendingPostsResult, TrendingTopicsResult } from '@/hooks/useTrendingFeeds';

interface TrendingSummaryPanelProps {
  topics?: TrendingTopicsResult;
  posts?: TrendingPostsResult;
  isTopicsFetching?: boolean;
  isPostsFetching?: boolean;
}

const formatRelativeTime = (timestamp: number | null | undefined) => {
  if (!timestamp) {
    return { display: null, helper: null };
  }

  const date = new Date(timestamp);
  return {
    display: formatDistanceToNow(date, { addSuffix: true, locale: ja }),
    helper: date.toLocaleString('ja-JP'),
  };
};

export function TrendingSummaryPanel({
  topics,
  posts,
  isTopicsFetching = false,
  isPostsFetching = false,
}: TrendingSummaryPanelProps) {
  const topicsCount =
    topics && topics.topics ? `${topics.topics.length.toLocaleString()}件` : null;

  const previewPostsCount =
    posts?.topics != null
      ? `${posts.topics.reduce((total, topic) => total + topic.posts.length, 0).toLocaleString()}件`
      : topics && topics.topics.length === 0
        ? '0件'
        : null;

  const averageScore =
    topics && topics.topics.length > 0
      ? `${(
          topics.topics.reduce((total, topic) => total + topic.trendingScore, 0) /
          topics.topics.length
        ).toFixed(1)}pt`
      : topics && topics.topics.length === 0
        ? '0pt'
        : null;

  const { display: updatedDisplay, helper: updatedHelper } = formatRelativeTime(
    topics?.generatedAt ?? null,
  );

  return (
    <section
      className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4"
      data-testid="trending-summary-panel"
    >
      <SummaryMetricCard
        label="トレンドトピック"
        value={topicsCount}
        isLoading={isTopicsFetching && !topics}
        helperText="今表示中のトレンド対象数"
        testId="trending-summary-topics"
      />
      <SummaryMetricCard
        label="プレビュー投稿"
        value={previewPostsCount}
        isLoading={isPostsFetching && !posts}
        helperText="最新プレビューの合計件数"
        testId="trending-summary-posts"
      />
      <SummaryMetricCard
        label="平均スコア"
        value={averageScore}
        isLoading={isTopicsFetching && !topics}
        helperText="全トレンドの平均スコア"
        testId="trending-summary-score"
      />
      <SummaryMetricCard
        label="最終更新"
        value={updatedDisplay}
        helperText={updatedHelper}
        isLoading={isTopicsFetching && !topics}
        testId="trending-summary-updated"
      />
    </section>
  );
}
