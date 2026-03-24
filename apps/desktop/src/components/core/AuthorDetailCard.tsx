import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';

import { RelationshipBadge } from './RelationshipBadge';
import { type AuthorDetailView } from './types';

type AuthorDetailCardProps = {
  view: AuthorDetailView;
  localAuthorPubkey: string;
  onClearAuthor: () => void;
  onToggleRelationship: (authorPubkey: string, following: boolean) => void;
};

export function AuthorDetailCard({
  view,
  localAuthorPubkey,
  onClearAuthor,
  onToggleRelationship,
}: AuthorDetailCardProps) {
  return (
    <Card className='author-detail'>
      <CardHeader className='author-detail-header'>
        <h3>Author Detail</h3>
        {view.author ? (
          <Button variant='secondary' type='button' onClick={onClearAuthor}>
            Clear Author
          </Button>
        ) : null}
      </CardHeader>

      {view.author ? (
        <>
          <div className='author-detail-header'>
            <div>
              <strong>{view.displayLabel}</strong>
              <p className='author-detail-copy'>{view.author.about?.trim() || 'No profile bio published yet.'}</p>
            </div>
            <RelationshipBadge label={view.summary?.label ?? null} />
          </div>

          <small>{view.author.author_pubkey}</small>

          <div className='topic-diagnostic topic-diagnostic-secondary'>
            <span>following: {view.summary?.following ? 'yes' : 'no'}</span>
            <span>followed by: {view.summary?.followedBy ? 'yes' : 'no'}</span>
          </div>
          <div className='topic-diagnostic topic-diagnostic-secondary'>
            <span>mutual: {view.summary?.mutual ? 'yes' : 'no'}</span>
            <span>friend of friend: {view.summary?.friendOfFriend ? 'yes' : 'no'}</span>
          </div>

          {view.summary && view.summary.viaPubkeys.length > 0 ? (
            <div className='topic-diagnostic topic-diagnostic-secondary'>
              <span>via</span>
              <p>{view.summary.viaPubkeys.join(', ')}</p>
            </div>
          ) : null}

          {view.author.author_pubkey !== localAuthorPubkey ? (
            <div className='post-actions'>
              <Button
                variant='secondary'
                type='button'
                onClick={() =>
                  onToggleRelationship(view.author!.author_pubkey, view.author!.following)
                }
              >
                {view.summary?.followActionLabel ?? (view.author.following ? 'Unfollow' : 'Follow')}
              </Button>
            </div>
          ) : null}
        </>
      ) : (
        <p className='empty'>Select an author to inspect profile and relationship.</p>
      )}

      {view.authorError ? <p className='error error-inline'>{view.authorError}</p> : null}
    </Card>
  );
}
