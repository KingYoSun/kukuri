import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';
import type { InfiniteData } from '@tanstack/react-query';

import { SummaryMetricCard } from '@/components/summary/SummaryMetricCard';
import type { FollowingFeedPageResult } from '@/hooks/useTrendingFeeds';
import { useDirectMessageBadge } from '@/hooks/useDirectMessageBadge';
import { Button } from '@/components/ui/button';
import { useDirectMessageStore } from '@/stores/directMessageStore';

interface FollowingSummaryPanelProps {
  data?: InfiniteData<FollowingFeedPageResult>;
  isLoading?: boolean;
  isFetching?: boolean;
  hasNextPage?: boolean;
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

export function FollowingSummaryPanel({
  data,
  isLoading = false,
  isFetching = false,
  hasNextPage = false,
}: FollowingSummaryPanelProps) {
  const { unreadTotal, latestMessage, latestConversationNpub } = useDirectMessageBadge();
  const openInbox = useDirectMessageStore((state) => state.openInbox);

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
  const { display: updatedDisplay, helper: updatedHelper } = formatRelativeTime(latestServerTime);
  const updatedLagLabel = formatLagLabel(latestServerTime);

  const remainingPages = data || hasNextPage ? (hasNextPage ? 'あり' : 'なし') : null;

  const showLoadingState = (condition: boolean) => (isLoading || isFetching) && condition;

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
      <SummaryMetricCard
        label="DM未読"
        value={`${unreadTotal.toLocaleString()}件`}
        helperText={dmHelperText}
        isLoading={false}
        testId="following-summary-direct-messages"
        action={
          <Button
            size="sm"
            variant="outline"
            onClick={openInbox}
            className="w-full"
            data-testid="following-summary-dm-cta"
          >
            DM Inbox を開く
          </Button>
        }
      />
    </section>
  );
}
