import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';

import { SummaryMetricCard } from '@/components/summary/SummaryMetricCard';
import type { TrendingPostsResult, TrendingTopicsResult } from '@/hooks/useTrendingFeeds';
import { useDirectMessageBadge } from '@/hooks/useDirectMessageBadge';
import { Button } from '@/components/ui/button';
import { useDirectMessageStore } from '@/stores/directMessageStore';

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

const formatLagLabel = (timestamp: number | null | undefined) => {
  if (!timestamp) {
    return null;
  }

  const lagSeconds = Math.max(0, Math.round((Date.now() - timestamp) / 1000));
  return `ラグ ${lagSeconds.toLocaleString()}秒`;
};

export function TrendingSummaryPanel({
  topics,
  posts,
  isTopicsFetching = false,
  isPostsFetching = false,
}: TrendingSummaryPanelProps) {
  const { unreadTotal, latestMessage, latestConversationNpub } = useDirectMessageBadge();
  const openInbox = useDirectMessageStore((state) => state.openInbox);

  const topicsCount = topics && topics.topics ? `${topics.topics.length.toLocaleString()}件` : null;

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
  const topicsLagLabel = formatLagLabel(topics?.generatedAt ?? null);

  const { display: previewUpdatedDisplay, helper: previewUpdatedHelper } = formatRelativeTime(
    posts?.generatedAt ?? null,
  );
  const previewLagLabel = formatLagLabel(posts?.generatedAt ?? null);

  const { display: dmDisplay, helper: dmHelper } = formatRelativeTime(
    latestMessage ? latestMessage.createdAt : null,
  );
  const dmHelperText = latestMessage
    ? [dmDisplay ?? dmHelper, latestConversationNpub ? `会話: ${latestConversationNpub}` : null]
        .filter(Boolean)
        .join(' / ') || '受信履歴なし'
    : '受信履歴なし';

  return (
    <section
      className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5"
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
        helperText={[updatedHelper, topicsLagLabel].filter(Boolean).join(' / ') || null}
        isLoading={isTopicsFetching && !topics}
        testId="trending-summary-updated"
      />
      <SummaryMetricCard
        label="プレビュー更新"
        value={previewUpdatedDisplay}
        helperText={[previewUpdatedHelper, previewLagLabel].filter(Boolean).join(' / ') || null}
        isLoading={isPostsFetching && !posts}
        testId="trending-summary-preview-updated"
      />
      <SummaryMetricCard
        label="DM未読"
        value={`${unreadTotal.toLocaleString()}件`}
        helperText={dmHelperText}
        isLoading={false}
        testId="trending-summary-direct-messages"
        action={
          <Button
            size="sm"
            variant="outline"
            onClick={openInbox}
            className="w-full"
            data-testid="trending-summary-dm-cta"
          >
            DM Inbox を開く
          </Button>
        }
      />
    </section>
  );
}
