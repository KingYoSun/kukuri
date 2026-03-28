import type {
  BookmarkedCustomReactionView,
  CustomReactionAssetView,
  ReactionKeyInput,
} from '@/lib/api';

import { TimelineFeed } from './TimelineFeed';
import { type PostCardView, type ThreadPanelState } from './types';

type ThreadPanelProps = {
  state: ThreadPanelState;
  posts: PostCardView[];
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
  onToggleReaction?: (post: PostCardView['post'], reactionKey: ReactionKeyInput) => void;
  onBookmarkCustomReaction?: (asset: CustomReactionAssetView) => void;
  onManageReactions?: () => void;
};

export function ThreadPanel({
  state,
  posts,
  onOpenAuthor,
  onOpenThread,
  onOpenThreadInTopic,
  onReply,
  onRepost,
  onQuoteRepost,
  localAuthorPubkey,
  mediaObjectUrls,
  ownedReactionAssets,
  bookmarkedReactionAssets,
  onToggleReaction,
  onBookmarkCustomReaction,
  onManageReactions,
}: ThreadPanelProps) {
  return (
    <div className='shell-main-stack'>
      <TimelineFeed
        posts={posts}
        emptyCopy={state.emptyCopy}
        listClassName='thread-list'
        itemClassName='thread-item'
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
        onToggleReaction={onToggleReaction}
        onBookmarkCustomReaction={onBookmarkCustomReaction}
        onManageReactions={onManageReactions}
      />
    </div>
  );
}
