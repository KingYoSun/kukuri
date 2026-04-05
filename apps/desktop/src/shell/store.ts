import { createContext, useCallback, useContext } from 'react';
import { useStore } from 'zustand';
import { createStore } from 'zustand/vanilla';

import {
  type ChannelAudienceOption,
  type ExtendedPanelStatus,
  type InviteOutputLabel,
  type PrivateChannelPendingAction,
} from '@/components/extended/types';
import {
  type ProfileConnectionsView,
  type ShellChromeState,
} from '@/components/shell/types';
import {
  type AuthorSocialView,
  type BookmarkedCustomReactionView,
  type BookmarkedPostView,
  type ChannelRef,
  type CommunityNodeConfig,
  type CommunityNodeNodeStatus,
  type CreateAttachmentInput,
  type CustomReactionAssetView,
  type DesktopApi,
  type DirectMessageConversationView,
  type DirectMessageMessageView,
  type DirectMessageStatusView,
  type DiscoveryConfig,
  type GameRoomStatus,
  type GameRoomView,
  type JoinedPrivateChannelView,
  type LiveSessionView,
  type PostView,
  type Profile,
  type ProfileInput,
  type RecentReactionView,
  type SyncStatus,
  type TimelineScope,
} from '@/lib/api';
import type { DesktopTheme } from '@/lib/theme';

export type AppProps = {
  api?: DesktopApi;
};

export type DraftMediaItem = {
  id: string;
  source_name: string;
  preview_url: string;
  attachments: CreateAttachmentInput[];
};

export type GameEditorDraft = {
  status: GameRoomStatus;
  phase_label: string;
  scores: Record<string, string>;
};

export type AsyncPanelState = {
  status: ExtendedPanelStatus;
  error: string | null;
};

export type SocialConnectionsState = Record<ProfileConnectionsView, AuthorSocialView[]>;
export type KnownAuthorsByPubkey = Record<string, AuthorSocialView>;

export type DesktopShellState = {
  trackedTopics: string[];
  activeTopic: string;
  topicInput: string;
  composer: string;
  draftMediaItems: DraftMediaItem[];
  attachmentInputKey: number;
  timelinesByTopic: Record<string, PostView[]>;
  publicTimelinesByTopic: Record<string, PostView[]>;
  liveSessionsByTopic: Record<string, LiveSessionView[]>;
  gameRoomsByTopic: Record<string, GameRoomView[]>;
  joinedChannelsByTopic: Record<string, JoinedPrivateChannelView[]>;
  selectedChannelIdByTopic: Record<string, string | null>;
  timelineScopeByTopic: Record<string, TimelineScope>;
  composeChannelByTopic: Record<string, ChannelRef>;
  thread: PostView[];
  selectedThread: string | null;
  replyTarget: PostView | null;
  repostTarget: PostView | null;
  peerTicket: string;
  localPeerTicket: string | null;
  discoveryConfig: DiscoveryConfig;
  discoverySeedInput: string;
  discoveryEditorDirty: boolean;
  discoveryError: string | null;
  communityNodeConfig: CommunityNodeConfig;
  communityNodeStatuses: CommunityNodeNodeStatus[];
  communityNodeInput: string;
  communityNodeEditorDirty: boolean;
  communityNodeError: string | null;
  mediaObjectUrls: Record<string, string | null>;
  unsupportedVideoManifests: Record<string, true>;
  syncStatus: SyncStatus;
  localProfile: Profile | null;
  profileTimeline: PostView[];
  knownAuthorsByPubkey: KnownAuthorsByPubkey;
  socialConnections: SocialConnectionsState;
  socialConnectionsPanelState: AsyncPanelState;
  ownedReactionAssets: CustomReactionAssetView[];
  bookmarkedReactionAssets: BookmarkedCustomReactionView[];
  bookmarkedPosts: BookmarkedPostView[];
  recentReactions: RecentReactionView[];
  profileDraft: ProfileInput;
  profileDirty: boolean;
  profileError: string | null;
  profilePanelState: AsyncPanelState;
  profileSaving: boolean;
  selectedAuthorPubkey: string | null;
  selectedAuthor: AuthorSocialView | null;
  selectedAuthorTimeline: PostView[];
  authorError: string | null;
  directMessagePaneOpen: boolean;
  selectedDirectMessagePeerPubkey: string | null;
  directMessages: DirectMessageConversationView[];
  directMessageTimelineByPeer: Record<string, DirectMessageMessageView[]>;
  directMessageStatusByPeer: Record<string, DirectMessageStatusView>;
  directMessageComposer: string;
  directMessageDraftMediaItems: DraftMediaItem[];
  directMessageAttachmentInputKey: number;
  directMessageError: string | null;
  directMessageSending: boolean;
  composerError: string | null;
  liveTitle: string;
  liveDescription: string;
  liveError: string | null;
  livePanelStateByTopic: Record<string, AsyncPanelState>;
  liveCreatePending: boolean;
  livePendingBySessionId: Record<string, true>;
  channelLabelInput: string;
  channelAudienceInput: ChannelAudienceOption['value'];
  inviteTokenInput: string;
  inviteOutput: string | null;
  inviteOutputLabel: InviteOutputLabel;
  channelError: string | null;
  channelPanelStateByTopic: Record<string, AsyncPanelState>;
  channelActionPending: PrivateChannelPendingAction;
  gameTitle: string;
  gameDescription: string;
  gameParticipantsInput: string;
  gameError: string | null;
  gameDrafts: Record<string, GameEditorDraft>;
  gamePanelStateByTopic: Record<string, AsyncPanelState>;
  gameCreatePending: boolean;
  gameSavingByRoomId: Record<string, true>;
  reactionPanelState: AsyncPanelState;
  reactionCreatePending: boolean;
  error: string | null;
  shellChromeState: ShellChromeState;
};

export type DesktopShellStateValue<K extends keyof DesktopShellState> =
  | DesktopShellState[K]
  | ((current: DesktopShellState[K]) => DesktopShellState[K]);

export type DesktopShellStore = DesktopShellState & {
  patchState: (patch: Partial<DesktopShellState>) => void;
  resetState: () => void;
  setField: <K extends keyof DesktopShellState>(
    key: K,
    value: DesktopShellStateValue<K>
  ) => void;
};

export type DesktopShellPageProps = AppProps & {
  theme: DesktopTheme;
  onThemeChange: (theme: DesktopTheme) => void;
};

export const DEFAULT_TOPIC = 'kukuri:topic:demo';
export const PUBLIC_CHANNEL_REF: ChannelRef = { kind: 'public' };
export const PUBLIC_TIMELINE_SCOPE: TimelineScope = { kind: 'public' };
export const REFRESH_INTERVAL_MS = 2000;
export const VIDEO_POSTER_TIMEOUT_MS = 5000;
export const MEDIA_DEBUG_STORAGE_KEY = 'kukuri:media-debug';
export const SHELL_WORKSPACE_ID = 'shell-primary-workspace';
export const SHELL_NAV_ID = 'shell-nav-rail';
export const SHELL_CONTEXT_ID = 'shell-context-pane';
export const SHELL_SETTINGS_ID = 'shell-settings-drawer';

export const DEFAULT_ASYNC_PANEL_STATE: AsyncPanelState = {
  status: 'loading',
  error: null,
};

export const DEFAULT_DISCOVERY_CONFIG: DiscoveryConfig = {
  mode: 'seeded_dht',
  connect_mode: 'direct_only',
  env_locked: false,
  seed_peers: [],
};

export const DEFAULT_COMMUNITY_NODE_CONFIG: CommunityNodeConfig = {
  nodes: [],
};

export const DEFAULT_SOCIAL_CONNECTIONS: SocialConnectionsState = {
  following: [],
  followed: [],
  muted: [],
};

export const DEFAULT_SYNC_STATUS: SyncStatus = {
  connected: false,
  peer_count: 0,
  pending_events: 0,
  status_detail: '',
  last_error: null,
  configured_peers: [],
  subscribed_topics: [],
  topic_diagnostics: [],
  local_author_pubkey: '',
  discovery: {
    mode: 'seeded_dht',
    connect_mode: 'direct_only',
    env_locked: false,
    configured_seed_peer_ids: [],
    bootstrap_seed_peer_ids: [],
    manual_ticket_peer_ids: [],
    connected_peer_ids: [],
    assist_peer_ids: [],
    local_endpoint_id: '',
    last_discovery_error: null,
  },
};

export function createInitialShellState(): DesktopShellState {
  return {
    trackedTopics: [DEFAULT_TOPIC],
    activeTopic: DEFAULT_TOPIC,
    topicInput: '',
    composer: '',
    draftMediaItems: [],
    attachmentInputKey: 0,
    timelinesByTopic: {
      [DEFAULT_TOPIC]: [],
    },
    publicTimelinesByTopic: {
      [DEFAULT_TOPIC]: [],
    },
    liveSessionsByTopic: {
      [DEFAULT_TOPIC]: [],
    },
    gameRoomsByTopic: {
      [DEFAULT_TOPIC]: [],
    },
    joinedChannelsByTopic: {
      [DEFAULT_TOPIC]: [],
    },
    selectedChannelIdByTopic: {
      [DEFAULT_TOPIC]: null,
    },
    timelineScopeByTopic: {
      [DEFAULT_TOPIC]: PUBLIC_TIMELINE_SCOPE,
    },
    composeChannelByTopic: {
      [DEFAULT_TOPIC]: PUBLIC_CHANNEL_REF,
    },
    thread: [],
    selectedThread: null,
    replyTarget: null,
    repostTarget: null,
    peerTicket: '',
    localPeerTicket: null,
    discoveryConfig: DEFAULT_DISCOVERY_CONFIG,
    discoverySeedInput: '',
    discoveryEditorDirty: false,
    discoveryError: null,
    communityNodeConfig: DEFAULT_COMMUNITY_NODE_CONFIG,
    communityNodeStatuses: [],
    communityNodeInput: '',
    communityNodeEditorDirty: false,
    communityNodeError: null,
    mediaObjectUrls: {},
    unsupportedVideoManifests: {},
    syncStatus: DEFAULT_SYNC_STATUS,
    localProfile: null,
    profileTimeline: [],
    knownAuthorsByPubkey: {},
    socialConnections: DEFAULT_SOCIAL_CONNECTIONS,
    socialConnectionsPanelState: DEFAULT_ASYNC_PANEL_STATE,
    ownedReactionAssets: [],
    bookmarkedReactionAssets: [],
    bookmarkedPosts: [],
    recentReactions: [],
    profileDraft: {},
    profileDirty: false,
    profileError: null,
    profilePanelState: DEFAULT_ASYNC_PANEL_STATE,
    profileSaving: false,
    selectedAuthorPubkey: null,
    selectedAuthor: null,
    selectedAuthorTimeline: [],
    authorError: null,
    directMessagePaneOpen: false,
    selectedDirectMessagePeerPubkey: null,
    directMessages: [],
    directMessageTimelineByPeer: {},
    directMessageStatusByPeer: {},
    directMessageComposer: '',
    directMessageDraftMediaItems: [],
    directMessageAttachmentInputKey: 0,
    directMessageError: null,
    directMessageSending: false,
    composerError: null,
    liveTitle: '',
    liveDescription: '',
    liveError: null,
    livePanelStateByTopic: {
      [DEFAULT_TOPIC]: DEFAULT_ASYNC_PANEL_STATE,
    },
    liveCreatePending: false,
    livePendingBySessionId: {},
    channelLabelInput: '',
    channelAudienceInput: 'invite_only',
    inviteTokenInput: '',
    inviteOutput: null,
    inviteOutputLabel: 'invite',
    channelError: null,
    channelPanelStateByTopic: {
      [DEFAULT_TOPIC]: DEFAULT_ASYNC_PANEL_STATE,
    },
    channelActionPending: null,
    gameTitle: '',
    gameDescription: '',
    gameParticipantsInput: '',
    gameError: null,
    gameDrafts: {},
    gamePanelStateByTopic: {
      [DEFAULT_TOPIC]: DEFAULT_ASYNC_PANEL_STATE,
    },
    gameCreatePending: false,
    gameSavingByRoomId: {},
    reactionPanelState: DEFAULT_ASYNC_PANEL_STATE,
    reactionCreatePending: false,
    error: null,
    shellChromeState: {
      activePrimarySection: 'timeline',
      timelineView: 'feed',
      activeSettingsSection: 'connectivity',
      profileMode: 'overview',
      profileConnectionsView: 'following',
      navOpen: false,
      settingsOpen: false,
    },
  };
}

export function createDesktopShellStore() {
  return createStore<DesktopShellStore>((set) => ({
    ...createInitialShellState(),
    patchState: (patch) => set((current) => ({ ...current, ...patch })),
    resetState: () => set(createInitialShellState()),
    setField: (key, value) =>
      set((current) => {
        const nextValue =
          typeof value === 'function'
            ? (value as (currentValue: DesktopShellState[typeof key]) => DesktopShellState[typeof key])(
                current[key]
              )
            : value;
        if (Object.is(current[key], nextValue)) {
          return current;
        }
        return {
          [key]: nextValue,
        };
      }),
  }));
}

export type DesktopShellStoreApi = ReturnType<typeof createDesktopShellStore>;

export const DesktopShellStoreContext = createContext<DesktopShellStoreApi | null>(null);

export function useDesktopShellStoreApi() {
  const store = useContext(DesktopShellStoreContext);
  if (!store) {
    throw new Error('desktop shell store is not available');
  }

  return store;
}

export function useDesktopShellStore(): DesktopShellStore;
export function useDesktopShellStore<T>(selector: (state: DesktopShellStore) => T): T;
export function useDesktopShellStore<T>(selector?: (state: DesktopShellStore) => T) {
  const resolvedSelector =
    selector ?? ((state: DesktopShellStore) => state as unknown as T);
  return useStore(
    useDesktopShellStoreApi(),
    resolvedSelector
  );
}

export function useDesktopShellFieldSetter<K extends keyof DesktopShellState>(key: K) {
  const setField = useDesktopShellStore((state) => state.setField);

  return useCallback(
    (value: DesktopShellStateValue<K>) => {
      setField(key, value);
    },
    [key, setField]
  );
}
