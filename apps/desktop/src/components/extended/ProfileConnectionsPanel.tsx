import { useTranslation } from 'react-i18next';

import { AuthorAvatar } from '@/components/core/AuthorAvatar';
import { RelationshipBadge } from '@/components/core/RelationshipBadge';
import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Notice } from '@/components/ui/notice';
import type { ProfileConnectionsView } from '@/components/shell/types';
import type { ExtendedPanelStatus } from '@/components/extended/types';
import type { AuthorSocialView } from '@/lib/api';

type ProfileConnectionsPanelProps = {
  activeView: ProfileConnectionsView;
  items: Array<AuthorSocialView & { picture_src?: string | null }>;
  localAuthorPubkey: string;
  status: ExtendedPanelStatus;
  error: string | null;
  onSelectView: (view: ProfileConnectionsView) => void;
  onToggleRelationship: (authorPubkey: string, following: boolean) => void;
  onToggleMute: (authorPubkey: string, muted: boolean) => void;
  onBack: () => void;
};

const CONNECTION_VIEWS: ProfileConnectionsView[] = ['following', 'followed', 'muted'];

function displayLabel(author: AuthorSocialView): string {
  return author.display_name?.trim() || author.name?.trim() || author.author_pubkey;
}

function strongestRelationshipLabel(author: AuthorSocialView): string | null {
  if (author.mutual) {
    return 'mutual';
  }
  if (author.following) {
    return 'following';
  }
  if (author.followed_by) {
    return 'follows you';
  }
  if (author.friend_of_friend) {
    return 'friend of friend';
  }
  return null;
}

export function ProfileConnectionsPanel({
  activeView,
  items,
  localAuthorPubkey,
  status,
  error,
  onSelectView,
  onToggleRelationship,
  onToggleMute,
  onBack,
}: ProfileConnectionsPanelProps) {
  const { t } = useTranslation(['profile', 'common']);

  return (
    <Card className='panel-subsection'>
      <CardHeader>
        <h3>{t('connections.title')}</h3>
        <Button variant='secondary' type='button' onClick={onBack}>
          {t('connections.back')}
        </Button>
      </CardHeader>

      <div className='shell-workspace-tabs' role='tablist' aria-label={t('connections.tabsLabel')}>
        {CONNECTION_VIEWS.map((view) => (
          <button
            key={view}
            className={`shell-tab${activeView === view ? ' shell-tab-active' : ''}`}
            role='tab'
            type='button'
            aria-selected={activeView === view}
            onClick={() => onSelectView(view)}
          >
            {t(`connections.tabs.${view}`)}
          </button>
        ))}
      </div>

      {status === 'loading' ? <Notice>{t('connections.loading')}</Notice> : null}
      {status === 'error' && error ? <Notice tone='destructive'>{error}</Notice> : null}

      {status === 'ready' && items.length === 0 ? (
        <p className='empty-state'>{t(`connections.empty.${activeView}`)}</p>
      ) : null}

      {items.length > 0 ? (
        <ul className='post-list'>
          {items.map((author) => {
            const label = displayLabel(author);
            const relationshipLabel = strongestRelationshipLabel(author);
            const showActions = author.author_pubkey !== localAuthorPubkey;

            return (
              <li key={author.author_pubkey}>
                <article className='post-card'>
                  <div className='post-meta'>
                    <span>{label}</span>
                    <span>{author.author_pubkey}</span>
                  </div>
                  <div className='post-body'>
                    <div className='author-detail-hero'>
                      <AuthorAvatar label={label} picture={author.picture_src ?? author.picture ?? null} size='sm' />
                      <div className='author-detail-copy-stack'>
                        <strong className='post-title'>{label}</strong>
                        <p className='author-detail-copy author-detail-break'>
                          {author.about?.trim() || t('common:fallbacks.noBio')}
                        </p>
                      </div>
                    </div>
                  </div>
                  <div className='post-actions'>
                    {relationshipLabel ? <RelationshipBadge label={relationshipLabel} /> : null}
                    {author.muted ? (
                      <span className='relationship-badge relationship-badge-direct'>
                        {t('connections.mutedBadge')}
                      </span>
                    ) : null}
                  </div>
                  {showActions ? (
                    <div className='post-actions'>
                      <Button
                        variant='secondary'
                        type='button'
                        onClick={() => onToggleRelationship(author.author_pubkey, author.following)}
                      >
                        {author.following ? t('common:actions.unfollow') : t('common:actions.follow')}
                      </Button>
                      <Button
                        variant='secondary'
                        type='button'
                        onClick={() => onToggleMute(author.author_pubkey, author.muted)}
                      >
                        {author.muted
                          ? t('common:actions.unmute', { defaultValue: 'Unmute' })
                          : t('common:actions.mute', { defaultValue: 'Mute' })}
                      </Button>
                    </div>
                  ) : null}
                </article>
              </li>
            );
          })}
        </ul>
      ) : null}
    </Card>
  );
}
