import { useTranslation } from 'react-i18next';

import { Card, CardHeader } from '@/components/ui/card';

import { AuthorAvatar } from './AuthorAvatar';
import { RelationshipBadge } from './RelationshipBadge';
import { type AuthorDetailView } from './types';

type AuthorDetailCardProps = {
  view: AuthorDetailView;
  localAuthorPubkey: string;
  onToggleRelationship: (authorPubkey: string, following: boolean) => void;
};

export function AuthorDetailCard({
  view,
  localAuthorPubkey,
  onToggleRelationship,
}: AuthorDetailCardProps) {
  const { t } = useTranslation(['common']);
  const author = view.author;
  const relationshipLabel = view.summary?.label ?? null;
  const showFollowAction = author?.author_pubkey !== localAuthorPubkey;

  return (
    <Card className='author-detail'>
      {author ? (
        <>
          <CardHeader className='author-detail-toolbar'>
            <div className='author-detail-summary'>
              <div className='author-detail-hero'>
                <AuthorAvatar
                  label={view.displayLabel}
                  picture={view.pictureSrc ?? author.picture ?? null}
                  size='sm'
                  testId='author-detail-avatar'
                />
                <div className='author-detail-identity'>
                  <strong className='author-detail-name author-detail-break'>{view.displayLabel}</strong>
                </div>
              </div>
              <div className='author-detail-copy-stack'>
                <p className='author-detail-copy author-detail-break'>
                  {author.about?.trim() || t('fallbacks.noBio')}
                </p>
                <small className='author-detail-monotext'>{author.author_pubkey}</small>
              </div>
            </div>
          </CardHeader>

          {view.summary && view.summary.viaPubkeys.length > 0 ? (
            <div className='topic-diagnostic topic-diagnostic-secondary'>
              <span>{t('relationships.via')}</span>
              <p className='author-detail-break'>{view.summary.viaPubkeys.join(', ')}</p>
            </div>
          ) : null}

          {relationshipLabel || showFollowAction ? (
            <div className='author-detail-actions'>
              {relationshipLabel ? <RelationshipBadge label={relationshipLabel} /> : <span />}
              {showFollowAction ? (
                <button
                  className='button button-secondary'
                  type='button'
                  onClick={() => onToggleRelationship(author.author_pubkey, author.following)}
                >
                  {view.summary?.followActionLabel === 'Unfollow'
                    ? t('actions.unfollow')
                    : t('actions.follow')}
                </button>
              ) : null}
            </div>
          ) : null}
        </>
      ) : (
        <p className='empty'>{t('fallbacks.selectAuthor')}</p>
      )}

      {view.authorError ? <p className='error error-inline'>{view.authorError}</p> : null}
    </Card>
  );
}
