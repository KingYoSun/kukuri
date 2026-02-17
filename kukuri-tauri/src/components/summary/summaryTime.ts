import { formatDistanceToNow } from 'date-fns';
import { getDateFnsLocale } from '@/i18n';
import i18n from '@/i18n';
import { formatDateTimeByI18n, formatNumberByI18n } from '@/lib/utils/localeFormat';

export const formatRelativeTimeInfo = (timestamp: number | null | undefined) => {
  if (!timestamp) {
    return { display: null, helper: null };
  }

  const date = new Date(timestamp);
  return {
    display: formatDistanceToNow(date, { addSuffix: true, locale: getDateFnsLocale() }),
    helper: formatDateTimeByI18n(date),
  };
};

export const formatLagLabel = (timestamp: number | null | undefined) => {
  if (!timestamp) {
    return null;
  }

  const lagSeconds = Math.max(0, Math.round((Date.now() - timestamp) / 1000));
  return i18n.t('summary.lag', { seconds: formatNumberByI18n(lagSeconds) });
};
