import { useMemo } from 'react';

import type {
  BookmarkedCustomReactionView,
  CustomReactionAssetView,
  ReactionKeyInput,
  RecentReactionView,
} from '@/lib/api';
import type { InternalSmartReference } from '@/lib/internalLinks';

import { Button } from '@/components/ui/button';

import { buildThreadTree } from './buildThreadTree';
import { PostCard } from './PostCard';
import { type PostCardView } from './types';
import { useInfiniteScrollSentinel } from './useInfiniteScrollSentinel';

const MAX_VISUAL_DEPTH = 6;

type ThreadTreeProps = {
  posts: PostCardView[];
  emptyCopy: string;
  onOpenAuthor: (authorPubkey: string) => void;
  onOpenThread: (threadId: string) => void;
  onOpenThreadInTopic?: (threadId: string, topicId: string) => void;
  onReply: (post: PostCardView['post']) => void;
  onRepost?: (post: PostCardView['post']) => void;
  onQuoteRepost?: (post: PostCardView['post']) => void;
  localAuthorPubkey?: string;
  mediaObjectUrls?: Record<string, string | null>;
  ownedReactionAssets?: CustomReactionAssetView[];
  bookmarkedReactionAssets?: BookmarkedCustomReactionView[];
  recentReactions?: RecentReactionView[];
  onToggleReaction?: (post: PostCardView['post'], reactionKey: ReactionKeyInput) => void;
  onBookmarkCustomReaction?: (asset: CustomReactionAssetView) => void;
  onReactionPickerOpen?: () => void;
  onRetryLocalPost?: (post: PostCardView['post']) => void;
  onRestoreLocalPost?: (post: PostCardView['post']) => void;
  onActivateReference?: (reference: InternalSmartReference) => void;
  onCopyPostLink?: (link: string) => void;
  focusedPostObjectId?: string | null;
  hasMore?: boolean;
  loadingMore?: boolean;
  onLoadMore?: () => void;
};

export function ThreadTree({
  posts,
  emptyCopy,
  onOpenAuthor,
  onOpenThread,
  onOpenThreadInTopic,
  onReply,
  onRepost,
  onQuoteRepost,
  localAuthorPubkey,
  mediaObjectUrls = {},
  ownedReactionAssets = [],
  bookmarkedReactionAssets = [],
  recentReactions = [],
  onToggleReaction,
  onBookmarkCustomReaction,
  onReactionPickerOpen,
  onRetryLocalPost,
  onRestoreLocalPost,
  onActivateReference,
  onCopyPostLink,
  focusedPostObjectId,
  hasMore = false,
  loadingMore = false,
  onLoadMore,
}: ThreadTreeProps) {
  const nodes = useMemo(() => buildThreadTree(posts), [posts]);
  const { sentinelRef: loadMoreRef, canAutoLoad } = useInfiniteScrollSentinel({
    hasMore,
    loadingMore,
    onLoadMore,
  });

  if (nodes.length === 0) {
    return <p className='empty'>{emptyCopy}</p>;
  }

  return (
    <ul className='thread-tree'>
      {nodes.map(({ view, depth, rails, isLast }) => {
        const visualDepth = Math.min(depth, MAX_VISUAL_DEPTH);
        const visibleRails = rails.slice(0, Math.max(0, visualDepth - 1));
        return (
          <li key={view.post.object_id} className='thread-tree-item' data-depth={visualDepth}>
            {visualDepth > 0 ? (
              <span className='thread-tree-rails' aria-hidden='true'>
                {visibleRails.map((continues, railIndex) => (
                  <span
                    key={`${view.post.object_id}-rail-${railIndex}`}
                    className={continues ? 'thread-rail thread-rail-line' : 'thread-rail'}
                  />
                ))}
                <span
                  className='thread-rail thread-rail-elbow'
                  data-last={isLast ? 'true' : 'false'}
                />
              </span>
            ) : null}
            <div className='thread-tree-body'>
            <PostCard
              view={view}
              onOpenAuthor={onOpenAuthor}
              onOpenThread={onOpenThread}
              onOpenThreadInTopic={onOpenThreadInTopic}
              onReply={onReply}
              onRepost={onRepost}
              onQuoteRepost={onQuoteRepost}
              localAuthorPubkey={localAuthorPubkey}
              mediaObjectUrls={mediaObjectUrls}
              ownedReactionAssets={ownedReactionAssets}
              bookmarkedReactionAssets={bookmarkedReactionAssets}
              recentReactions={recentReactions}
              onToggleReaction={onToggleReaction}
              onBookmarkCustomReaction={onBookmarkCustomReaction}
              onReactionPickerOpen={onReactionPickerOpen}
              onRetryLocalPost={onRetryLocalPost}
              onRestoreLocalPost={onRestoreLocalPost}
              onActivateReference={onActivateReference}
              onCopyLink={onCopyPostLink}
              isFocused={focusedPostObjectId === view.post.object_id}
            />
            </div>
          </li>
        );
      })}
      {hasMore ? (
        <li className='thread-tree-item' data-depth={0}>
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
