import { useTranslation } from 'react-i18next';

import { formatLocalizedTime } from '@/i18n/format';

import { AuthorAvatar } from './AuthorAvatar';
import { Button } from '@/components/ui/button';

import { RelationshipBadge } from './RelationshipBadge';
import { PostMedia } from './PostMedia';
import { type PostCardView } from './types';

type PostCardProps = {
  view: PostCardView;
  onOpenAuthor: (authorPubkey: string) => void;
  onOpenThread: (threadId: string) => void;
  onReply: (post: PostCardView['post']) => void;
};

export function PostCard({ view, onOpenAuthor, onOpenThread, onReply }: PostCardProps) {
  const { t } = useTranslation(['common']);
  const { post, context } = view;
  const isPendingText = post.content_status === 'Missing' && post.content === '[blob pending]';
  const audienceChipLabel = view.audienceChipLabel ?? post.audience_label;

  return (
    <article className={context === 'thread' ? 'post-card post-card-thread' : 'post-card'}>
      <div className='post-meta'>
        <div className='post-meta-author'>
          <AuthorAvatar
            label={view.authorLabel}
            picture={view.authorPicture ?? null}
            size='sm'
            testId={`${post.object_id}-author-avatar`}
          />
          <button
            className='author-link'
            type='button'
            onClick={() => onOpenAuthor(post.author_pubkey)}
          >
            {view.authorLabel}
          </button>
        </div>
        <div className='post-meta-trailing'>
          <RelationshipBadge label={view.relationshipLabel} />
          <span className='post-meta-chip'>{audienceChipLabel}</span>
          <span>{formatLocalizedTime(post.created_at * 1000)}</span>
        </div>
      </div>

      <button className='post-link' type='button' onClick={() => onOpenThread(view.threadTargetId)}>
        <PostMedia media={view.media} />

        <div className='post-body'>
          {isPendingText ? (
            <div
              className='text-skeleton-group'
              data-testid={`text-skeleton-${post.object_id}`}
              aria-hidden='true'
            >
              <span className='text-skeleton text-skeleton-line' />
              <span className='text-skeleton text-skeleton-line text-skeleton-line-short' />
            </div>
          ) : (
            <strong className='post-title'>{post.content}</strong>
          )}
        </div>

        <small>{post.envelope_id}</small>
        {post.reply_to ? <em className='post-reply-flag'>{t('actions.reply')}</em> : null}
      </button>

      <div className='post-actions'>
        <Button variant='secondary' type='button' onClick={() => onReply(post)}>
          {t('actions.reply')}
        </Button>
      </div>
    </article>
  );
}
