import type {
  BookmarkedCustomReactionView,
  BookmarkedPostView,
  CustomReactionAssetView,
  RecentReactionView,
} from '@/lib/api';

import { type AsyncPanelState, DEFAULT_ASYNC_PANEL_STATE } from '@/shell/slices/shared';

/// リアクション資産・ブックマーク(WP-H6 PR3 のドメインスライス)。
export type ReactionsBookmarksSliceState = {
  ownedReactionAssets: CustomReactionAssetView[];
  bookmarkedReactionAssets: BookmarkedCustomReactionView[];
  bookmarkedPosts: BookmarkedPostView[];
  recentReactions: RecentReactionView[];
  reactionPanelState: AsyncPanelState;
  reactionCreatePending: boolean;
};

export function createInitialReactionsBookmarksSlice(): ReactionsBookmarksSliceState {
  return {
    ownedReactionAssets: [],
    bookmarkedReactionAssets: [],
    bookmarkedPosts: [],
    recentReactions: [],
    reactionPanelState: DEFAULT_ASYNC_PANEL_STATE,
    reactionCreatePending: false,
  };
}
