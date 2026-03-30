import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Reply, Repeat2 } from 'lucide-react';

import { formatLocalizedTime } from '@/i18n/format';
import type {
  BookmarkedCustomReactionView,
  CustomReactionAssetView,
  ReactionKeyInput,
  ReactionKeyView,
  RecentReactionView,
} from '@/lib/api';

import { AuthorAvatar } from './AuthorAvatar';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';

import { RelationshipBadge } from './RelationshipBadge';
import { PostMedia } from './PostMedia';
import { ReactionPickerPopover } from './ReactionPickerPopover';
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
  localAuthorPubkey?: string;
  mediaObjectUrls?: Record<string, string | null>;
  ownedReactionAssets?: CustomReactionAssetView[];
  bookmarkedReactionAssets?: BookmarkedCustomReactionView[];
  recentReactions?: RecentReactionView[];
  onToggleReaction?: (post: PostCardView['post'], reactionKey: ReactionKeyInput) => void;
  onBookmarkCustomReaction?: (asset: CustomReactionAssetView) => void;
  showBookmarkAction?: boolean;
  isBookmarked?: boolean;
  onToggleBookmark?: (post: PostCardView['post']) => void;
};

function reactionKeyInputFromView(reaction: ReactionKeyView): ReactionKeyInput | null {
  if (reaction.reaction_key_kind === 'emoji' && reaction.emoji?.trim()) {
    return { kind: 'emoji', emoji: reaction.emoji };
  }
  if (reaction.reaction_key_kind === 'custom_asset' && reaction.custom_asset) {
    return { kind: 'custom_asset', asset: reaction.custom_asset };
  }
  return null;
}

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
  localAuthorPubkey,
  mediaObjectUrls = {},
  ownedReactionAssets = [],
  bookmarkedReactionAssets = [],
  recentReactions = [],
  onToggleReaction,
  onBookmarkCustomReaction,
  showBookmarkAction = false,
  isBookmarked = false,
  onToggleBookmark,
}: PostCardProps) {
  const { t } = useTranslation(['common', 'profile']);
  const { post, context } = view;
  const [repostMenuOpen, setRepostMenuOpen] = useState(false);
  const isPendingText = post.content_status === 'Missing' && post.content === '[blob pending]';
  const audienceChipLabel = view.audienceChipLabel ?? post.audience_label;
  const publishedTopicId = post.published_topic_id?.trim() || post.origin_topic_id?.trim() || null;
  const repostSource = post.repost_of ?? null;
  const isQuoteRepost = post.object_kind === 'repost' && Boolean(post.repost_commentary?.trim());
  const canReply = view.canReply ?? true;
  const canRepost = view.canRepost ?? false;
  const hasPrimaryContent = isPendingText || post.content.trim().length > 0;
  const reactionSummary = post.reaction_summary ?? [];
  const myReactionKeys = useMemo(
    () => new Set((post.my_reactions ?? []).map((reaction) => reaction.normalized_reaction_key)),
    [post.my_reactions]
  );
  const bookmarkedAssetIds = useMemo(
    () => new Set(bookmarkedReactionAssets.map((asset) => asset.asset_id)),
    [bookmarkedReactionAssets]
  );
  const pickerAssets = useMemo(() => {
    const deduped = new Map<string, CustomReactionAssetView>();
    for (const asset of [...ownedReactionAssets, ...bookmarkedReactionAssets]) {
      deduped.set(asset.asset_id, asset);
    }
    return [...deduped.values()];
  }, [bookmarkedReactionAssets, ownedReactionAssets]);

  const openPrimaryTarget = () => {
    const topicId = view.threadTopicId?.trim();
    if (topicId && onOpenThreadInTopic) {
      onOpenThreadInTopic(view.threadTargetId, topicId);
      return;
    }
    onOpenThread(view.threadTargetId);
  };

  return (
    <article
      className={context === 'thread' ? 'post-card post-card-thread post-layout-safe' : 'post-card post-layout-safe'}
    >
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
        <div className='post-link post-layout-safe'>
          <PostMedia media={view.media} />

          <div className='post-body post-layout-safe'>
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
              <strong className='post-title post-copy-wrap'>{post.content}</strong>
            ) : null}

            {repostSource ? (
              <div className='repost-source-card post-layout-safe'>
                <div className='topic-diagnostic topic-diagnostic-secondary'>
                  <span>{isQuoteRepost ? t('feed.quoteRepost') : t('feed.reposted')}</span>
                  <span>{t('labels.sourceTopic')}</span>
                  <span className='shell-topic-link-label' title={repostSource.source_topic_id}>
                    {repostSource.source_topic_id}
                  </span>
                </div>
                <div className='post-body repost-source-body post-layout-safe'>
                  <strong className='post-title post-copy-wrap'>{sourceAuthorLabel(repostSource)}</strong>
                  {repostSource.content.trim().length > 0 ? (
                    <span className='post-copy-wrap'>{repostSource.content}</span>
                  ) : null}
                </div>
              </div>
            ) : null}
          </div>

          <small className='post-copy-wrap'>{post.envelope_id}</small>
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
        <button className='post-link post-layout-safe' type='button' onClick={openPrimaryTarget}>
          <PostMedia media={view.media} />

          <div className='post-body post-layout-safe'>
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
              <strong className='post-title post-copy-wrap'>{post.content}</strong>
            ) : null}

            {repostSource ? (
              <div className='repost-source-card post-layout-safe'>
                <div className='topic-diagnostic topic-diagnostic-secondary'>
                  <span>{isQuoteRepost ? t('feed.quoteRepost') : t('feed.reposted')}</span>
                  <span>{t('labels.sourceTopic')}</span>
                  <span className='shell-topic-link-label' title={repostSource.source_topic_id}>
                    {repostSource.source_topic_id}
                  </span>
                </div>
                <div className='post-body repost-source-body post-layout-safe'>
                  <strong className='post-title post-copy-wrap'>{sourceAuthorLabel(repostSource)}</strong>
                  {repostSource.content.trim().length > 0 ? (
                    <span className='post-copy-wrap'>{repostSource.content}</span>
                  ) : null}
                </div>
              </div>
            ) : null}
          </div>

          <small className='post-copy-wrap'>{post.envelope_id}</small>
          {post.reply_to ? <em className='post-reply-flag'>{t('actions.reply')}</em> : null}
        </button>
      )}

      <div className='post-actions'>
        {readOnly ? (
          publishedTopicId && onOpenOriginalTopic ? (
            <Button
              variant='secondary'
              type='button'
              onClick={() => onOpenOriginalTopic(publishedTopicId)}
            >
              {t('feed.openOriginalTopic', { ns: 'profile' })}
            </Button>
          ) : null
        ) : (
          <>
            {reactionSummary.length > 0 ? (
              <div className='post-reaction-summary'>
                {reactionSummary.map((reaction) => {
                  const reactionKey = reactionKeyInputFromView(reaction);
                  const previewUrl =
                    reaction.custom_asset &&
                    typeof mediaObjectUrls[reaction.custom_asset.blob_hash] === 'string'
                      ? mediaObjectUrls[reaction.custom_asset.blob_hash]
                      : null;
                  const canBookmark =
                    reaction.custom_asset &&
                    reaction.custom_asset.owner_pubkey !== localAuthorPubkey &&
                    !bookmarkedAssetIds.has(reaction.custom_asset.asset_id);
                  return (
                    <span key={reaction.normalized_reaction_key} className='post-reaction-chip-wrap'>
                      <button
                        className={`post-reaction-chip${
                          myReactionKeys.has(reaction.normalized_reaction_key)
                            ? ' post-reaction-chip-active'
                            : ''
                        }`}
                        type='button'
                        onClick={() => {
                          if (reactionKey && onToggleReaction) {
                            onToggleReaction(post, reactionKey);
                          }
                        }}
                      >
                        {previewUrl ? (
                          <img
                            className='post-reaction-chip-image'
                            src={previewUrl}
                            alt={reaction.custom_asset?.asset_id ?? reaction.emoji ?? reaction.normalized_reaction_key}
                          />
                        ) : null}
                        <span>{reaction.emoji ?? reaction.custom_asset?.asset_id.slice(0, 6) ?? '?'}</span>
                        <span>{reaction.count}</span>
                      </button>
                      {canBookmark && reaction.custom_asset && onBookmarkCustomReaction ? (
                        <Button
                          variant='secondary'
                          type='button'
                          onClick={() => onBookmarkCustomReaction(reaction.custom_asset as CustomReactionAssetView)}
                        >
                          {t('common:actions.save')}
                        </Button>
                      ) : null}
                    </span>
                  );
                })}
              </div>
            ) : null}
            <ReactionPickerPopover
              post={post}
              recentReactions={recentReactions}
              assets={pickerAssets}
              mediaObjectUrls={mediaObjectUrls}
              onToggleReaction={onToggleReaction}
            />
            {canRepost && (onRepost || onQuoteRepost) ? (
              <Popover open={repostMenuOpen} onOpenChange={setRepostMenuOpen}>
                <PopoverTrigger asChild>
                  <Button
                    variant='secondary'
                    size='icon'
                    className='post-action-button'
                    type='button'
                    aria-label={t('actions.repost')}
                  >
                    <Repeat2 className='size-4' aria-hidden='true' />
                  </Button>
                </PopoverTrigger>
                <PopoverContent align='end' className='post-action-popover'>
                  <div className='post-action-popover-stack'>
                    {onRepost ? (
                      <Button
                        variant='secondary'
                        type='button'
                        onClick={() => {
                          setRepostMenuOpen(false);
                          onRepost(post);
                        }}
                      >
                        {t('actions.repost')}
                      </Button>
                    ) : null}
                    {onQuoteRepost ? (
                      <Button
                        variant='secondary'
                        type='button'
                        onClick={() => {
                          setRepostMenuOpen(false);
                          onQuoteRepost(post);
                        }}
                      >
                        {t('actions.quoteRepost')}
                      </Button>
                    ) : null}
                  </div>
                </PopoverContent>
              </Popover>
            ) : null}
            {canReply ? (
              <Button
                variant='secondary'
                size='icon'
                className='post-action-button'
                type='button'
                aria-label={t('actions.reply')}
                onClick={() => onReply(post)}
              >
                <Reply className='size-4' aria-hidden='true' />
              </Button>
            ) : null}
            {showBookmarkAction && onToggleBookmark ? (
              <Button
                variant='secondary'
                type='button'
                onClick={() => onToggleBookmark(post)}
              >
                {isBookmarked ? t('common:actions.removeBookmark') : t('common:actions.bookmark')}
              </Button>
            ) : null}
          </>
        )}
      </div>
    </article>
  );
}
