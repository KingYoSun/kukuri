import { useTranslation } from 'react-i18next';
import { SummaryMetricCard } from '@/components/summary/SummaryMetricCard';
import { Button } from '@/components/ui/button';
import { useDirectMessageBadge } from '@/hooks/useDirectMessageBadge';
import { useDirectMessageStore } from '@/stores/directMessageStore';
import { formatRelativeTimeInfo } from './summaryTime';

interface SummaryDirectMessageCardProps {
  testIdPrefix: string;
}

export const SummaryDirectMessageCard = ({ testIdPrefix }: SummaryDirectMessageCardProps) => {
  const { t } = useTranslation();
  const { unreadTotal, latestMessage, latestConversationNpub } = useDirectMessageBadge();
  const openInbox = useDirectMessageStore((state) => state.openInbox);

  const { display, helper } = formatRelativeTimeInfo(
    latestMessage ? latestMessage.createdAt : null,
  );
  const helperText = latestMessage
    ? [
        display ?? helper,
        latestConversationNpub
          ? t('summary.dmConversation', { npub: latestConversationNpub })
          : null,
      ]
        .filter(Boolean)
        .join(' / ') || t('summary.dmNoHistory')
    : t('summary.dmNoHistory');

  return (
    <SummaryMetricCard
      label={t('summary.dmUnread')}
      value={t('summary.dmItems', { count: unreadTotal })}
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
          {t('summary.dmOpenInbox')}
        </Button>
      }
    />
  );
};
