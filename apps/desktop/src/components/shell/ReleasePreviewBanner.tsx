import { RefreshCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { RELEASE_CHANNEL } from '@/lib/releaseReadiness';

type ReleasePreviewBannerProps = {
  onOpenReleaseSettings: () => void;
};

export function ReleasePreviewBanner({ onOpenReleaseSettings }: ReleasePreviewBannerProps) {
  const { t } = useTranslation('shell');

  return (
    <aside className='shell-topbar release-preview-banner' aria-label={t('releaseBanner.label')}>
      <div className='min-w-0'>
        <strong>{t('releaseBanner.title', { channel: RELEASE_CHANNEL })}</strong>
        <span>{t('releaseBanner.body')}</span>
      </div>
      <Button variant='secondary' size='sm' type='button' onClick={onOpenReleaseSettings}>
        <RefreshCw className='size-4' aria-hidden='true' />
        {t('releaseBanner.action')}
      </Button>
    </aside>
  );
}
