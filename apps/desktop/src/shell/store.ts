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
  type SettingsSection,
  type ShellChromeState,
} from '@/components/shell/types';
import { parseHashRouteLocation } from '@/shell/routes';
import {
  type AuthorSocialView,
  type BookmarkedCustomReactionView,
  type BookmarkedPostView,
  type ChannelRef,
  type CommunityNodeConfig,
  type CommunityNodeManifest,
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
  type NotificationStatusView,
  type NotificationView,
  type PostView,
  type Profile,
  type ProfileInput,
  type RecentReactionView,
  type SyncStatus,
  type TimelineCursor,
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

export type CommunityNodeDraftNode = {
  id: string;
  base_url: string;
  auto_approve: boolean;
};

// public manifest endpoint (#356) からの取得状態。base_url ごとに保持する。
export type CommunityNodeManifestEntry =
  | { status: 'loading' }
  | { status: 'ok'; manifest: CommunityNodeManifest }
  | { status: 'absent' }
  | { status: 'error'; error: string };

export type DesktopShellState = {
  trackedTopics: string[];
  activeTopic: string;
  topicInput: string;
  composer: string;
  draftMediaItems: DraftMediaItem[];
  attachmentInputKey: number;
  timelinesByKey: Record<string, PostView[]>;
  timelineNextCursorByKey: Record<string, TimelineCursor | null>;
  timelineLoadingMoreByKey: Record<string, boolean>;
  pendingTimelineSnapshotsByKey: Record<string, PostView[]>;
  pendingTimelineCountsByKey: Record<string, number>;
  pendingTimelineNextCursorByKey: Record<string, TimelineCursor | null>;
  publicTimelinesByTopic: Record<string, PostView[]>;
  publicTimelineNextCursorByTopic: Record<string, TimelineCursor | null>;
  publicTimelineLoadingMoreByTopic: Record<string, boolean>;
  liveSessionsByTopic: Record<string, LiveSessionView[]>;
  gameRoomsByTopic: Record<string, GameRoomView[]>;
  joinedChannelsByTopic: Record<string, JoinedPrivateChannelView[]>;
  selectedChannelIdByTopic: Record<string, string | null>;
  timelineScopeByTopic: Record<string, TimelineScope>;
  composeChannelByTopic: Record<string, ChannelRef>;
  thread: PostView[];
  threadNextCursorById: Record<string, TimelineCursor | null>;
  threadLoadingMoreById: Record<string, boolean>;
  selectedThread: string | null;
  focusedObjectId: string | null;
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
  communityNodeManifests: Record<string, CommunityNodeManifestEntry>;
  communityNodeInput: CommunityNodeDraftNode[];
  communityNodeEditorDirty: boolean;
  communityNodeError: string | null;
  mediaObjectUrls: Record<string, string | null>;
  unsupportedVideoManifests: Record<string, true>;
  syncStatus: SyncStatus;
  localProfile: Profile | null;
  profileTimeline: PostView[];
  profileTimelineNextCursor: TimelineCursor | null;
  profileTimelineLoadingMore: boolean;
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
  selectedAuthorTimelineNextCursor: TimelineCursor | null;
  selectedAuthorTimelineLoadingMore: boolean;
  authorError: string | null;
  notifications: NotificationView[];
  notificationStatus: NotificationStatusView;
  notificationPanelState: AsyncPanelState;
  notificationAutoReadError: string | null;
  directMessagePaneOpen: boolean;
  selectedDirectMessagePeerPubkey: string | null;
  directMessages: DirectMessageConversationView[];
  directMessageTimelineByPeer: Record<string, DirectMessageMessageView[]>;
  directMessageTimelineNextCursorByPeer: Record<string, TimelineCursor | null>;
  directMessageTimelineLoadingMoreByPeer: Record<string, boolean>;
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
  selectedLiveSessionId: string | null;
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
  selectedGameRoomId: string | null;
  reactionPanelState: AsyncPanelState;
  reactionCreatePending: boolean;
  error: string | null;
  lastNonNotificationsRoute: string | null;
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
export const STARTER_TOPICS = [
  DEFAULT_TOPIC,
  'kukuri:topic:iroh',
  'kukuri:topic:nostr',
  'kukuri:topic:operators',
] as const;
export const PUBLIC_CHANNEL_REF: ChannelRef = { kind: 'public' };
export const PUBLIC_TIMELINE_SCOPE: TimelineScope = { kind: 'public' };
export const REFRESH_INTERVAL_MS = 3000;
export const STATUS_REFRESH_INTERVAL_MS = 10000;
export const VIDEO_POSTER_TIMEOUT_MS = 5000;
export const MEDIA_DEBUG_STORAGE_KEY = 'kukuri:media-debug';
export const SHELL_WORKSPACE_ID = 'shell-primary-workspace';
export const SHELL_NAV_ID = 'shell-nav-rail';
export const SHELL_CONTEXT_ID = 'shell-context-pane';
export const SHELL_SETTINGS_ID = 'shell-settings-drawer';

export function timelineScopeStorageKey(topic: string, scope: TimelineScope): string {
  if (scope.kind === 'channel') {
    return `${topic}::channel::${scope.channel_id}`;
  }
  return `${topic}::${scope.kind}`;
}

export function activeTimelineStorageKey(
  state: Pick<DesktopShellState, 'timelineScopeByTopic'>,
  topic: string
): string {
  return timelineScopeStorageKey(topic, state.timelineScopeByTopic[topic] ?? PUBLIC_TIMELINE_SCOPE);
}

export function timelineStorageKeyForChannel(topic: string, channelId: string | null): string {
  return timelineScopeStorageKey(
    topic,
    channelId ? { kind: 'channel', channel_id: channelId } : PUBLIC_TIMELINE_SCOPE
  );
}

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

function buildStarterTopicRecord<T>(factory: () => T): Record<string, T> {
  return Object.fromEntries(STARTER_TOPICS.map((topic) => [topic, factory()])) as Record<
    string,
    T
  >;
}

export const DEFAULT_SOCIAL_CONNECTIONS: SocialConnectionsState = {
  following: [],
  followed: [],
  muted: [],
};

const DEFAULT_SETTINGS_SECTION: SettingsSection = 'connectivity';

function parseInitialSettingsSection(): {
  activeSettingsSection: SettingsSection;
  settingsOpen: boolean;
} {
  if (typeof window === 'undefined') {
    return {
      activeSettingsSection: DEFAULT_SETTINGS_SECTION,
      settingsOpen: false,
    };
  }

  const { search } = parseHashRouteLocation(window.location.hash);
  if (!search) {
    return {
      activeSettingsSection: DEFAULT_SETTINGS_SECTION,
      settingsOpen: false,
    };
  }

  const requestedSection = new URLSearchParams(search).get('settings');
  if (
    requestedSection !== 'appearance' &&
    requestedSection !== 'connectivity' &&
      requestedSection !== 'discovery' &&
      requestedSection !== 'community-node' &&
      requestedSection !== 'reactions' &&
      requestedSection !== 'release'
  ) {
    return {
      activeSettingsSection: DEFAULT_SETTINGS_SECTION,
      settingsOpen: false,
    };
  }

  return {
    activeSettingsSection: requestedSection,
    settingsOpen: true,
  };
}

export const DEFAULT_SYNC_STATUS: SyncStatus = {
  connected: false,
  delivery_state: 'Offline',
  peer_count: 0,
  pending_events: 0,
  status_detail: '',
  last_error: null,
  configured_peers: [],
  subscribed_topics: [],
  active_path: 'direct_p2p',
  fallback_peer_ids: [],
  topic_diagnostics: [],
  local_author_pubkey: '',
  discovery: {
    mode: 'seeded_dht',
    connect_mode: 'direct_only',
    active_path: 'direct_p2p',
    fallback_peer_ids: [],
    env_locked: false,
    configured_seed_peer_ids: [],
    bootstrap_seed_peer_ids: [],
    manual_ticket_peer_ids: [],
    connected_peer_ids: [],
    docs_assist_peer_ids: [],
    blob_assist_peer_ids: [],
    local_endpoint_id: '',
    last_discovery_error: null,
  },
  gossip_disabled_topics: [],
  gossip_disabled_channels: [],
};

export const DEFAULT_NOTIFICATION_STATUS: NotificationStatusView = {
  unread_count: 0,
};

export function createInitialShellState(): DesktopShellState {
  const initialSettingsState = parseInitialSettingsSection();
  return {
    trackedTopics: [...STARTER_TOPICS],
    activeTopic: DEFAULT_TOPIC,
    topicInput: '',
    composer: '',
    draftMediaItems: [],
    attachmentInputKey: 0,
    timelinesByKey: Object.fromEntries(
      STARTER_TOPICS.map((topic) => [timelineScopeStorageKey(topic, PUBLIC_TIMELINE_SCOPE), []])
    ),
    timelineNextCursorByKey: Object.fromEntries(
      STARTER_TOPICS.map((topic) => [timelineScopeStorageKey(topic, PUBLIC_TIMELINE_SCOPE), null])
    ),
    timelineLoadingMoreByKey: Object.fromEntries(
      STARTER_TOPICS.map((topic) => [timelineScopeStorageKey(topic, PUBLIC_TIMELINE_SCOPE), false])
    ),
    pendingTimelineSnapshotsByKey: {},
    pendingTimelineCountsByKey: {},
    pendingTimelineNextCursorByKey: {},
    publicTimelinesByTopic: buildStarterTopicRecord(() => [] as PostView[]),
    publicTimelineNextCursorByTopic: buildStarterTopicRecord(() => null as TimelineCursor | null),
    publicTimelineLoadingMoreByTopic: buildStarterTopicRecord(() => false),
    liveSessionsByTopic: buildStarterTopicRecord(() => [] as LiveSessionView[]),
    gameRoomsByTopic: buildStarterTopicRecord(() => [] as GameRoomView[]),
    joinedChannelsByTopic: buildStarterTopicRecord(() => [] as JoinedPrivateChannelView[]),
    selectedChannelIdByTopic: buildStarterTopicRecord(() => null as string | null),
    timelineScopeByTopic: buildStarterTopicRecord(() => ({ ...PUBLIC_TIMELINE_SCOPE })),
    composeChannelByTopic: buildStarterTopicRecord(() => ({ ...PUBLIC_CHANNEL_REF })),
    thread: [],
    threadNextCursorById: {},
    threadLoadingMoreById: {},
    selectedThread: null,
    focusedObjectId: null,
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
    communityNodeManifests: {},
    communityNodeInput: [],
    communityNodeEditorDirty: false,
    communityNodeError: null,
    mediaObjectUrls: {},
    unsupportedVideoManifests: {},
    syncStatus: DEFAULT_SYNC_STATUS,
    localProfile: null,
    profileTimeline: [],
    profileTimelineNextCursor: null,
    profileTimelineLoadingMore: false,
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
    selectedAuthorTimelineNextCursor: null,
    selectedAuthorTimelineLoadingMore: false,
    authorError: null,
    notifications: [],
    notificationStatus: DEFAULT_NOTIFICATION_STATUS,
    notificationPanelState: DEFAULT_ASYNC_PANEL_STATE,
    notificationAutoReadError: null,
    directMessagePaneOpen: false,
    selectedDirectMessagePeerPubkey: null,
    directMessages: [],
    directMessageTimelineByPeer: {},
    directMessageTimelineNextCursorByPeer: {},
    directMessageTimelineLoadingMoreByPeer: {},
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
    livePanelStateByTopic: buildStarterTopicRecord(() => ({ ...DEFAULT_ASYNC_PANEL_STATE })),
    liveCreatePending: false,
    livePendingBySessionId: {},
    selectedLiveSessionId: null,
    channelLabelInput: '',
    channelAudienceInput: 'invite_only',
    inviteTokenInput: '',
    inviteOutput: null,
    inviteOutputLabel: 'invite',
    channelError: null,
    channelPanelStateByTopic: buildStarterTopicRecord(() => ({ ...DEFAULT_ASYNC_PANEL_STATE })),
    channelActionPending: null,
    gameTitle: '',
    gameDescription: '',
    gameParticipantsInput: '',
    gameError: null,
    gameDrafts: {},
    gamePanelStateByTopic: buildStarterTopicRecord(() => ({ ...DEFAULT_ASYNC_PANEL_STATE })),
    gameCreatePending: false,
    gameSavingByRoomId: {},
    selectedGameRoomId: null,
    reactionPanelState: DEFAULT_ASYNC_PANEL_STATE,
    reactionCreatePending: false,
    error: null,
    lastNonNotificationsRoute: null,
    shellChromeState: {
      activePrimarySection: 'timeline',
      timelineView: 'feed',
      activeSettingsSection: initialSettingsState.activeSettingsSection,
      profileMode: 'overview',
      profileConnectionsView: 'following',
      navOpen: false,
      settingsOpen: initialSettingsState.settingsOpen,
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
