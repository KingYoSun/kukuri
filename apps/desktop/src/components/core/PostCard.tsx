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
  const { post, context } = view;
  const isPendingText = post.content_status === 'Missing' && post.content === '[blob pending]';

  return (
    <article className={context === 'thread' ? 'post-card post-card-thread' : 'post-card'}>
      <div className='post-meta'>
        <button className='author-link' type='button' onClick={() => onOpenAuthor(post.author_pubkey)}>
          {view.authorLabel}
        </button>
        <div className='post-meta-trailing'>
          <RelationshipBadge label={view.relationshipLabel} />
          <span>{post.object_kind}</span>
          <span className='reply-chip'>{post.audience_label}</span>
          <span>{new Date(post.created_at * 1000).toLocaleTimeString('ja-JP')}</span>
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
        {post.reply_to ? <em className='reply-chip'>Reply</em> : null}
      </button>

      <div className='post-actions'>
        <Button variant='secondary' type='button' onClick={() => onReply(post)}>
          Reply
        </Button>
      </div>
    </article>
  );
}
