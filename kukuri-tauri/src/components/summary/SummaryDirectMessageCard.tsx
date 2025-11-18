import { SummaryMetricCard } from '@/components/summary/SummaryMetricCard';
import { Button } from '@/components/ui/button';
import { useDirectMessageBadge } from '@/hooks/useDirectMessageBadge';
import { useDirectMessageStore } from '@/stores/directMessageStore';
import { formatRelativeTimeInfo } from './summaryTime';

interface SummaryDirectMessageCardProps {
  testIdPrefix: string;
}

export const SummaryDirectMessageCard = ({ testIdPrefix }: SummaryDirectMessageCardProps) => {
  const { unreadTotal, latestMessage, latestConversationNpub } = useDirectMessageBadge();
  const openInbox = useDirectMessageStore((state) => state.openInbox);

  const { display, helper } = formatRelativeTimeInfo(
    latestMessage ? latestMessage.createdAt : null,
  );
  const helperText = latestMessage
    ? [display ?? helper, latestConversationNpub ? `会話: ${latestConversationNpub}` : null]
        .filter(Boolean)
        .join(' / ') || '受信履歴なし'
    : '受信履歴なし';

  return (
    <SummaryMetricCard
      label="DM未読"
      value={`${unreadTotal.toLocaleString()}件`}
      helperText={helperText}
      isLoading={false}
      testId={`${testIdPrefix}-direct-messages`}
      action={
        <Button
          size="sm"
          variant="outline"
          onClick={openInbox}
          className="w-full"
          data-testid={`${testIdPrefix}-direct-messages-cta`}
        >
          DM Inbox を開く
        </Button>
      }
    />
  );
};
