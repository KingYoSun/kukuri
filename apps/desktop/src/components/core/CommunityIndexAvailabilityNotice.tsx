import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import type { CommunityNodeAvailability } from '@/lib/api/communityNodeAvailability';
import { Button } from '@/components/ui/button';
import { Notice } from '@/components/ui/notice';

export function CommunityIndexAvailabilityNotice({
  availability, onRetry, onReviewPolicies, onOpenSettings, onAutomatic,
}: {
  availability: CommunityNodeAvailability;
  onRetry: () => Promise<void>;
  onReviewPolicies: (baseUrl: string) => void;
  onOpenSettings: () => void;
  onAutomatic: () => void;
}) {
  const { t } = useTranslation(['shell', 'settings', 'common']);
  const [now, setNow] = useState(() => Date.now());
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const remaining = Math.max(0, Math.ceil((availability.retryAfter ?? 0) - now / 1000));
  useEffect(() => {
    if (availability.retryAfter == null || availability.retryAfter * 1000 <= Date.now()) return;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [availability.retryAfter]);
  if (availability.reason === 'ready') return null;
  const canRetry = ['status', 'metadata', 'manifest'].includes(availability.recovery ?? '');
  return (
    <Notice className='shell-community-index-notice' tone={availability.reason === 'checking' || availability.reason === 'connecting' ? 'neutral' : 'warning'}>
      <div className='space-y-2'>
        <p role='status'>{t(`shell:communityIndex.availability.${availability.reason}`)}</p>
        {availability.baseUrl ? <p className='break-all text-sm'>
          {t('shell:communityIndex.nodeLabel')}: {availability.baseUrl}
        </p> : null}
        {availability.manual ? <p>{t('shell:communityIndex.availability.manualStopped')}</p> : null}
        {remaining > 0 ? <p>{t('shell:communityIndex.availability.retryAfter', { seconds: remaining })}</p> : null}
        {failed ? <p role='alert'>{t('shell:communityIndex.availability.retryFailed')}</p> : null}
        <div className='flex flex-wrap gap-2'>
          {availability.recovery === 'consent' && availability.baseUrl ? (
            <Button variant='secondary' onClick={() => onReviewPolicies(availability.baseUrl!)}>
              {t('settings:communityNode.onboarding.reviewPolicies')}
            </Button>
          ) : null}
          {canRetry ? (
            <Button variant='secondary' disabled={busy || remaining > 0} onClick={() => {
              if (busy || remaining > 0) return;
              setBusy(true);
              setFailed(false);
              void onRetry().catch(() => setFailed(true)).finally(() => setBusy(false));
            }}>
              {busy ? t('shell:communityIndex.loading') : t('shell:communityIndex.availability.retry')}
            </Button>
          ) : null}
          <Button variant='secondary' onClick={onOpenSettings}>
            {t('settings:communityNode.onboarding.openSettings')}
          </Button>
          {availability.manual ? (
            <Button variant='secondary' onClick={onAutomatic}>{t('shell:communityIndex.availability.automatic')}</Button>
          ) : null}
        </div>
      </div>
    </Notice>
  );
}
