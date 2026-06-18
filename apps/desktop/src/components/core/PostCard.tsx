import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Bookmark, Link2, Reply, Repeat2 } from 'lucide-react';

import { formatLocalizedTime } from '@/i18n/format';
import type {
  BookmarkedCustomReactionView,
  CustomReactionAssetView,
  ReactionKeyInput,
  ReactionKeyView,
  RecentReactionView,
} from '@/lib/api';
import { copyTextToClipboard } from '@/lib/utils';
import {
  buildPostLink,
  type InternalSmartReference,
} from '@/lib/internalLinks';

import { Button } from '@/components/ui/button';
import {
  ContextActionMenu,
  type ContextActionMenuPosition,
} from '@/components/ui/context-action-menu';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';

import { AuthorAvatar } from './AuthorAvatar';
import { AuthorIdentityButton } from './AuthorIdentityButton';
import { MediaViewerDialog } from './MediaViewerDialog';
import { PostMedia } from './PostMedia';
import { ReactionPickerPopover } from './ReactionPickerPopover';
import { RelationshipBadge } from './RelationshipBadge';
import { SmartReferenceText } from './SmartReferenceText';
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
  onRetryLocalPost?: (post: PostCardView['post']) => void;
  onRestoreLocalPost?: (post: PostCardView['post']) => void;
  onReactionPickerOpen?: () => void;
  onActivateReference?: (reference: InternalSmartReference) => void;
  onCopyLink?: (link: string) => void;
  isFocused?: boolean;
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
  onRetryLocalPost,
  onRestoreLocalPost,
  onReactionPickerOpen,
  onActivateReference,
  onCopyLink,
  isFocused = false,
}: PostCardProps) {
  const { t } = useTranslation(['common', 'profile']);
  const { post, context } = view;
  const [repostMenuOpen, setRepostMenuOpen] = useState(false);
  const [mediaViewerOpen, setMediaViewerOpen] = useState(false);
  const [mediaViewerIndex, setMediaViewerIndex] = useState(view.media.currentImageIndex ?? 0);
  const [reactionMenuPosition, setReactionMenuPosition] = useState<ContextActionMenuPosition | null>(
    null
  );
  const [reactionMenuAsset, setReactionMenuAsset] = useState<CustomReactionAssetView | null>(null);
  const isPendingText = post.content_status === 'Missing' && post.content === '[blob pending]';
  const localState = post.local_state ?? null;
  const interactionDisabled = localState !== null;
  const audienceChipLabel = view.audienceChipLabel ?? post.audience_label;
  const publishedTopicId = post.published_topic_id?.trim() || post.origin_topic_id?.trim() || null;
  const canonicalPostTopicId = view.threadTopicId?.trim() || publishedTopicId;
  const canonicalPostLink = canonicalPostTopicId
    ? buildPostLink(canonicalPostTopicId, view.threadTargetId, post.object_id)
    : null;
  const repostSource = post.repost_of ?? null;
  const replyPreview = post.reply_preview ?? null;
  const isQuoteRepost = post.object_kind === 'repost' && Boolean(post.repost_commentary?.trim());
  const isPureRepost = post.object_kind === 'repost' && !isQuoteRepost;
  // X-style: a pure repost renders the original post as the primary content, with the
  // reposter demoted to a small attribution header above the (source) author identity.
  const showRepostAsPrimary = isPureRepost && repostSource !== null && view.repostSourceAuthor != null;
  const showReplyContext = replyPreview !== null && !view.suppressReplyPreview;
  const primaryAuthor =
    showRepostAsPrimary && view.repostSourceAuthor
      ? view.repostSourceAuthor
      : { pubkey: post.author_pubkey, label: view.authorLabel, picture: view.authorPicture ?? null };
  const canReply = view.canReply ?? true;
  const canRepost = view.canRepost ?? false;
  const localStateLabel =
    localState === 'pending'
      ? t('feed.localPosting')
      : localState === 'syncing'
        ? t('feed.localSyncing')
        : localState === 'failed'
          ? t('feed.localFailed')
          : null;
  const primaryContent = showRepostAsPrimary && repostSource ? repostSource.content : post.content;
  const hasPrimaryContent = isPendingText || primaryContent.trim().length > 0;
  const reactionSummary = post.reaction_summary ?? [];
  const myReactionKeys = useMemo(
    () => new Set((post.my_reactions ?? []).map((reaction) => reaction.normalized_reaction_key)),
    [post.my_reactions]
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

  const reactionMenuItems = useMemo(() => {
    if (!reactionMenuAsset) {
      return [];
    }
    const canSaveReaction =
      Boolean(onBookmarkCustomReaction) &&
      reactionMenuAsset.owner_pubkey !== localAuthorPubkey;
    return [
      {
        id: 'save',
        label: t('actions.save'),
        disabled: !canSaveReaction,
        onSelect: async () => {
          if (canSaveReaction && onBookmarkCustomReaction) {
            onBookmarkCustomReaction(reactionMenuAsset);
          }
        },
      },
      {
        id: 'copy-hash',
        label: t('actions.copyHash'),
        onSelect: async () => {
          await copyTextToClipboard(reactionMenuAsset.blob_hash);
        },
      },
    ];
  }, [localAuthorPubkey, onBookmarkCustomReaction, reactionMenuAsset, t]);

  const renderReferencedCard = (
    source:
      | {
          authorLabel: string | null;
          content: string;
          topic: string;
          attachments: { hash: string }[];
          replyTo?: string | null;
        }
      | null,
    eyebrow: string,
    author?: PostCardView['repostSourceAuthor']
  ) => {
    if (!source) {
      return null;
    }
    return (
      <div className='repost-source-card post-layout-safe'>
        <div className='repost-source-meta'>
          <span className='repost-source-eyebrow'>{eyebrow}</span>
          <span className='repost-source-topic'>
            <span>{t('labels.sourceTopic')}</span>
            <SmartReferenceText
              text={source.topic}
              className='shell-topic-link-label'
              onActivateReference={onActivateReference}
            />
          </span>
          {source.attachments.length > 0 ? (
            <span className='repost-source-attachments'>{`+${source.attachments.length} media`}</span>
          ) : null}
        </div>
        <div className='post-body repost-source-body post-layout-safe'>
          {author ? (
            <button
              type='button'
              className='repost-source-author author-link'
              onClick={(event) => {
                event.stopPropagation();
                onOpenAuthor(author.pubkey);
              }}
            >
              <AuthorAvatar label={author.label} picture={author.picture ?? null} size='sm' />
              <span>{author.label}</span>
            </button>
          ) : source.authorLabel ? (
            <strong className='post-title post-copy-wrap'>{source.authorLabel}</strong>
          ) : null}
          {source.content.trim().length > 0 ? (
            <SmartReferenceText
              text={source.content}
              className='post-copy-wrap'
              onActivateReference={onActivateReference}
              mentionAuthors={view.mentionAuthors}
              onOpenMention={onOpenAuthor}
            />
          ) : null}
        </div>
      </div>
    );
  };

  const contentBlock = (
    <>
      <div className='post-body post-layout-safe'>
        {showReplyContext && view.replyParentAuthor && replyPreview ? (
          <div className='post-reply-context'>
            <button
              type='button'
              className='post-reply-context-avatar'
              aria-label={view.replyParentAuthor.label}
              onClick={(event) => {
                event.stopPropagation();
                onOpenAuthor(view.replyParentAuthor!.pubkey);
              }}
            >
              <AuthorAvatar
                label={view.replyParentAuthor.label}
                picture={view.replyParentAuthor.picture ?? null}
                size='sm'
              />
            </button>
            <div className='post-reply-context-main'>
              <button
                type='button'
                className='post-reply-context-author author-link'
                onClick={(event) => {
                  event.stopPropagation();
                  onOpenAuthor(view.replyParentAuthor!.pubkey);
                }}
              >
                {t('feed.replyingTo', { author: view.replyParentAuthor.label })}
              </button>
              {replyPreview.content.trim().length > 0 ? (
                <div className='post-reply-context-body post-copy-wrap'>
                  <SmartReferenceText
                    text={replyPreview.content}
                    className='post-copy-wrap'
                    onActivateReference={onActivateReference}
                    mentionAuthors={view.mentionAuthors}
                    onOpenMention={onOpenAuthor}
                  />
                </div>
              ) : null}
            </div>
          </div>
        ) : null}

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
          <strong className='post-title post-copy-wrap'>
            <SmartReferenceText
              text={primaryContent}
              className='post-copy-wrap'
              onActivateReference={onActivateReference}
              mentionAuthors={view.mentionAuthors}
              onOpenMention={onOpenAuthor}
            />
          </strong>
        ) : null}

        {showRepostAsPrimary && repostSource ? (
          <div className='post-source-topic'>
            <span>{t('labels.sourceTopic')}</span>
            <SmartReferenceText
              text={repostSource.source_topic_id}
              className='shell-topic-link-label'
              onActivateReference={onActivateReference}
            />
            {repostSource.attachments.length > 0 ? (
              <span className='repost-source-attachments'>{`+${repostSource.attachments.length} media`}</span>
            ) : null}
          </div>
        ) : repostSource ? (
          renderReferencedCard(
            {
              authorLabel: sourceAuthorLabel(repostSource),
              content: repostSource.content,
              topic: repostSource.source_topic_id,
              attachments: repostSource.attachments,
              replyTo: repostSource.reply_to ?? null,
            },
            isQuoteRepost ? t('feed.quoteRepost') : t('feed.reposted'),
            view.repostSourceAuthor
          )
        ) : null}
      </div>

      <small className='post-copy-wrap'>{post.envelope_id}</small>
      {readOnly && publishedTopicId ? (
        <div className='topic-diagnostic topic-diagnostic-secondary'>
          <span>{t('feed.originTopic', { ns: 'profile' })}</span>
          <SmartReferenceText
            text={publishedTopicId}
            className='shell-topic-link-label'
            onActivateReference={onActivateReference}
          />
        </div>
      ) : null}
    </>
  );

  return (
    <article
      className={
        context === 'thread'
          ? `post-card post-card-thread post-layout-safe${isFocused ? ' post-card-targeted' : ''}`
          : `post-card post-layout-safe${isFocused ? ' post-card-targeted' : ''}`
      }
      aria-busy={localState === 'pending' || localState === 'syncing'}
      data-post-object-id={post.object_id}
      tabIndex={isFocused ? -1 : undefined}
    >
      {showRepostAsPrimary ? (
        <button
          type='button'
          className='post-repost-attribution author-link'
          onClick={() => onOpenAuthor(post.author_pubkey)}
        >
          <Repeat2 className='size-3.5' aria-hidden='true' />
          <AuthorAvatar
            label={view.authorLabel}
            picture={view.authorPicture ?? null}
            size='sm'
            className='post-repost-attribution-avatar'
          />
          <span>{t('feed.repostedBy', { author: view.authorLabel })}</span>
        </button>
      ) : null}

      <div className='post-meta'>
        <AuthorIdentityButton
          label={primaryAuthor.label}
          picture={primaryAuthor.picture ?? null}
          avatarTestId={`${post.object_id}-author-avatar`}
          onClick={() => onOpenAuthor(primaryAuthor.pubkey)}
        />
        <div className='post-meta-trailing'>
          <RelationshipBadge label={view.relationshipLabel} />
          <span className='post-meta-chip'>{audienceChipLabel}</span>
          <span>{formatLocalizedTime(post.created_at * 1000)}</span>
        </div>
      </div>

      {view.media.kind ? (
        <PostMedia
          media={view.media}
          onOpenImage={(index) => {
            setMediaViewerIndex(index);
            setMediaViewerOpen(true);
          }}
        />
      ) : null}

      {readOnly ? (
        <div className='post-link post-layout-safe'>{contentBlock}</div>
      ) : (
        <div
          className='post-link post-layout-safe'
          role='button'
          tabIndex={0}
          onClick={openPrimaryTarget}
          onKeyDown={(event) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              openPrimaryTarget();
            }
          }}
        >
          {contentBlock}
        </div>
      )}

      {localStateLabel ? (
        <div className='topic-diagnostic topic-diagnostic-secondary' aria-live='polite'>
          <span>{localStateLabel}</span>
          {post.local_error ? <span>{post.local_error}</span> : null}
          {localState === 'failed' && onRetryLocalPost ? (
            <Button variant='secondary' type='button' onClick={() => onRetryLocalPost(post)}>
              {t('actions.retry')}
            </Button>
          ) : null}
          {localState === 'failed' && onRestoreLocalPost ? (
            <Button variant='secondary' type='button' onClick={() => onRestoreLocalPost(post)}>
              {t('actions.restoreDraft')}
            </Button>
          ) : null}
        </div>
      ) : null}

      <div className='post-actions'>
        {readOnly ? (
          <>
            {publishedTopicId && onOpenOriginalTopic ? (
              <Button
                variant='secondary'
                type='button'
                onClick={() => onOpenOriginalTopic(publishedTopicId)}
              >
                {t('feed.openOriginalTopic', { ns: 'profile' })}
              </Button>
            ) : null}
            {canonicalPostLink && onCopyLink ? (
              <Button
                variant='secondary'
                size='icon'
                className='post-action-button'
                type='button'
                aria-label={t('actions.copyLink')}
                onClick={() => onCopyLink(canonicalPostLink)}
              >
                <Link2 className='size-4' aria-hidden='true' />
              </Button>
            ) : null}
          </>
        ) : (
          <>
            {reactionSummary.length > 0 && !interactionDisabled ? (
              <div className='post-reaction-summary'>
                {reactionSummary.map((reaction) => {
                  const reactionKey = reactionKeyInputFromView(reaction);
                  const previewUrl =
                    reaction.custom_asset &&
                    typeof mediaObjectUrls[reaction.custom_asset.blob_hash] === 'string'
                      ? mediaObjectUrls[reaction.custom_asset.blob_hash]
                      : null;
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
                        onContextMenu={(event) => {
                          if (!reaction.custom_asset) {
                            return;
                          }
                          event.preventDefault();
                          setReactionMenuAsset(reaction.custom_asset);
                          setReactionMenuPosition({
                            x: event.clientX,
                            y: event.clientY,
                          });
                        }}
                      >
                        {previewUrl ? (
                          <img
                            className='post-reaction-chip-image'
                            src={previewUrl}
                            alt={
                              reaction.custom_asset?.asset_id ??
                              reaction.emoji ??
                              reaction.normalized_reaction_key
                            }
                          />
                        ) : null}
                        <span>
                          {reaction.emoji ?? reaction.custom_asset?.asset_id.slice(0, 6) ?? '?'}
                        </span>
                        <span>{reaction.count}</span>
                      </button>
                    </span>
                  );
                })}
              </div>
            ) : null}
            {!interactionDisabled ? (
              <ReactionPickerPopover
                post={post}
                recentReactions={recentReactions}
                assets={pickerAssets}
                mediaObjectUrls={mediaObjectUrls}
                onToggleReaction={onToggleReaction}
                onOpen={() => onReactionPickerOpen?.()}
              />
            ) : null}
            {!interactionDisabled && canRepost && (onRepost || onQuoteRepost) ? (
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
            {canReply && !interactionDisabled ? (
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
            {canonicalPostLink && onCopyLink ? (
              <Button
                variant='secondary'
                size='icon'
                className='post-action-button'
                type='button'
                aria-label={t('actions.copyLink')}
                onClick={() => onCopyLink(canonicalPostLink)}
              >
                <Link2 className='size-4' aria-hidden='true' />
              </Button>
            ) : null}
            {showBookmarkAction && onToggleBookmark ? (
              <Button
                variant='secondary'
                size='icon'
                className={`post-action-button${isBookmarked ? ' post-action-button-active' : ''}`}
                type='button'
                aria-label={isBookmarked ? t('actions.removeBookmark') : t('actions.bookmark')}
                aria-pressed={isBookmarked}
                onClick={() => onToggleBookmark(post)}
              >
                <Bookmark
                  className='size-4'
                  fill={isBookmarked ? 'currentColor' : 'none'}
                  aria-hidden='true'
                />
              </Button>
            ) : null}
          </>
        )}
      </div>

      <MediaViewerDialog
        items={view.media.imageGalleryItems ?? []}
        index={mediaViewerIndex}
        open={mediaViewerOpen}
        onOpenChange={setMediaViewerOpen}
        onIndexChange={setMediaViewerIndex}
      />
      <ContextActionMenu
        open={reactionMenuAsset !== null}
        position={reactionMenuPosition}
        items={reactionMenuItems}
        onClose={() => {
          setReactionMenuAsset(null);
          setReactionMenuPosition(null);
        }}
      />
    </article>
  );
}
