import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';

export const formatRelativeTimeInfo = (timestamp: number | null | undefined) => {
  if (!timestamp) {
    return { display: null, helper: null };
  }

  const date = new Date(timestamp);
  return {
    display: formatDistanceToNow(date, { addSuffix: true, locale: ja }),
    helper: date.toLocaleString('ja-JP'),
  };
};

export const formatLagLabel = (timestamp: number | null | undefined) => {
  if (!timestamp) {
    return null;
  }

  const lagSeconds = Math.max(0, Math.round((Date.now() - timestamp) / 1000));
  return `ラグ ${lagSeconds.toLocaleString()}秒`;
};
