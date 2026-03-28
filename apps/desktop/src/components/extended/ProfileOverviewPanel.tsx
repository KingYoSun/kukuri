import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Notice } from '@/components/ui/notice';

type ProfileOverviewPanelProps = {
  authorLabel: string;
  about: string | null;
  picture: string | null;
  status: 'loading' | 'ready' | 'error';
  error: string | null;
  postCount: number;
  onEdit: () => void;
};

export function ProfileOverviewPanel({
  authorLabel,
  about,
  picture,
  status,
  error,
  postCount,
  onEdit,
}: ProfileOverviewPanelProps) {
  const { t } = useTranslation('profile');

  return (
    <Card className='panel-subsection'>
      <CardHeader className='profile-overview-header'>
        <div className='profile-overview-summary'>
          <div className='profile-overview-avatar'>
            {picture ? (
              <img src={picture} alt={`${authorLabel} avatar`} className='profile-overview-image' />
            ) : (
              <span>{authorLabel.slice(0, 1).toUpperCase()}</span>
            )}
          </div>
          <div>
            <h3>{t('overview.title')}</h3>
            <small>{authorLabel}</small>
          </div>
        </div>
        <Button variant='secondary' type='button' onClick={onEdit}>
          {t('overview.edit')}
        </Button>
      </CardHeader>

      {status === 'loading' ? <Notice>{t('overview.loading')}</Notice> : null}
      {status === 'error' && error ? <Notice tone='destructive'>{error}</Notice> : null}

      <div className='shell-main-stack'>
        <p className='lede'>{about?.trim() || t('overview.noBio')}</p>
        <div className='topic-diagnostic topic-diagnostic-secondary'>
          <span>{t('overview.postCount')}</span>
          <span>{postCount}</span>
        </div>
      </div>
    </Card>
  );
}
