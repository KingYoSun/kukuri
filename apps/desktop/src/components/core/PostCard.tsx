import { useTranslation } from 'react-i18next';

import { formatLocalizedTime } from '@/i18n/format';

import { AuthorAvatar } from './AuthorAvatar';
import { Button } from '@/components/ui/button';

import { RelationshipBadge } from './RelationshipBadge';
import { PostMedia } from './PostMedia';
import { type PostCardView } from './types';

function sourceAuthorLabel(view: PostCardView['post']['repost_of']): string | null {
  if (!view) {
    return null;
  }
  return (
    view.source_author_display_name?.trim() ||
    view.source_author_name?.trim() ||
    `${view.source_author_pubkey.slice(0, 8)}…`
  );
}

type PostCardProps = {
  view: PostCardView;
  onOpenAuthor: (authorPubkey: string) => void;
  onOpenThread: (threadId: string) => void;
  onOpenThreadInTopic?: (threadId: string, topicId: string) => void;
  onReply: (post: PostCardView['post']) => void;
  onRepost?: (post: PostCardView['post']) => void;
  onQuoteRepost?: (post: PostCardView['post']) => void;
  readOnly?: boolean;
  onOpenOriginalTopic?: (topicId: string) => void;
};

export function PostCard({
  view,
  onOpenAuthor,
  onOpenThread,
  onOpenThreadInTopic,
  onReply,
  onRepost,
  onQuoteRepost,
  readOnly = false,
  onOpenOriginalTopic,
}: PostCardProps) {
  const { t } = useTranslation(['common', 'profile']);
  const { post, context } = view;
  const isPendingText = post.content_status === 'Missing' && post.content === '[blob pending]';
  const audienceChipLabel = view.audienceChipLabel ?? post.audience_label;
  const publishedTopicId = post.published_topic_id?.trim() || post.origin_topic_id?.trim() || null;
  const repostSource = post.repost_of ?? null;
  const isQuoteRepost = post.object_kind === 'repost' && Boolean(post.repost_commentary?.trim());
  const canReply = view.canReply ?? true;
  const canRepost = view.canRepost ?? false;
  const hasPrimaryContent = isPendingText || post.content.trim().length > 0;

  const openPrimaryTarget = () => {
    const topicId = view.threadTopicId?.trim();
    if (topicId && onOpenThreadInTopic) {
      onOpenThreadInTopic(view.threadTargetId, topicId);
      return;
    }
    onOpenThread(view.threadTargetId);
  };

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

      {readOnly ? (
        <div className='post-link'>
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
            ) : hasPrimaryContent ? (
              <strong className='post-title'>{post.content}</strong>
            ) : null}

            {repostSource ? (
              <div className='repost-source-card'>
                <div className='topic-diagnostic topic-diagnostic-secondary'>
                  <span>{isQuoteRepost ? t('feed.quoteRepost') : t('feed.reposted')}</span>
                  <span>{t('labels.sourceTopic')}</span>
                  <span className='shell-topic-link-label' title={repostSource.source_topic_id}>
                    {repostSource.source_topic_id}
                  </span>
                </div>
                <div className='post-body repost-source-body'>
                  <strong className='post-title'>{sourceAuthorLabel(repostSource)}</strong>
                  {repostSource.content.trim().length > 0 ? (
                    <span>{repostSource.content}</span>
                  ) : null}
                </div>
              </div>
            ) : null}
          </div>

          <small>{post.envelope_id}</small>
          {post.reply_to ? <em className='post-reply-flag'>{t('actions.reply')}</em> : null}
          {publishedTopicId ? (
            <div className='topic-diagnostic topic-diagnostic-secondary'>
              <span>{t('feed.originTopic', { ns: 'profile' })}</span>
              <span className='shell-topic-link-label' title={publishedTopicId}>
                {publishedTopicId}
              </span>
            </div>
          ) : null}
        </div>
      ) : (
        <button className='post-link' type='button' onClick={openPrimaryTarget}>
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
            ) : hasPrimaryContent ? (
              <strong className='post-title'>{post.content}</strong>
            ) : null}

            {repostSource ? (
              <div className='repost-source-card'>
                <div className='topic-diagnostic topic-diagnostic-secondary'>
                  <span>{isQuoteRepost ? t('feed.quoteRepost') : t('feed.reposted')}</span>
                  <span>{t('labels.sourceTopic')}</span>
                  <span className='shell-topic-link-label' title={repostSource.source_topic_id}>
                    {repostSource.source_topic_id}
                  </span>
                </div>
                <div className='post-body repost-source-body'>
                  <strong className='post-title'>{sourceAuthorLabel(repostSource)}</strong>
                  {repostSource.content.trim().length > 0 ? (
                    <span>{repostSource.content}</span>
                  ) : null}
                </div>
              </div>
            ) : null}
          </div>

          <small>{post.envelope_id}</small>
          {post.reply_to ? <em className='post-reply-flag'>{t('actions.reply')}</em> : null}
        </button>
      )}

      <div className='post-actions'>
        {readOnly ? (
          publishedTopicId ? (
            <Button
              variant='secondary'
              type='button'
              onClick={() => onOpenOriginalTopic?.(publishedTopicId)}
            >
              {t('feed.openOriginalTopic', { ns: 'profile' })}
            </Button>
          ) : null
        ) : (
          <>
            {canRepost && onRepost ? (
              <Button variant='secondary' type='button' onClick={() => onRepost(post)}>
                {t('actions.repost')}
              </Button>
            ) : null}
            {canRepost && onQuoteRepost ? (
              <Button variant='secondary' type='button' onClick={() => onQuoteRepost(post)}>
                {t('actions.quoteRepost')}
              </Button>
            ) : null}
            {canReply ? (
              <Button variant='secondary' type='button' onClick={() => onReply(post)}>
                {t('actions.reply')}
              </Button>
            ) : null}
          </>
        )}
      </div>
    </article>
  );
}
