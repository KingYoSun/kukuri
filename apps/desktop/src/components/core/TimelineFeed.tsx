import type * as React from 'react';
import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';

import type {
  BookmarkedCustomReactionView,
  CustomReactionAssetView,
  ReactionKeyInput,
  RecentReactionView,
} from '@/lib/api';
import type { InternalSmartReference } from '@/lib/internalLinks';

import { Button } from '@/components/ui/button';

import { PostCard } from './PostCard';
import { type PostCardView } from './types';

type TimelineFeedProps = {
  posts: PostCardView[];
  emptyCopy: string;
  listClassName?: string;
  itemClassName?: string;
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
  onReactionPickerOpen?: () => void;
  showBookmarkAction?: boolean;
  bookmarkedPostIds?: Set<string>;
  onToggleBookmark?: (post: PostCardView['post']) => void;
  onRetryLocalPost?: (post: PostCardView['post']) => void;
  onRestoreLocalPost?: (post: PostCardView['post']) => void;
  onActivateReference?: (reference: InternalSmartReference) => void;
  onCopyPostLink?: (link: string) => void;
  focusedPostObjectId?: string | null;
  hasMore?: boolean;
  loadingMore?: boolean;
  onLoadMore?: () => void;
  pendingCount?: number;
  onApplyPending?: () => void;
};

export function TimelineFeed({
  posts,
  emptyCopy,
  listClassName = 'post-list',
  itemClassName,
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
  onReactionPickerOpen,
  showBookmarkAction = false,
  bookmarkedPostIds,
  onToggleBookmark,
  onRetryLocalPost,
  onRestoreLocalPost,
  onActivateReference,
  onCopyPostLink,
  focusedPostObjectId,
  hasMore = false,
  loadingMore = false,
  onLoadMore,
  pendingCount = 0,
  onApplyPending,
}: TimelineFeedProps) {
  const { t } = useTranslation('common');
  const loadMoreRef = useRef<HTMLDivElement | null>(null);
  const overscrollAccumulationRef = useRef(0);
  const touchStartYRef = useRef<number | null>(null);
  const canAutoLoad =
    typeof window !== 'undefined' &&
    'IntersectionObserver' in window &&
    typeof onLoadMore === 'function';

  const canApplyPending = pendingCount > 0 && typeof onApplyPending === 'function';

  const handleOverscrollIntent = () => {
    if (!onApplyPending) {
      return;
    }
    onApplyPending();
    overscrollAccumulationRef.current = 0;
  };

  const handleWheel = (event: React.WheelEvent<HTMLUListElement>) => {
    if (!onApplyPending || posts.length === 0) {
      return;
    }
    const target = event.currentTarget;
    if (target.scrollTop > 0 || event.deltaY >= 0) {
      overscrollAccumulationRef.current = 0;
      return;
    }
    overscrollAccumulationRef.current += Math.abs(event.deltaY);
    if (overscrollAccumulationRef.current >= 120) {
      handleOverscrollIntent();
    }
  };

  const handleTouchStart = (event: React.TouchEvent<HTMLUListElement>) => {
    touchStartYRef.current = event.touches[0]?.clientY ?? null;
  };

  const handleTouchMove = (event: React.TouchEvent<HTMLUListElement>) => {
    if (!onApplyPending || posts.length === 0) {
      return;
    }
    const startY = touchStartYRef.current;
    const currentY = event.touches[0]?.clientY ?? null;
    if (startY === null || currentY === null || event.currentTarget.scrollTop > 0) {
      return;
    }
    if (currentY - startY >= 80) {
      touchStartYRef.current = currentY;
      handleOverscrollIntent();
    }
  };

  useEffect(() => {
    if (!canAutoLoad || !hasMore || loadingMore || !loadMoreRef.current) {
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          onLoadMore?.();
        }
      },
      { rootMargin: '200px 0px' }
    );
    observer.observe(loadMoreRef.current);
    return () => observer.disconnect();
  }, [canAutoLoad, hasMore, loadingMore, onLoadMore]);

  if (posts.length === 0 && !canApplyPending) {
    return <p className='empty'>{emptyCopy}</p>;
  }

  return (
    <ul
      className={listClassName}
      onWheel={handleWheel}
      onTouchStart={handleTouchStart}
      onTouchMove={handleTouchMove}
    >
      {canApplyPending ? (
        <li className={itemClassName}>
          <Button
            variant='secondary'
            type='button'
            className='timeline-feed-refresh-banner'
            onClick={() => onApplyPending()}
          >
            {t('feed.pendingPosts', { count: pendingCount })}
          </Button>
        </li>
      ) : null}
      {posts.map((view) => (
        <li key={view.post.object_id} className={itemClassName}>
          <PostCard
            view={view}
            onOpenAuthor={onOpenAuthor}
            onOpenThread={onOpenThread}
            onOpenThreadInTopic={onOpenThreadInTopic}
            onReply={onReply}
            onRepost={onRepost}
            onQuoteRepost={onQuoteRepost}
            readOnly={readOnly}
            onOpenOriginalTopic={onOpenOriginalTopic}
            localAuthorPubkey={localAuthorPubkey}
            mediaObjectUrls={mediaObjectUrls}
            ownedReactionAssets={ownedReactionAssets}
            bookmarkedReactionAssets={bookmarkedReactionAssets}
            recentReactions={recentReactions}
            onToggleReaction={onToggleReaction}
            onBookmarkCustomReaction={onBookmarkCustomReaction}
            onReactionPickerOpen={onReactionPickerOpen}
            showBookmarkAction={showBookmarkAction}
            isBookmarked={bookmarkedPostIds?.has(view.post.object_id) ?? false}
            onToggleBookmark={onToggleBookmark}
            onRetryLocalPost={onRetryLocalPost}
            onRestoreLocalPost={onRestoreLocalPost}
            onActivateReference={onActivateReference}
            onCopyLink={onCopyPostLink}
            isFocused={focusedPostObjectId === view.post.object_id}
          />
        </li>
      ))}
      {hasMore ? (
        <li className={itemClassName}>
          {canAutoLoad ? <div ref={loadMoreRef} aria-hidden='true' /> : null}
          {!canAutoLoad && onLoadMore ? (
            <Button variant='secondary' type='button' onClick={() => onLoadMore()}>
              {loadingMore ? 'Loading...' : 'Load more'}
            </Button>
          ) : null}
          {canAutoLoad && loadingMore ? <p className='empty'>Loading more…</p> : null}
        </li>
      ) : null}
    </ul>
  );
}
