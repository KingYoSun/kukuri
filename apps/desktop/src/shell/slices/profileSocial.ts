import { type ProfileConnectionsView } from '@/components/shell/types';
import type {
  AuthorSocialView,
  PostView,
  Profile,
  ProfileInput,
  TimelineCursor,
} from '@/lib/api';

import { type AsyncPanelState, DEFAULT_ASYNC_PANEL_STATE } from '@/shell/slices/shared';

/// プロフィール・作者詳細・ソーシャルグラフ(WP-H6 PR3 のドメインスライス)。

export type SocialConnectionsState = Record<ProfileConnectionsView, AuthorSocialView[]>;
export type KnownAuthorsByPubkey = Record<string, AuthorSocialView>;

export type ProfileSocialSliceState = {
  localProfile: Profile | null;
  profileTimeline: PostView[];
  profileTimelineNextCursor: TimelineCursor | null;
  profileTimelineLoadingMore: boolean;
  knownAuthorsByPubkey: KnownAuthorsByPubkey;
  socialConnections: SocialConnectionsState;
  socialConnectionsPanelState: AsyncPanelState;
  profileDraft: ProfileInput;
  profileDirty: boolean;
  profileError: string | null;
  profilePanelState: AsyncPanelState;
  profileSaving: boolean;
  selectedAuthorPubkey: string | null;
  selectedAuthor: AuthorSocialView | null;
  selectedAuthorTimeline: PostView[];
  authorTimelinesByPubkey: Record<string, PostView[]>;
  authorTimelineNextCursorByPubkey: Record<string, TimelineCursor | null>;
  authorErrorsByPubkey: Record<string, string | null>;
  selectedAuthorTimelineNextCursor: TimelineCursor | null;
  selectedAuthorTimelineLoadingMore: boolean;
  authorError: string | null;
};

export const DEFAULT_SOCIAL_CONNECTIONS: SocialConnectionsState = {
  following: [],
  followed: [],
  muted: [],
};

export function createInitialProfileSocialSlice(): ProfileSocialSliceState {
  return {
    localProfile: null,
    profileTimeline: [],
    profileTimelineNextCursor: null,
    profileTimelineLoadingMore: false,
    knownAuthorsByPubkey: {},
    socialConnections: DEFAULT_SOCIAL_CONNECTIONS,
    socialConnectionsPanelState: DEFAULT_ASYNC_PANEL_STATE,
    profileDraft: {},
    profileDirty: false,
    profileError: null,
    profilePanelState: DEFAULT_ASYNC_PANEL_STATE,
    profileSaving: false,
    selectedAuthorPubkey: null,
    selectedAuthor: null,
    selectedAuthorTimeline: [],
    authorTimelinesByPubkey: {},
    authorTimelineNextCursorByPubkey: {},
    authorErrorsByPubkey: {},
    selectedAuthorTimelineNextCursor: null,
    selectedAuthorTimelineLoadingMore: false,
    authorError: null,
  };
}
