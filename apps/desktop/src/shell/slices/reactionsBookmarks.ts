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
  /** #994: ブックマーク一覧の取得状態。初回は loading、取得成功で ready、初回失敗で error。 */
  bookmarksPanelState: AsyncPanelState;
  recentReactions: RecentReactionView[];
  reactionPanelState: AsyncPanelState;
  reactionCreatePending: boolean;
};

export function createInitialReactionsBookmarksSlice(): ReactionsBookmarksSliceState {
  return {
    ownedReactionAssets: [],
    bookmarkedReactionAssets: [],
    bookmarkedPosts: [],
    bookmarksPanelState: DEFAULT_ASYNC_PANEL_STATE,
    recentReactions: [],
    reactionPanelState: DEFAULT_ASYNC_PANEL_STATE,
    reactionCreatePending: false,
  };
}
