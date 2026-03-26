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
            <h3>Profile</h3>
            <small>{authorLabel}</small>
          </div>
        </div>
        <Button variant='secondary' type='button' onClick={onEdit}>
          プロフィールを編集
        </Button>
      </CardHeader>

      {status === 'loading' ? <Notice>Loading profile…</Notice> : null}
      {status === 'error' && error ? <Notice tone='destructive'>{error}</Notice> : null}

      <div className='shell-main-stack'>
        <p className='lede'>{about?.trim() || 'No profile bio published yet.'}</p>
        <div className='topic-diagnostic topic-diagnostic-secondary'>
          <span>Public posts in active topic</span>
          <span>{postCount}</span>
        </div>
      </div>
    </Card>
  );
}
