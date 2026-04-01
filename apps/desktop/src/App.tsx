import {
  ChangeEvent,
  FormEvent,
  SyntheticEvent,
  createContext,
  startTransition,
  useCallback,
  useEffect,
  useContext,
  useMemo,
  useRef,
  useState,
} from 'react';
import {
  HashRouter,
  useLocation,
  useNavigate,
} from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useStore } from 'zustand';
import { createStore } from 'zustand/vanilla';
import { Lock, PanelLeftOpen, Plus, Settings } from 'lucide-react';

import { AuthorDetailCard } from '@/components/core/AuthorDetailCard';
import { ComposerDraftPreviewList } from '@/components/core/ComposerDraftPreviewList';
import { ComposerPanel } from '@/components/core/ComposerPanel';
import { ThreadPanel } from '@/components/core/ThreadPanel';
import { TimelineFeed } from '@/components/core/TimelineFeed';
import { TimelineWorkspaceHeader } from '@/components/core/TimelineWorkspaceHeader';
import { TopicNavList } from '@/components/core/TopicNavList';
import {
  type AuthorDetailView,
  type ComposerDraftMediaView,
  type PostCardView,
  type ThreadPanelState,
  type TopicDiagnosticSummary,
} from '@/components/core/types';
import { ProfileOverviewPanel } from '@/components/extended/ProfileOverviewPanel';
import { ProfileEditorPanel } from '@/components/extended/ProfileEditorPanel';
import { ProfileConnectionsPanel } from '@/components/extended/ProfileConnectionsPanel';
import { PrivateChannelPanel } from '@/components/extended/PrivateChannelPanel';
import { AppearancePanel } from '@/components/settings/AppearancePanel';
import { CommunityNodePanel } from '@/components/settings/CommunityNodePanel';
import { ConnectivityPanel } from '@/components/settings/ConnectivityPanel';
import { DiscoveryPanel } from '@/components/settings/DiscoveryPanel';
import { ReactionsPanel } from '@/components/settings/ReactionsPanel';
import {
  type AppearancePanelView,
  type CommunityNodePanelView,
  type ConnectivityPanelView,
  type DiscoveryPanelView,
  type ReactionsPanelView,
} from '@/components/settings/types';
import {
  type ChannelAudienceOption,
  type ExtendedPanelStatus,
  type GameDraftView,
  type InviteOutputLabel,
  type PrivateChannelListItemView,
  type PrivateChannelPendingAction,
} from '@/components/extended/types';
import { ContextPane } from '@/components/shell/ContextPane';
import { ShellFrame } from '@/components/shell/ShellFrame';
import { ShellNavRail } from '@/components/shell/ShellNavRail';
import { SettingsDrawer } from '@/components/shell/SettingsDrawer';
import { ShellTopBar } from '@/components/shell/ShellTopBar';
import {
  type PrimarySection,
  type ProfileConnectionsView,
  type ProfileWorkspaceMode,
  type SettingsSection,
  type ShellChromeState,
  type TimelineWorkspaceView,
} from '@/components/shell/types';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { Select } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';

import {
  ChannelAccessTokenPreview,
  AuthorSocialView,
  AttachmentView,
  BlobMediaPayload,
  BookmarkedCustomReactionView,
  BookmarkedPostView,
  ChannelAudienceKind,
  ChannelRef,
  CustomReactionAssetView,
  CustomReactionCropRect,
  CommunityNodeConfig,
  CommunityNodeNodeStatus,
  CreateAttachmentInput,
  DesktopApi,
  DirectMessageConversationView,
  DirectMessageMessageView,
  DirectMessageStatusView,
  DiscoveryConfig,
  GameRoomStatus,
  GameRoomView,
  GameScoreView,
  JoinedPrivateChannelView,
  LiveSessionView,
  PostView,
  Profile,
  ProfileInput,
  ReactionKeyInput,
  ReactionStateView,
  RecentReactionView,
  SyncStatus,
  TimelineScope,
  TopicSyncStatus,
  runtimeApi,
} from './lib/api';
import { blobToCreateAttachment, fileToCreateAttachment } from './lib/attachments';
import { readDesktopTheme, type DesktopTheme, writeDesktopTheme } from './lib/theme';
import i18n, { type SupportedLocale } from './i18n';
import {
  formatLocalizedBytes,
  formatLocalizedNumber,
  formatLocalizedTime,
  getResolvedLocale,
} from './i18n/format';

type AppProps = {
  api?: DesktopApi;
};

type DraftMediaItem = {
  id: string;
  source_name: string;
  preview_url: string;
  attachments: CreateAttachmentInput[];
};

type GameEditorDraft = {
  status: GameRoomStatus;
  phase_label: string;
  scores: Record<string, string>;
};

type AsyncPanelState = {
  status: ExtendedPanelStatus;
  error: string | null;
};

type SocialConnectionsState = Record<ProfileConnectionsView, AuthorSocialView[]>;
type KnownAuthorsByPubkey = Record<string, AuthorSocialView>;

type DesktopShellState = {
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
  channelAudienceInput: ChannelAudienceKind;
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

type DesktopShellStateValue<K extends keyof DesktopShellState> =
  | DesktopShellState[K]
  | ((current: DesktopShellState[K]) => DesktopShellState[K]);

type DesktopShellStore = DesktopShellState & {
  patchState: (patch: Partial<DesktopShellState>) => void;
  resetState: () => void;
  setField: <K extends keyof DesktopShellState>(
    key: K,
    value: DesktopShellStateValue<K>
  ) => void;
};

type DesktopShellRouteOverrides = {
  activeTopic?: string;
  composeTarget?: ChannelRef;
  primarySection?: PrimarySection;
  profileMode?: ProfileWorkspaceMode;
  profileConnectionsView?: ProfileConnectionsView;
  selectedAuthorPubkey?: string | null;
  directMessagePaneOpen?: boolean;
  selectedDirectMessagePeerPubkey?: string | null;
  selectedThread?: string | null;
  settingsOpen?: boolean;
  settingsSection?: SettingsSection;
  timelineScope?: TimelineScope;
  timelineView?: TimelineWorkspaceView;
};

type OpenThreadOptions = {
  historyMode?: 'push' | 'replace';
  normalizeOnEmpty?: boolean;
  topic?: string;
};

type DesktopShellPageProps = AppProps & {
  theme: DesktopTheme;
  onThemeChange: (theme: DesktopTheme) => void;
};

type OpenAuthorOptions = {
  fromThread?: boolean;
  historyMode?: 'push' | 'replace';
  normalizeOnError?: boolean;
  threadId?: string | null;
};

type DesktopShellStoreApi = ReturnType<typeof createDesktopShellStore>;

type MediaDebugValue = boolean | number | string | null | undefined;
type MediaDebugFields = Record<string, MediaDebugValue>;

const DEFAULT_TOPIC = 'kukuri:topic:demo';
const PUBLIC_CHANNEL_REF: ChannelRef = { kind: 'public' };
const PUBLIC_TIMELINE_SCOPE: TimelineScope = { kind: 'public' };
const REFRESH_INTERVAL_MS = 2000;
const VIDEO_POSTER_TIMEOUT_MS = 5000;
const MEDIA_DEBUG_STORAGE_KEY = 'kukuri:media-debug';
const SHELL_WORKSPACE_ID = 'shell-primary-workspace';
const SHELL_NAV_ID = 'shell-nav-rail';
const SHELL_CONTEXT_ID = 'shell-context-pane';
const SHELL_SETTINGS_ID = 'shell-settings-drawer';
const DEFAULT_ASYNC_PANEL_STATE: AsyncPanelState = {
  status: 'loading',
  error: null,
};
const DEFAULT_DISCOVERY_CONFIG: DiscoveryConfig = {
  mode: 'seeded_dht',
  connect_mode: 'direct_only',
  env_locked: false,
  seed_peers: [],
};
const DEFAULT_COMMUNITY_NODE_CONFIG: CommunityNodeConfig = {
  nodes: [],
};
const DEFAULT_SOCIAL_CONNECTIONS: SocialConnectionsState = {
  following: [],
  followed: [],
  muted: [],
};
const DEFAULT_SYNC_STATUS: SyncStatus = {
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

function createInitialShellState(): DesktopShellState {
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

function createDesktopShellStore() {
  return createStore<DesktopShellStore>((set) => ({
    ...createInitialShellState(),
    patchState: (patch) => set((current) => ({ ...current, ...patch })),
    resetState: () => set(createInitialShellState()),
    setField: (key, value) =>
      set((current) => ({
        [key]:
          typeof value === 'function'
            ? (value as (currentValue: DesktopShellState[typeof key]) => DesktopShellState[typeof key])(
                current[key]
              )
            : value,
      })),
  }));
}

const DesktopShellStoreContext = createContext<DesktopShellStoreApi | null>(null);

function useDesktopShellStoreApi() {
  const store = useContext(DesktopShellStoreContext);
  if (!store) {
    throw new Error('desktop shell store is not available');
  }

  return store;
}

function useDesktopShellStore() {
  return useStore(useDesktopShellStoreApi());
}

const PRIMARY_SECTION_ITEMS: Array<{
  id: PrimarySection;
  label: string;
}> = [
  {
    id: 'timeline',
    label: 'Timeline',
  },
  {
    id: 'live',
    label: 'Live',
  },
  {
    id: 'game',
    label: 'Game',
  },
  {
    id: 'messages',
    label: 'Messages',
  },
  {
    id: 'profile',
    label: 'Profile',
  },
];

const SETTINGS_SECTION_COPY: Array<{
  id: SettingsSection;
  label: string;
  description: string;
}> = [
  {
    id: 'appearance',
    label: 'Appearance',
    description: 'Local light and dark theme selection.',
  },
  {
    id: 'connectivity',
    label: 'Connectivity',
    description: 'Sync summary, peer tickets, and global error visibility.',
  },
  {
    id: 'discovery',
    label: 'Discovery',
    description: 'Seeded DHT configuration and discovery diagnostics.',
  },
  {
    id: 'community-node',
    label: 'Community Node',
    description: 'Configured community nodes, auth, consent, and refresh actions.',
  },
  {
    id: 'reactions',
    label: 'Reactions',
    description: 'Custom reaction creation and saved reaction management.',
  },
];

function isSettingsSection(value: string | null): value is SettingsSection {
  return (
    value === 'appearance' ||
    value === 'connectivity' ||
    value === 'discovery' ||
    value === 'community-node' ||
    value === 'reactions'
  );
}

function isProfileConnectionsView(value: string | null): value is ProfileConnectionsView {
  return value === 'following' || value === 'followed' || value === 'muted';
}

const PRIMARY_SECTION_PATHS: Record<PrimarySection, string> = {
  timeline: '/timeline',
  live: '/live',
  game: '/game',
  messages: '/messages',
  profile: '/profile',
};

function parsePrimarySectionPath(pathname: string): PrimarySection | null {
  const normalizedPath = pathname === '/' ? '/timeline' : pathname;
  if (normalizedPath === '/channels') {
    return null;
  }
  const match = (
    Object.entries(PRIMARY_SECTION_PATHS) as Array<[PrimarySection, string]>
  ).find(([, path]) => path === normalizedPath);
  return match?.[0] ?? null;
}

function translate(key: string, options?: Record<string, unknown>): string {
  return i18n.t(key, options) as string;
}

function selectPrimaryImage(post: PostView): AttachmentView | null {
  return selectPrimaryImageAttachment(post.attachments);
}

function selectVideoPoster(post: PostView): AttachmentView | null {
  return selectVideoPosterAttachment(post.attachments);
}

function selectVideoManifest(post: PostView): AttachmentView | null {
  return selectVideoManifestAttachment(post.attachments);
}

function selectPrimaryImageAttachment(attachments: AttachmentView[]): AttachmentView | null {
  return attachments.find((attachment) => attachment.role === 'image_original') ?? null;
}

function selectVideoPosterAttachment(attachments: AttachmentView[]): AttachmentView | null {
  return attachments.find((attachment) => attachment.role === 'video_poster') ?? null;
}

function selectVideoManifestAttachment(attachments: AttachmentView[]): AttachmentView | null {
  return (
    attachments.find(
      (attachment) =>
        attachment.role === 'video_manifest' || attachment.mime.startsWith('video/')
    ) ?? null
  );
}

function formatBytes(bytes: number, locale?: string | null): string {
  return formatLocalizedBytes(bytes, locale);
}

function shortPubkey(pubkey: string): string {
  return pubkey.slice(0, 12);
}

function isHex64(value: string): boolean {
  return value.length === 64 && [...value].every((character) => character.match(/[0-9a-f]/i));
}

function messageFromError(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback;
}

function profileInputFromProfile(profile: Profile): ProfileInput {
  return {
    name: profile.name ?? '',
    display_name: profile.display_name ?? '',
    about: profile.about ?? '',
    picture: profile.picture ?? '',
    picture_upload: null,
    clear_picture: false,
  };
}

function resolveProfilePictureSrc(
  profile:
    | Pick<Profile, 'picture' | 'picture_asset'>
    | Pick<AuthorSocialView, 'picture' | 'picture_asset'>
    | null
    | undefined,
  mediaObjectUrls: Record<string, string | null>
): string | null {
  const pictureAssetHash = profile?.picture_asset?.hash;
  if (pictureAssetHash && typeof mediaObjectUrls[pictureAssetHash] === 'string') {
    return mediaObjectUrls[pictureAssetHash];
  }
  return profile?.picture ?? null;
}

function authorDisplayLabel(
  authorPubkey: string,
  displayName?: string | null,
  name?: string | null
): string {
  return displayName?.trim() || name?.trim() || shortPubkey(authorPubkey);
}

function publishedTopicIdForPost(post: Pick<PostView, 'published_topic_id' | 'origin_topic_id'>): string | null {
  return post.published_topic_id?.trim() || post.origin_topic_id?.trim() || null;
}

function patchReactionStateIntoPosts(posts: PostView[], reactionState: ReactionStateView): PostView[] {
  return posts.map((post) =>
    post.object_id === reactionState.target_object_id
      ? {
          ...post,
          reaction_summary: reactionState.reaction_summary,
          my_reactions: reactionState.my_reactions,
        }
      : post
  );
}

function canCreateRepostFromPost(post: PostView): boolean {
  return (post.object_kind === 'post' || post.object_kind === 'comment') && !post.channel_id;
}

function isQuoteRepost(post: Pick<PostView, 'object_kind' | 'repost_commentary'>): boolean {
  return post.object_kind === 'repost' && Boolean(post.repost_commentary?.trim());
}

function formatListLabel(values: string[]): string {
  return values.length > 0 ? values.join(', ') : translate('common:fallbacks.none');
}

function formatLastReceivedLabel(timestamp?: number | null, locale?: string | null): string {
  return timestamp
    ? formatLocalizedTime(timestamp, locale)
    : translate('common:fallbacks.noEvents');
}

function strongestRelationshipLabel(relationship: {
  mutual: boolean;
  following: boolean;
  followed_by: boolean;
  friend_of_friend: boolean;
}): string | null {
  if (relationship.mutual) {
    return 'mutual';
  }
  if (relationship.following) {
    return 'following';
  }
  if (relationship.followed_by) {
    return 'follows you';
  }
  if (relationship.friend_of_friend) {
    return 'friend of friend';
  }
  return null;
}

function mergeAuthorView(
  current: AuthorSocialView | null | undefined,
  incoming: Partial<AuthorSocialView> & { author_pubkey: string }
): AuthorSocialView {
  return {
    author_pubkey: incoming.author_pubkey,
    name: incoming.name ?? current?.name ?? null,
    display_name: incoming.display_name ?? current?.display_name ?? null,
    about: incoming.about ?? current?.about ?? null,
    picture: incoming.picture ?? current?.picture ?? null,
    picture_asset: incoming.picture_asset ?? current?.picture_asset ?? null,
    updated_at: incoming.updated_at ?? current?.updated_at ?? null,
    following: incoming.following ?? current?.following ?? false,
    followed_by: incoming.followed_by ?? current?.followed_by ?? false,
    mutual: incoming.mutual ?? current?.mutual ?? false,
    friend_of_friend: incoming.friend_of_friend ?? current?.friend_of_friend ?? false,
    friend_of_friend_via_pubkeys:
      incoming.friend_of_friend_via_pubkeys ?? current?.friend_of_friend_via_pubkeys ?? [],
    muted: incoming.muted ?? current?.muted ?? false,
  };
}

function mergeKnownAuthors(
  current: KnownAuthorsByPubkey,
  incoming: Array<(Partial<AuthorSocialView> & { author_pubkey: string }) | null | undefined>
): KnownAuthorsByPubkey {
  let next = current;
  for (const view of incoming) {
    if (!view) {
      continue;
    }
    const merged = mergeAuthorView(next[view.author_pubkey], view);
    if (next === current) {
      next = { ...current };
    }
    next[view.author_pubkey] = merged;
  }
  return next;
}

function authorViewFromDirectMessageConversation(
  conversation: DirectMessageConversationView
): AuthorSocialView {
  return {
    author_pubkey: conversation.peer_pubkey,
    name: conversation.peer_name ?? null,
    display_name: conversation.peer_display_name ?? null,
    about: null,
    picture: conversation.peer_picture ?? null,
    picture_asset: conversation.peer_picture_asset ?? null,
    updated_at: null,
    following: false,
    followed_by: false,
    mutual: conversation.status.mutual,
    friend_of_friend: false,
    friend_of_friend_via_pubkeys: [],
    muted: false,
  };
}

function privateTimelineScope(channelId: string | null): TimelineScope {
  return channelId
    ? {
        kind: 'channel',
        channel_id: channelId,
      }
    : PUBLIC_TIMELINE_SCOPE;
}

function privateComposeTarget(channelId: string | null): ChannelRef {
  return channelId
    ? {
        kind: 'private_channel',
        channel_id: channelId,
      }
    : PUBLIC_CHANNEL_REF;
}

function audienceLabelForChannelRef(
  channelRef: ChannelRef,
  joinedChannels: JoinedPrivateChannelView[]
): string {
  if (channelRef.kind === 'public') {
    return translate('common:audience.public');
  }
  return (
    joinedChannels.find((channel) => channel.channel_id === channelRef.channel_id)?.label ??
    translate('common:audience.privateChannel')
  );
}

function audienceLabelForTimelineScope(
  scope: TimelineScope,
  joinedChannels: JoinedPrivateChannelView[]
): string {
  if (scope.kind === 'all_joined') {
    return translate('common:audience.allJoined');
  }
  if (scope.kind === 'channel') {
    return (
      joinedChannels.find((channel) => channel.channel_id === scope.channel_id)?.label ??
      translate('common:audience.privateChannel')
    );
  }
  return translate('common:audience.public');
}

function formatSeedPeer(peer: DiscoveryConfig['seed_peers'][number]): string {
  return peer.addr_hint ? `${peer.endpoint_id}@${peer.addr_hint}` : peer.endpoint_id;
}

function seedPeersToEditorValue(config: DiscoveryConfig): string {
  return config.seed_peers.map((peer) => formatSeedPeer(peer)).join('\n');
}

function communityNodesToEditorValue(config: CommunityNodeConfig): string {
  return config.nodes.map((node) => node.base_url).join('\n');
}

function syncStatusBadgeTone(syncStatus: SyncStatus): 'accent' | 'destructive' | 'warning' {
  if (syncStatus.last_error) {
    return 'destructive';
  }
  return syncStatus.connected ? 'accent' : 'warning';
}

function syncStatusBadgeLabel(syncStatus: SyncStatus): string {
  if (syncStatus.last_error) {
    return translate('common:states.error');
  }
  return syncStatus.connected
    ? translate('common:states.connected')
    : translate('common:states.waiting');
}

function topicConnectionLabel(diagnostic?: TopicSyncStatus): string {
  if (!diagnostic) {
    return 'idle';
  }
  if (diagnostic.connected_peers.length > 0) {
    return 'joined';
  }
  if (diagnostic.assist_peer_ids.length > 0) {
    return 'relay-assisted';
  }
  return diagnostic.joined ? 'joined' : 'idle';
}

function communityNodeConnectivityUrlsLabel(status?: CommunityNodeNodeStatus): string {
  if (status?.resolved_urls?.connectivity_urls?.length) {
    return status.resolved_urls.connectivity_urls.join(', ');
  }
  if (status?.consent_state && !status.consent_state.all_required_accepted) {
    return translate('settings:communityNode.values.pendingConsentAcceptance');
  }
  if (status?.auth_state.authenticated) {
    return translate('settings:communityNode.values.notResolvedYet');
  }
  return translate('settings:communityNode.values.notResolved');
}

function communityNodeNextStepLabel(status?: CommunityNodeNodeStatus): string {
  if (!status) {
    return translate('settings:communityNode.values.saveNodesToBegin');
  }
  if (!status.auth_state.authenticated) {
    return translate('settings:communityNode.values.authenticateThisNode');
  }
  if (status.consent_state && !status.consent_state.all_required_accepted) {
    return translate('settings:communityNode.values.acceptPolicies');
  }
  if (status.restart_required) {
    return translate('settings:communityNode.values.restartUnexpected');
  }
  if (!status.resolved_urls) {
    return translate('settings:communityNode.values.refreshMetadata');
  }
  return translate('settings:communityNode.values.connectivityUrlsActiveOnCurrentSession');
}

function communityNodeSessionActivationLabel(status?: CommunityNodeNodeStatus): string {
  if (!status) {
    return translate('common:states.unknown');
  }
  if (status.restart_required) {
    return translate('settings:communityNode.values.restartRequiredUnexpected');
  }
  if (status.resolved_urls?.connectivity_urls?.length) {
    return translate('settings:communityNode.values.activeOnCurrentSession');
  }
  if (status.consent_state && !status.consent_state.all_required_accepted) {
    return translate('settings:communityNode.values.waitingForConsent');
  }
  if (status.auth_state.authenticated) {
    return translate('settings:communityNode.values.awaitingConnectivityMetadata');
  }
  return translate('settings:communityNode.values.notAuthenticated');
}

function communityNodeAuthLabel(status?: CommunityNodeNodeStatus): string {
  return status?.auth_state.authenticated
    ? `${translate('common:states.yes')} (${status.auth_state.expires_at ?? translate('common:states.unknown')})`
    : translate('common:states.no');
}

function communityNodeConsentLabel(status?: CommunityNodeNodeStatus): string {
  if (!status?.consent_state) {
    return translate('common:states.unknown');
  }
  return status.consent_state.all_required_accepted
    ? translate('common:states.accepted')
    : translate('common:states.required');
}

function translateTopicConnectionText(label: string): string {
  if (label === 'joined') {
    return translate('common:states.joined');
  }
  if (label === 'relay-assisted') {
    return translate('common:states.relayAssisted');
  }
  if (label === 'idle') {
    return translate('common:states.idle');
  }
  return label;
}

function translateAudienceKindLabel(kind: ChannelAudienceKind): string {
  return translate(`channels:audienceOptions.${kind}`);
}

function translateLiveStatus(status: LiveSessionView['status']): string {
  return translate(`live:statuses.${status}`);
}

function translateGameStatus(status: GameRoomStatus): string {
  return translate(`game:statuses.${status}`);
}

function formatCount(value: number): string {
  return formatLocalizedNumber(value);
}

function localizeAudienceLabel(label: string): string {
  if (label === 'Public') {
    return translate('common:audience.public');
  }
  if (label === 'All joined') {
    return translate('common:audience.allJoined');
  }
  if (label === 'Private channel') {
    return translate('common:audience.privateChannel');
  }
  return label;
}

function mergeCommunityNodeStatus(
  previous: CommunityNodeNodeStatus | undefined,
  next: CommunityNodeNodeStatus
): CommunityNodeNodeStatus {
  return {
    ...next,
    consent_state: next.auth_state.authenticated
      ? next.consent_state ?? previous?.consent_state ?? null
      : next.consent_state ?? null,
    resolved_urls: next.resolved_urls ?? previous?.resolved_urls ?? null,
    last_error: next.last_error ?? previous?.last_error ?? null,
  };
}

function mergeCommunityNodeStatuses(
  previous: CommunityNodeNodeStatus[],
  next: CommunityNodeNodeStatus[]
): CommunityNodeNodeStatus[] {
  const previousByBaseUrl = Object.fromEntries(
    previous.map((status) => [status.base_url, status])
  ) as Record<string, CommunityNodeNodeStatus>;
  return next.map((status) => mergeCommunityNodeStatus(previousByBaseUrl[status.base_url], status));
}

function upsertCommunityNodeStatus(
  current: CommunityNodeNodeStatus[],
  next: CommunityNodeNodeStatus
): CommunityNodeNodeStatus[] {
  const previous = current.find((status) => status.base_url === next.base_url);
  const merged = mergeCommunityNodeStatus(previous, next);
  const remaining = current.filter((status) => status.base_url !== next.base_url);
  return [...remaining, merged].sort((left, right) => left.base_url.localeCompare(right.base_url));
}

function syncCommunityNodeConfigWithStatus(
  current: CommunityNodeConfig,
  status: CommunityNodeNodeStatus
): CommunityNodeConfig {
  return {
    nodes: current.nodes.map((node) =>
      node.base_url === status.base_url
        ? {
            ...node,
            resolved_urls: status.resolved_urls ?? node.resolved_urls ?? null,
          }
        : node
    ),
  };
}

function base64ToBytes(base64: string): Uint8Array {
  const binary = window.atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

function createObjectUrlFromPayload(payload: BlobMediaPayload): string {
  const bytes = base64ToBytes(payload.bytes_base64);
  const normalizedBytes = new Uint8Array(bytes.length);
  normalizedBytes.set(bytes);
  return URL.createObjectURL(new Blob([normalizedBytes], { type: payload.mime }));
}

function isMediaDebugEnabled(): boolean {
  if (import.meta.env.MODE === 'test') {
    return false;
  }

  if (import.meta.env.DEV) {
    return true;
  }

  try {
    return window.localStorage.getItem(MEDIA_DEBUG_STORAGE_KEY) === '1';
  } catch {
    return false;
  }
}

function logMediaDebug(level: 'info' | 'warn', event: string, fields: MediaDebugFields): void {
  if (!isMediaDebugEnabled()) {
    return;
  }

  const logger = level === 'warn' ? console.warn : console.info;
  logger(`[kukuri.media] ${event}`, fields);
}

function mediaElementDebugFields(media: HTMLMediaElement): MediaDebugFields {
  return {
    current_src: media.currentSrc || media.getAttribute('src') || null,
    current_time: Number.isFinite(media.currentTime) ? media.currentTime : null,
    duration: Number.isFinite(media.duration) ? media.duration : null,
    ended: media.ended,
    error_code: media.error?.code ?? null,
    network_state: media.networkState,
    paused: media.paused,
    ready_state: media.readyState,
  };
}

function attachVideoDebugListeners(
  video: HTMLVideoElement,
  phase: string,
  fields: MediaDebugFields
): () => void {
  const eventNames = [
    'loadstart',
    'loadedmetadata',
    'loadeddata',
    'canplay',
    'durationchange',
    'seeked',
    'playing',
    'error',
  ] as const;
  const removeListeners = eventNames.map((eventName) => {
    const handler = () => {
      logMediaDebug(eventName === 'error' ? 'warn' : 'info', `${phase} ${eventName}`, {
        ...fields,
        ...mediaElementDebugFields(video),
        video_height: video.videoHeight || null,
        video_width: video.videoWidth || null,
      });
    };
    video.addEventListener(eventName, handler);
    return () => {
      video.removeEventListener(eventName, handler);
    };
  });

  return () => {
    for (const removeListener of removeListeners) {
      removeListener();
    }
  };
}

function posterFileName(fileName: string): string {
  const extensionIndex = fileName.lastIndexOf('.');
  const baseName = extensionIndex >= 0 ? fileName.slice(0, extensionIndex) : fileName;
  return `${baseName}.poster.jpg`;
}

function attachHiddenVideo(video: HTMLVideoElement) {
  video.setAttribute('aria-hidden', 'true');
  video.style.position = 'fixed';
  video.style.left = '-9999px';
  video.style.top = '0';
  video.style.width = '1px';
  video.style.height = '1px';
  video.style.opacity = '0';
  video.style.pointerEvents = 'none';
  document.body.appendChild(video);
}

async function waitForPosterFrame(video: HTMLVideoElement): Promise<void> {
  return await new Promise<void>((resolve, reject) => {
    let settled = false;

    const cleanup = () => {
      video.removeEventListener('loadeddata', resolveIfReady);
      video.removeEventListener('canplay', resolveIfReady);
      video.removeEventListener('seeked', resolveIfReady);
      video.removeEventListener('timeupdate', resolveIfReady);
      video.removeEventListener('loadedmetadata', handleMetadata);
      video.removeEventListener('error', fail);
    };

    const finish = () => {
      if (settled) {
        return;
      }
      settled = true;
      cleanup();
      resolve();
    };

    const fail = () => {
      if (settled) {
        return;
      }
      settled = true;
      cleanup();
      reject(new Error(translate('common:errors.failedToGenerateVideoPoster')));
    };

    const resolveIfReady = () => {
      if (
        video.videoWidth > 0 &&
        video.videoHeight > 0 &&
        video.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA
      ) {
        finish();
      }
    };

    const handleMetadata = () => {
      resolveIfReady();
      if (settled) {
        return;
      }

      const duration = Number.isFinite(video.duration) ? video.duration : 0;
      const seekTarget = duration > 0 ? Math.min(duration / 2, 0.1) : 0.1;
      if (seekTarget > 0) {
        try {
          video.currentTime = seekTarget;
        } catch {
          // Some platforms reject seek before decode warms up.
        }
      }

      try {
        const playAttempt = video.play();
        if (playAttempt && typeof playAttempt.then === 'function') {
          void playAttempt.then(() => {
            video.pause();
            resolveIfReady();
          });
        }
      } catch {
        // ignore
      }
    };

    video.addEventListener('loadeddata', resolveIfReady);
    video.addEventListener('canplay', resolveIfReady);
    video.addEventListener('seeked', resolveIfReady);
    video.addEventListener('timeupdate', resolveIfReady);
    video.addEventListener('loadedmetadata', handleMetadata);
    video.addEventListener('error', fail, { once: true });
    resolveIfReady();
  });
}

async function generateVideoPoster(file: File): Promise<File> {
  const videoObjectUrl = URL.createObjectURL(file);
  logMediaDebug('info', 'poster generation start', {
    file_name: file.name,
    mime: file.type || null,
    size: file.size,
    video_object_url: videoObjectUrl,
  });

  try {
    return await new Promise<File>((resolve, reject) => {
      const video = document.createElement('video');
      const canvas = document.createElement('canvas');
      let finished = false;
      const removeDebugListeners = attachVideoDebugListeners(video, 'poster', {
        file_name: file.name,
        mime: file.type || null,
        size: file.size,
      });

      const fail = () => {
        if (finished) {
          return;
        }
        finished = true;
        logMediaDebug('warn', 'poster generation failed', {
          file_name: file.name,
          mime: file.type || null,
          size: file.size,
          ...mediaElementDebugFields(video),
          video_height: video.videoHeight || null,
          video_width: video.videoWidth || null,
        });
        reject(new Error(translate('common:errors.failedToGenerateVideoPoster')));
      };

      const timeoutId = window.setTimeout(fail, VIDEO_POSTER_TIMEOUT_MS);

      const cleanup = () => {
        window.clearTimeout(timeoutId);
        removeDebugListeners();
        try {
          video.pause();
        } catch {
          // ignore
        }
        video.removeAttribute('src');
        try {
          video.load();
        } catch {
          // ignore
        }
        video.remove();
      };

      video.preload = 'metadata';
      video.muted = true;
      video.playsInline = true;
      attachHiddenVideo(video);

      video.src = videoObjectUrl;
      video.load();

      void waitForPosterFrame(video)
        .then(() => {
          if (finished) {
            return;
          }

          const width = video.videoWidth;
          const height = video.videoHeight;
          if (!width || !height) {
            cleanup();
            fail();
            return;
          }

          logMediaDebug('info', 'poster frame ready', {
            file_name: file.name,
            height,
            mime: file.type || null,
            size: file.size,
            width,
            ...mediaElementDebugFields(video),
          });

          canvas.width = width;
          canvas.height = height;
          const context = canvas.getContext('2d');
          if (!context) {
            cleanup();
            fail();
            return;
          }

          context.drawImage(video, 0, 0, width, height);
          canvas.toBlob(
            (blob) => {
              if (finished) {
                return;
              }
              cleanup();
              if (!blob) {
                fail();
                return;
              }
              finished = true;
              logMediaDebug('info', 'poster generation complete', {
                blob_size: blob.size,
                file_name: file.name,
                mime: file.type || null,
                poster_file_name: posterFileName(file.name),
                size: file.size,
              });
              resolve(
                new File([blob], posterFileName(file.name), {
                  type: 'image/jpeg',
                })
              );
            },
            'image/jpeg',
            0.85
          );
        })
        .catch((error: unknown) => {
          logMediaDebug('warn', 'poster generation exception', {
            error: error instanceof Error ? error.message : 'unknown error',
            file_name: file.name,
            mime: file.type || null,
            size: file.size,
          });
          cleanup();
          fail();
        });
    });
  } finally {
    URL.revokeObjectURL(videoObjectUrl);
  }
}

function createGameEditorDraft(room: GameRoomView): GameEditorDraft {
  return {
    status: room.status,
    phase_label: room.phase_label ?? '',
    scores: Object.fromEntries(room.scores.map((score) => [score.participant_id, String(score.score)])),
  };
}

function upsertJoinedChannel(
  channels: JoinedPrivateChannelView[],
  nextChannel: JoinedPrivateChannelView
): JoinedPrivateChannelView[] {
  const remaining = channels.filter((channel) => channel.channel_id !== nextChannel.channel_id);
  return [...remaining, nextChannel];
}

function joinedChannelFromAccessTokenPreview(
  preview: ChannelAccessTokenPreview
): JoinedPrivateChannelView {
  if (preview.kind === 'grant') {
    return {
      topic_id: preview.topic_id,
      channel_id: preview.channel_id,
      label: preview.channel_label,
      creator_pubkey: preview.owner_pubkey,
      owner_pubkey: preview.owner_pubkey,
      joined_via_pubkey: preview.sponsor_pubkey ?? null,
      audience_kind: 'friend_only',
      is_owner: false,
      current_epoch_id: preview.epoch_id,
      archived_epoch_ids: [],
      sharing_state: 'open',
      rotation_required: false,
      participant_count: 1,
      stale_participant_count: 0,
    };
  }
  if (preview.kind === 'share') {
    return {
      topic_id: preview.topic_id,
      channel_id: preview.channel_id,
      label: preview.channel_label,
      creator_pubkey: preview.owner_pubkey,
      owner_pubkey: preview.owner_pubkey,
      joined_via_pubkey: preview.sponsor_pubkey ?? null,
      audience_kind: 'friend_plus',
      is_owner: false,
      current_epoch_id: preview.epoch_id,
      archived_epoch_ids: [],
      sharing_state: 'open',
      rotation_required: false,
      participant_count: 2,
      stale_participant_count: 0,
    };
  }
  return {
    topic_id: preview.topic_id,
    channel_id: preview.channel_id,
    label: preview.channel_label,
    creator_pubkey: preview.inviter_pubkey ?? preview.owner_pubkey,
    owner_pubkey: preview.owner_pubkey,
    joined_via_pubkey: preview.inviter_pubkey ?? null,
    audience_kind: 'invite_only',
    is_owner: false,
    current_epoch_id: preview.epoch_id,
    archived_epoch_ids: [],
    sharing_state: 'open',
    rotation_required: false,
    participant_count: 1,
    stale_participant_count: 0,
  };
}

function DesktopShellPage({
  api = runtimeApi,
  theme,
  onThemeChange,
}: DesktopShellPageProps) {
  const { t, i18n: i18nInstance } = useTranslation([
    'common',
    'shell',
    'settings',
    'profile',
    'channels',
    'live',
    'game',
  ]);
  const locale = getResolvedLocale(i18nInstance.resolvedLanguage);
  const storeApi = useDesktopShellStoreApi();
  const {
    trackedTopics,
    activeTopic,
    topicInput,
    composer,
    draftMediaItems,
    attachmentInputKey,
    timelinesByTopic,
    publicTimelinesByTopic,
    liveSessionsByTopic,
    gameRoomsByTopic,
    joinedChannelsByTopic,
    selectedChannelIdByTopic,
    timelineScopeByTopic,
    composeChannelByTopic,
    thread,
    selectedThread,
    replyTarget,
    repostTarget,
    peerTicket,
    localPeerTicket,
    discoveryConfig,
    discoverySeedInput,
    discoveryEditorDirty,
    discoveryError,
    communityNodeConfig,
    communityNodeStatuses,
    communityNodeInput,
    communityNodeEditorDirty,
    communityNodeError,
    mediaObjectUrls,
    unsupportedVideoManifests,
    syncStatus,
    localProfile,
    profileTimeline,
    knownAuthorsByPubkey,
    socialConnections,
    socialConnectionsPanelState,
    ownedReactionAssets,
    bookmarkedReactionAssets,
    bookmarkedPosts,
    recentReactions,
    profileDraft,
    profileDirty,
    profileError,
    profilePanelState,
    profileSaving,
    selectedAuthorPubkey,
    selectedAuthor,
    selectedAuthorTimeline,
    authorError,
    directMessagePaneOpen,
    selectedDirectMessagePeerPubkey,
    directMessages,
    directMessageTimelineByPeer,
    directMessageStatusByPeer,
    directMessageComposer,
    directMessageDraftMediaItems,
    directMessageAttachmentInputKey,
    directMessageError,
    directMessageSending,
    composerError,
    liveTitle,
    liveDescription,
    liveError,
    livePanelStateByTopic,
    liveCreatePending,
    livePendingBySessionId,
    channelLabelInput,
    channelAudienceInput,
    inviteTokenInput,
    inviteOutput,
    inviteOutputLabel,
    channelError,
    channelPanelStateByTopic,
    channelActionPending,
    gameTitle,
    gameDescription,
    gameParticipantsInput,
    gameError,
    gameDrafts,
    gamePanelStateByTopic,
    gameCreatePending,
    gameSavingByRoomId,
    reactionPanelState,
    reactionCreatePending,
    error,
    shellChromeState,
    setField,
  } = useDesktopShellStore();
  const [composeDialogOpen, setComposeDialogOpen] = useState(false);
  const [channelDialogOpen, setChannelDialogOpen] = useState(false);
  const [liveCreateDialogOpen, setLiveCreateDialogOpen] = useState(false);
  const [gameCreateDialogOpen, setGameCreateDialogOpen] = useState(false);
  const [profileAvatarPreviewUrl, setProfileAvatarPreviewUrl] = useState<string | null>(null);
  const [profileAvatarInputKey, setProfileAvatarInputKey] = useState(0);
  const previousPrimarySectionRef = useRef(shellChromeState.activePrimarySection);
  const previousTimelineViewRef = useRef(shellChromeState.timelineView);

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  useEffect(
    () => () => {
      if (profileAvatarPreviewUrl) {
        URL.revokeObjectURL(profileAvatarPreviewUrl);
      }
    },
    [profileAvatarPreviewUrl]
  );

  useEffect(() => {
    const previousPrimarySection = previousPrimarySectionRef.current;
    const previousTimelineView = previousTimelineViewRef.current;
    const enteredBookmarkTimeline =
      previousPrimarySection === 'timeline' &&
      shellChromeState.activePrimarySection === 'timeline' &&
      previousTimelineView !== 'bookmarks' &&
      shellChromeState.timelineView === 'bookmarks';

    if (
      (shellChromeState.activePrimarySection !== 'timeline' || enteredBookmarkTimeline) &&
      composeDialogOpen
    ) {
      setComposeDialogOpen(false);
    }
    if (shellChromeState.activePrimarySection !== 'live' && liveCreateDialogOpen) {
      setLiveCreateDialogOpen(false);
    }
    if (shellChromeState.activePrimarySection !== 'game' && gameCreateDialogOpen) {
      setGameCreateDialogOpen(false);
    }
    previousPrimarySectionRef.current = shellChromeState.activePrimarySection;
    previousTimelineViewRef.current = shellChromeState.timelineView;
  }, [
    composeDialogOpen,
    gameCreateDialogOpen,
    liveCreateDialogOpen,
    shellChromeState.activePrimarySection,
    shellChromeState.timelineView,
  ]);

  const makeFieldSetter = useCallback(
    function <K extends keyof DesktopShellState>(key: K) {
      return (value: DesktopShellStateValue<K>) => setField(key, value);
    },
    [setField]
  );
  const setTrackedTopics = useMemo(() => makeFieldSetter('trackedTopics'), [makeFieldSetter]);
  const setActiveTopic = useMemo(() => makeFieldSetter('activeTopic'), [makeFieldSetter]);
  const setTopicInput = useMemo(() => makeFieldSetter('topicInput'), [makeFieldSetter]);
  const setComposer = useMemo(() => makeFieldSetter('composer'), [makeFieldSetter]);
  const setDraftMediaItems = useMemo(() => makeFieldSetter('draftMediaItems'), [makeFieldSetter]);
  const setAttachmentInputKey = useMemo(
    () => makeFieldSetter('attachmentInputKey'),
    [makeFieldSetter]
  );
  const setTimelinesByTopic = useMemo(() => makeFieldSetter('timelinesByTopic'), [makeFieldSetter]);
  const setPublicTimelinesByTopic = useMemo(
    () => makeFieldSetter('publicTimelinesByTopic'),
    [makeFieldSetter]
  );
  const setLiveSessionsByTopic = useMemo(
    () => makeFieldSetter('liveSessionsByTopic'),
    [makeFieldSetter]
  );
  const setGameRoomsByTopic = useMemo(() => makeFieldSetter('gameRoomsByTopic'), [makeFieldSetter]);
  const setJoinedChannelsByTopic = useMemo(
    () => makeFieldSetter('joinedChannelsByTopic'),
    [makeFieldSetter]
  );
  const setSelectedChannelIdByTopic = useMemo(
    () => makeFieldSetter('selectedChannelIdByTopic'),
    [makeFieldSetter]
  );
  const setTimelineScopeByTopic = useMemo(
    () => makeFieldSetter('timelineScopeByTopic'),
    [makeFieldSetter]
  );
  const setComposeChannelByTopic = useMemo(
    () => makeFieldSetter('composeChannelByTopic'),
    [makeFieldSetter]
  );
  const setThread = useMemo(() => makeFieldSetter('thread'), [makeFieldSetter]);
  const setSelectedThread = useMemo(() => makeFieldSetter('selectedThread'), [makeFieldSetter]);
  const setReplyTarget = useMemo(() => makeFieldSetter('replyTarget'), [makeFieldSetter]);
  const setRepostTarget = useMemo(() => makeFieldSetter('repostTarget'), [makeFieldSetter]);
  const setPeerTicket = useMemo(() => makeFieldSetter('peerTicket'), [makeFieldSetter]);
  const setLocalPeerTicket = useMemo(() => makeFieldSetter('localPeerTicket'), [makeFieldSetter]);
  const setDiscoveryConfig = useMemo(() => makeFieldSetter('discoveryConfig'), [makeFieldSetter]);
  const setDiscoverySeedInput = useMemo(
    () => makeFieldSetter('discoverySeedInput'),
    [makeFieldSetter]
  );
  const setDiscoveryEditorDirty = useMemo(
    () => makeFieldSetter('discoveryEditorDirty'),
    [makeFieldSetter]
  );
  const setDiscoveryError = useMemo(() => makeFieldSetter('discoveryError'), [makeFieldSetter]);
  const setCommunityNodeConfig = useMemo(
    () => makeFieldSetter('communityNodeConfig'),
    [makeFieldSetter]
  );
  const setCommunityNodeStatuses = useMemo(
    () => makeFieldSetter('communityNodeStatuses'),
    [makeFieldSetter]
  );
  const setCommunityNodeInput = useMemo(
    () => makeFieldSetter('communityNodeInput'),
    [makeFieldSetter]
  );
  const setCommunityNodeEditorDirty = useMemo(
    () => makeFieldSetter('communityNodeEditorDirty'),
    [makeFieldSetter]
  );
  const setCommunityNodeError = useMemo(
    () => makeFieldSetter('communityNodeError'),
    [makeFieldSetter]
  );
  const setMediaObjectUrls = useMemo(() => makeFieldSetter('mediaObjectUrls'), [makeFieldSetter]);
  const setUnsupportedVideoManifests = useMemo(
    () => makeFieldSetter('unsupportedVideoManifests'),
    [makeFieldSetter]
  );
  const setSyncStatus = useMemo(() => makeFieldSetter('syncStatus'), [makeFieldSetter]);
  const setLocalProfile = useMemo(() => makeFieldSetter('localProfile'), [makeFieldSetter]);
  const setProfileTimeline = useMemo(() => makeFieldSetter('profileTimeline'), [makeFieldSetter]);
  const setKnownAuthorsByPubkey = useMemo(
    () => makeFieldSetter('knownAuthorsByPubkey'),
    [makeFieldSetter]
  );
  const setSocialConnections = useMemo(
    () => makeFieldSetter('socialConnections'),
    [makeFieldSetter]
  );
  const setSocialConnectionsPanelState = useMemo(
    () => makeFieldSetter('socialConnectionsPanelState'),
    [makeFieldSetter]
  );
  const setOwnedReactionAssets = useMemo(
    () => makeFieldSetter('ownedReactionAssets'),
    [makeFieldSetter]
  );
  const setBookmarkedReactionAssets = useMemo(
    () => makeFieldSetter('bookmarkedReactionAssets'),
    [makeFieldSetter]
  );
  const setBookmarkedPosts = useMemo(() => makeFieldSetter('bookmarkedPosts'), [makeFieldSetter]);
  const setRecentReactions = useMemo(
    () => makeFieldSetter('recentReactions'),
    [makeFieldSetter]
  );
  const setProfileDraft = useMemo(() => makeFieldSetter('profileDraft'), [makeFieldSetter]);
  const setProfileDirty = useMemo(() => makeFieldSetter('profileDirty'), [makeFieldSetter]);
  const setProfileError = useMemo(() => makeFieldSetter('profileError'), [makeFieldSetter]);
  const setProfilePanelState = useMemo(
    () => makeFieldSetter('profilePanelState'),
    [makeFieldSetter]
  );
  const setProfileSaving = useMemo(() => makeFieldSetter('profileSaving'), [makeFieldSetter]);
  const setSelectedAuthorPubkey = useMemo(
    () => makeFieldSetter('selectedAuthorPubkey'),
    [makeFieldSetter]
  );
  const setSelectedAuthor = useMemo(() => makeFieldSetter('selectedAuthor'), [makeFieldSetter]);
  const setSelectedAuthorTimeline = useMemo(
    () => makeFieldSetter('selectedAuthorTimeline'),
    [makeFieldSetter]
  );
  const setAuthorError = useMemo(() => makeFieldSetter('authorError'), [makeFieldSetter]);
  const setDirectMessagePaneOpen = useMemo(
    () => makeFieldSetter('directMessagePaneOpen'),
    [makeFieldSetter]
  );
  const setSelectedDirectMessagePeerPubkey = useMemo(
    () => makeFieldSetter('selectedDirectMessagePeerPubkey'),
    [makeFieldSetter]
  );
  const setDirectMessages = useMemo(() => makeFieldSetter('directMessages'), [makeFieldSetter]);
  const setDirectMessageTimelineByPeer = useMemo(
    () => makeFieldSetter('directMessageTimelineByPeer'),
    [makeFieldSetter]
  );
  const setDirectMessageStatusByPeer = useMemo(
    () => makeFieldSetter('directMessageStatusByPeer'),
    [makeFieldSetter]
  );
  const setDirectMessageComposer = useMemo(
    () => makeFieldSetter('directMessageComposer'),
    [makeFieldSetter]
  );
  const setDirectMessageDraftMediaItems = useMemo(
    () => makeFieldSetter('directMessageDraftMediaItems'),
    [makeFieldSetter]
  );
  const setDirectMessageAttachmentInputKey = useMemo(
    () => makeFieldSetter('directMessageAttachmentInputKey'),
    [makeFieldSetter]
  );
  const setDirectMessageError = useMemo(
    () => makeFieldSetter('directMessageError'),
    [makeFieldSetter]
  );
  const setDirectMessageSending = useMemo(
    () => makeFieldSetter('directMessageSending'),
    [makeFieldSetter]
  );
  const setComposerError = useMemo(() => makeFieldSetter('composerError'), [makeFieldSetter]);
  const setLiveTitle = useMemo(() => makeFieldSetter('liveTitle'), [makeFieldSetter]);
  const setLiveDescription = useMemo(() => makeFieldSetter('liveDescription'), [makeFieldSetter]);
  const setLiveError = useMemo(() => makeFieldSetter('liveError'), [makeFieldSetter]);
  const setLivePanelStateByTopic = useMemo(
    () => makeFieldSetter('livePanelStateByTopic'),
    [makeFieldSetter]
  );
  const setLiveCreatePending = useMemo(
    () => makeFieldSetter('liveCreatePending'),
    [makeFieldSetter]
  );
  const setLivePendingBySessionId = useMemo(
    () => makeFieldSetter('livePendingBySessionId'),
    [makeFieldSetter]
  );
  const setChannelLabelInput = useMemo(
    () => makeFieldSetter('channelLabelInput'),
    [makeFieldSetter]
  );
  const setChannelAudienceInput = useMemo(
    () => makeFieldSetter('channelAudienceInput'),
    [makeFieldSetter]
  );
  const setInviteTokenInput = useMemo(
    () => makeFieldSetter('inviteTokenInput'),
    [makeFieldSetter]
  );
  const setInviteOutput = useMemo(() => makeFieldSetter('inviteOutput'), [makeFieldSetter]);
  const setInviteOutputLabel = useMemo(
    () => makeFieldSetter('inviteOutputLabel'),
    [makeFieldSetter]
  );
  const setChannelError = useMemo(() => makeFieldSetter('channelError'), [makeFieldSetter]);
  const setChannelPanelStateByTopic = useMemo(
    () => makeFieldSetter('channelPanelStateByTopic'),
    [makeFieldSetter]
  );
  const setChannelActionPending = useMemo(
    () => makeFieldSetter('channelActionPending'),
    [makeFieldSetter]
  );
  const setGameTitle = useMemo(() => makeFieldSetter('gameTitle'), [makeFieldSetter]);
  const setGameDescription = useMemo(
    () => makeFieldSetter('gameDescription'),
    [makeFieldSetter]
  );
  const setGameParticipantsInput = useMemo(
    () => makeFieldSetter('gameParticipantsInput'),
    [makeFieldSetter]
  );
  const setGameError = useMemo(() => makeFieldSetter('gameError'), [makeFieldSetter]);
  const setGameDrafts = useMemo(() => makeFieldSetter('gameDrafts'), [makeFieldSetter]);
  const setGamePanelStateByTopic = useMemo(
    () => makeFieldSetter('gamePanelStateByTopic'),
    [makeFieldSetter]
  );
  const setGameCreatePending = useMemo(
    () => makeFieldSetter('gameCreatePending'),
    [makeFieldSetter]
  );
  const setGameSavingByRoomId = useMemo(
    () => makeFieldSetter('gameSavingByRoomId'),
    [makeFieldSetter]
  );
  const setReactionPanelState = useMemo(
    () => makeFieldSetter('reactionPanelState'),
    [makeFieldSetter]
  );
  const setReactionCreatePending = useMemo(
    () => makeFieldSetter('reactionCreatePending'),
    [makeFieldSetter]
  );
  const setError = useMemo(() => makeFieldSetter('error'), [makeFieldSetter]);
  const setShellChromeState = useMemo(
    () => makeFieldSetter('shellChromeState'),
    [makeFieldSetter]
  );
  const draftSequenceRef = useRef(0);
  const mediaFetchAttemptRef = useRef(new Map<string, number>());
  const remoteObjectUrlRef = useRef(new Map<string, string>());
  const draftPreviewUrlRef = useRef(new Map<string, string>());
  const directMessageDraftPreviewUrlRef = useRef(new Map<string, string>());
  const loadTopicsRequestRef = useRef(0);
  const pendingRouteUrlRef = useRef<string | null>(null);
  const didSyncRouteSectionRef = useRef(false);
  const navTriggerRef = useRef<HTMLButtonElement | null>(null);
  const settingsTriggerRef = useRef<HTMLButtonElement | null>(null);
  const primarySectionRefs = useRef<Record<PrimarySection, HTMLElement | null>>({
    timeline: null,
    live: null,
    game: null,
    messages: null,
    profile: null,
  });
  const location = useLocation();
  const navigate = useNavigate();
  const routeSection = useMemo(
    () => parsePrimarySectionPath(location.pathname) ?? 'timeline',
    [location.pathname]
  );

  const activeTimeline = useMemo(
    () => timelinesByTopic[activeTopic] ?? [],
    [activeTopic, timelinesByTopic]
  );
  const activePublicTimeline = useMemo(
    () => publicTimelinesByTopic[activeTopic] ?? [],
    [activeTopic, publicTimelinesByTopic]
  );
  const activeLiveSessions = useMemo(
    () => liveSessionsByTopic[activeTopic] ?? [],
    [activeTopic, liveSessionsByTopic]
  );
  const activeGameRooms = useMemo(
    () => gameRoomsByTopic[activeTopic] ?? [],
    [activeTopic, gameRoomsByTopic]
  );
  const activeJoinedChannels = useMemo(
    () => joinedChannelsByTopic[activeTopic] ?? [],
    [activeTopic, joinedChannelsByTopic]
  );
  const selectedPrivateChannelId = useMemo(
    () => selectedChannelIdByTopic[activeTopic] ?? null,
    [activeTopic, selectedChannelIdByTopic]
  );
  const activeTimelineScope = useMemo(
    () => privateTimelineScope(selectedPrivateChannelId),
    [selectedPrivateChannelId]
  );
  const activeComposeChannel = useMemo(() => {
    if (repostTarget) {
      return PUBLIC_CHANNEL_REF;
    }
    if (replyTarget?.channel_id) {
      return {
        kind: 'private_channel',
        channel_id: replyTarget.channel_id,
      } as ChannelRef;
    }
    return privateComposeTarget(selectedPrivateChannelId);
  }, [replyTarget, repostTarget, selectedPrivateChannelId]);
  const activeComposeAudienceLabel = useMemo(() => {
    if (repostTarget) {
      return translate('common:audience.public');
    }
    if (replyTarget) {
      return replyTarget.audience_label;
    }
    return audienceLabelForChannelRef(activeComposeChannel, activeJoinedChannels);
  }, [activeComposeChannel, activeJoinedChannels, replyTarget, repostTarget]);
  const profileMode = shellChromeState.profileMode;
  const profileConnectionsView = shellChromeState.profileConnectionsView;
  const activeSocialConnections = socialConnections[profileConnectionsView] ?? [];
  const activeSocialConnectionViews = useMemo(
    () =>
      activeSocialConnections.map((author) => {
        const knownAuthor = knownAuthorsByPubkey[author.author_pubkey] ?? author;
        return {
          ...author,
          picture_src: resolveProfilePictureSrc(knownAuthor, mediaObjectUrls),
        };
      }),
    [activeSocialConnections, knownAuthorsByPubkey, mediaObjectUrls]
  );
  const activePrivateChannel = useMemo(
    () =>
      selectedPrivateChannelId
        ? activeJoinedChannels.find((channel) => channel.channel_id === selectedPrivateChannelId) ?? null
        : null,
    [activeJoinedChannels, selectedPrivateChannelId]
  );
  const activeChannelPanelState = useMemo(
    () => channelPanelStateByTopic[activeTopic] ?? DEFAULT_ASYNC_PANEL_STATE,
    [activeTopic, channelPanelStateByTopic]
  );
  const activeLivePanelState = useMemo(
    () => livePanelStateByTopic[activeTopic] ?? DEFAULT_ASYNC_PANEL_STATE,
    [activeTopic, livePanelStateByTopic]
  );
  const activeGamePanelState = useMemo(
    () => gamePanelStateByTopic[activeTopic] ?? DEFAULT_ASYNC_PANEL_STATE,
    [activeTopic, gamePanelStateByTopic]
  );
  const selectedDirectMessageConversation = useMemo(
    () =>
      selectedDirectMessagePeerPubkey
        ? directMessages.find((conversation) => conversation.peer_pubkey === selectedDirectMessagePeerPubkey) ?? null
        : null,
    [directMessages, selectedDirectMessagePeerPubkey]
  );
  const selectedDirectMessageTimeline = useMemo(
    () =>
      selectedDirectMessagePeerPubkey
        ? directMessageTimelineByPeer[selectedDirectMessagePeerPubkey] ?? []
        : [],
    [directMessageTimelineByPeer, selectedDirectMessagePeerPubkey]
  );
  const selectedDirectMessageStatus = useMemo(
    () =>
      selectedDirectMessagePeerPubkey
        ? directMessageStatusByPeer[selectedDirectMessagePeerPubkey] ??
          selectedDirectMessageConversation?.status ??
          null
        : null,
    [directMessageStatusByPeer, selectedDirectMessageConversation, selectedDirectMessagePeerPubkey]
  );
  const channelAudienceOptions = useMemo<ChannelAudienceOption[]>(
    () => [
      {
        value: 'invite_only',
        label: t('channels:audienceOptions.invite_only'),
      },
      {
        value: 'friend_only',
        label: t('channels:audienceOptions.friend_only'),
      },
      {
        value: 'friend_plus',
        label: t('channels:audienceOptions.friend_plus'),
      },
    ],
    [t]
  );
  const privateChannelListItems = useMemo<PrivateChannelListItemView[]>(
    () =>
      activeJoinedChannels.map((channel) => ({
        channel,
        active: channel.channel_id === selectedPrivateChannelId,
      })),
    [activeJoinedChannels, selectedPrivateChannelId]
  );
  const floatingActionLabel = useMemo(() => {
    if (shellChromeState.activePrimarySection === 'live') {
      return t('live:actions.start');
    }
    if (shellChromeState.activePrimarySection === 'game') {
      return t('game:actions.createRoom');
    }
    return t('common:actions.publish');
  }, [shellChromeState.activePrimarySection, t]);
  const showFloatingActionButton =
    shellChromeState.activePrimarySection !== 'profile' &&
    shellChromeState.activePrimarySection !== 'messages' &&
    !(
      shellChromeState.activePrimarySection === 'timeline' &&
      shellChromeState.timelineView === 'bookmarks'
    );
  const syncRoute = useCallback((
    mode: 'push' | 'replace' = 'replace',
    overrides?: DesktopShellRouteOverrides
  ) => {
    const hasOverride = <K extends keyof DesktopShellRouteOverrides>(key: K) =>
      overrides ? Object.prototype.hasOwnProperty.call(overrides, key) : false;
    const search = new URLSearchParams();
    const nextTopic = overrides?.activeTopic ?? activeTopic;
    const nextPrimarySection = overrides?.primarySection ?? shellChromeState.activePrimarySection;
    const resolvedPrimarySection = nextPrimarySection;
    const nextTimelineView = overrides?.timelineView ?? shellChromeState.timelineView;
    const nextProfileMode = overrides?.profileMode ?? shellChromeState.profileMode;
    const nextProfileConnectionsView =
      overrides?.profileConnectionsView ?? shellChromeState.profileConnectionsView;
    const nextSelectedThread = hasOverride('selectedThread')
      ? overrides?.selectedThread ?? null
      : selectedThread;
    const nextSelectedAuthorPubkey = hasOverride('selectedAuthorPubkey')
      ? overrides?.selectedAuthorPubkey ?? null
      : selectedAuthorPubkey;
    const nextSelectedDirectMessagePeerPubkey = hasOverride('selectedDirectMessagePeerPubkey')
      ? overrides?.selectedDirectMessagePeerPubkey ?? null
      : selectedDirectMessagePeerPubkey;
    const nextSettingsOpen = hasOverride('settingsOpen')
      ? overrides?.settingsOpen ?? false
      : shellChromeState.settingsOpen;
    const nextSettingsSection =
      overrides?.settingsSection ?? shellChromeState.activeSettingsSection;
    let nextSelectedChannelId = selectedChannelIdByTopic[nextTopic] ?? null;

    if (hasOverride('composeTarget')) {
      nextSelectedChannelId =
        overrides?.composeTarget?.kind === 'private_channel'
          ? overrides.composeTarget.channel_id
          : null;
    } else if (hasOverride('timelineScope')) {
      nextSelectedChannelId =
        overrides?.timelineScope?.kind === 'channel' ? overrides.timelineScope.channel_id : null;
    }

    search.set('topic', nextTopic);
    if (
      resolvedPrimarySection !== 'messages' &&
      nextSelectedChannelId &&
      !(resolvedPrimarySection === 'timeline' && nextTimelineView === 'bookmarks')
    ) {
      search.set('channel', nextSelectedChannelId);
    }
    if (resolvedPrimarySection === 'timeline' && nextTimelineView === 'bookmarks') {
      search.set('timelineView', 'bookmarks');
    }
    if (resolvedPrimarySection === 'messages') {
      if (nextSelectedDirectMessagePeerPubkey) {
        search.set('peerPubkey', nextSelectedDirectMessagePeerPubkey);
      }
    } else if (nextSelectedThread) {
      search.set('context', 'thread');
      search.set('threadId', nextSelectedThread);
      if (nextSelectedAuthorPubkey) {
        search.set('authorPubkey', nextSelectedAuthorPubkey);
      }
    } else if (nextSelectedAuthorPubkey) {
      search.set('context', 'author');
      search.set('authorPubkey', nextSelectedAuthorPubkey);
    }
    if (resolvedPrimarySection === 'profile' && nextProfileMode === 'edit') {
      search.set('profileMode', 'edit');
    }
    if (resolvedPrimarySection === 'profile' && nextProfileMode === 'connections') {
      search.set('profileMode', 'connections');
      search.set('connectionsView', nextProfileConnectionsView);
    }
    if (nextSettingsOpen) {
      search.set('settings', nextSettingsSection);
    }

    const nextPath = PRIMARY_SECTION_PATHS[resolvedPrimarySection];
    const nextSearch = search.toString();
    const nextUrl = nextSearch ? `${nextPath}?${nextSearch}` : nextPath;
    const currentUrl = `${location.pathname}${location.search}`;
    if (currentUrl !== nextUrl) {
      pendingRouteUrlRef.current = nextUrl;
      navigate(nextUrl, { replace: mode === 'replace' });
      return;
    }
    pendingRouteUrlRef.current = null;
  }, [
    activeTopic,
    location.pathname,
    location.search,
    navigate,
    selectedDirectMessagePeerPubkey,
    selectedAuthorPubkey,
    selectedThread,
    selectedChannelIdByTopic,
    shellChromeState.activePrimarySection,
    shellChromeState.timelineView,
    shellChromeState.activeSettingsSection,
    shellChromeState.profileMode,
    shellChromeState.profileConnectionsView,
    shellChromeState.settingsOpen,
  ]);
  const liveSessionListItems = useMemo(
    () =>
      activeLiveSessions.map((session) => ({
        session,
        isOwner: session.host_pubkey === syncStatus.local_author_pubkey,
        pending: Boolean(livePendingBySessionId[session.session_id]),
      })),
    [activeLiveSessions, livePendingBySessionId, syncStatus.local_author_pubkey]
  );
  const gameDraftViews = useMemo<Record<string, GameDraftView>>(
    () =>
      Object.fromEntries(
        activeGameRooms.map((room) => {
          const draft = gameDrafts[room.room_id] ?? createGameEditorDraft(room);
          return [
            room.room_id,
            {
              status: draft.status,
              phaseLabel: draft.phase_label,
              scores: draft.scores,
            },
          ];
        })
      ),
    [activeGameRooms, gameDrafts]
  );
  const profileEditorFields = useMemo(
    () => ({
      displayName: profileDraft.display_name ?? '',
      name: profileDraft.name ?? '',
      about: profileDraft.about ?? '',
    }),
    [profileDraft]
  );
  const profileEditorPictureSrc = profileAvatarPreviewUrl
    ?? resolveProfilePictureSrc(localProfile, mediaObjectUrls);
  const profileEditorHasPicture = Boolean(
    profileAvatarPreviewUrl
      || profileDraft.clear_picture
      || profileDraft.picture_upload
      || localProfile?.picture
      || localProfile?.picture_asset
  ) && !profileDraft.clear_picture;
  const communityNodeStatusByBaseUrl = useMemo(
    () =>
      Object.fromEntries(communityNodeStatuses.map((status) => [status.base_url, status])) as Record<
        string,
        CommunityNodeNodeStatus
      >,
    [communityNodeStatuses]
  );
  const topicDiagnostics = useMemo(
    () =>
      Object.fromEntries(
        syncStatus.topic_diagnostics.map((diagnostic) => [diagnostic.topic, diagnostic])
      ) as Record<string, TopicSyncStatus>,
    [syncStatus.topic_diagnostics]
  );
  const effectivePeerIds = useMemo(
    () =>
      [
        ...new Set([
          ...syncStatus.topic_diagnostics.flatMap((diagnostic) => diagnostic.connected_peers),
          ...syncStatus.discovery.assist_peer_ids,
        ]),
      ],
    [syncStatus.discovery.assist_peer_ids, syncStatus.topic_diagnostics]
  );
  const previewableMediaAttachments = useMemo(() => {
    const attachments = new Map<string, AttachmentView>();
    const tryAddAttachment = (attachment: AttachmentView | null) => {
      if (!attachment) {
        return;
      }
      const hash = attachment.hash.trim();
      const mime = attachment.mime.trim();
      if (!hash || !mime) {
        logMediaDebug('warn', 'remote media metadata skipped', {
          hash: attachment.hash || null,
          mime: attachment.mime || null,
          role: attachment.role,
          status: attachment.status,
        });
        return;
      }
      attachments.set(hash, {
        ...attachment,
        hash,
        mime,
      });
    };
    for (const post of [
      ...activeTimeline,
      ...activePublicTimeline,
      ...profileTimeline,
      ...selectedAuthorTimeline,
      ...thread,
    ]) {
      for (const attachment of [
        selectPrimaryImage(post),
        selectVideoPoster(post),
        selectVideoManifest(post),
      ]) {
        tryAddAttachment(attachment);
      }
    }
    for (const message of selectedDirectMessageTimeline) {
      for (const attachment of [
        selectPrimaryImageAttachment(message.attachments),
        selectVideoPosterAttachment(message.attachments),
        selectVideoManifestAttachment(message.attachments),
      ]) {
        tryAddAttachment(attachment);
      }
    }
    for (const asset of [...ownedReactionAssets, ...bookmarkedReactionAssets]) {
      tryAddAttachment({
        hash: asset.blob_hash,
        mime: asset.mime,
        bytes: asset.bytes,
        role: 'image_original',
        status: 'Available',
      });
    }
    for (const pictureAsset of [
      localProfile?.picture_asset ?? null,
      ...Object.values(knownAuthorsByPubkey).map((author) => author.picture_asset ?? null),
    ]) {
      tryAddAttachment(
        pictureAsset
          ? {
              hash: pictureAsset.hash,
              mime: pictureAsset.mime,
              bytes: pictureAsset.bytes,
              role: pictureAsset.role,
              status: 'Available',
            }
          : null
      );
    }
    return [...attachments.values()];
  }, [
    activePublicTimeline,
    activeTimeline,
    bookmarkedReactionAssets,
    knownAuthorsByPubkey,
    localProfile?.picture_asset,
    ownedReactionAssets,
    profileTimeline,
    selectedDirectMessageTimeline,
    selectedAuthorTimeline,
    thread,
  ]);

  const loadTopics = useCallback(
    async (currentTopics: string[], currentActiveTopic: string, currentThread: string | null) => {
      const requestId = loadTopicsRequestRef.current + 1;
      loadTopicsRequestRef.current = requestId;
      const currentState = storeApi.getState();
      const currentSelectedChannelIdByTopic = currentState.selectedChannelIdByTopic;
      const currentSelectedAuthorPubkey = currentState.selectedAuthorPubkey;
      const currentDirectMessagePaneOpen = currentState.directMessagePaneOpen;
      const currentSelectedDirectMessagePeerPubkey = currentState.selectedDirectMessagePeerPubkey;
      const currentDiscoveryEditorDirty = currentState.discoveryEditorDirty;
      const currentCommunityNodeEditorDirty = currentState.communityNodeEditorDirty;
      const currentProfileDirty = currentState.profileDirty;

      try {
        const [
          timelineViews,
          publicTimelineViews,
          liveViewsResult,
          gameViewsResult,
          joinedChannelViewsResult,
          threadView,
          directMessagesView,
          status,
        ] = await Promise.all([
          Promise.all(
            currentTopics.map(async (topic) => ({
                topic,
                timeline: await api.listTimeline(
                  topic,
                  null,
                  50,
                  privateTimelineScope(currentSelectedChannelIdByTopic[topic] ?? null)
                ),
              }))
          ),
          Promise.all(
            currentTopics.map(async (topic) => ({
              topic,
              timeline: await api.listTimeline(topic, null, 50, PUBLIC_TIMELINE_SCOPE),
            }))
          ),
          Promise.allSettled(
            currentTopics.map(async (topic) => ({
              topic,
              sessions: await api.listLiveSessions(
                topic,
                privateTimelineScope(currentSelectedChannelIdByTopic[topic] ?? null)
              ),
            }))
          ),
          Promise.allSettled(
            currentTopics.map(async (topic) => ({
              topic,
              rooms: await api.listGameRooms(
                topic,
                privateTimelineScope(currentSelectedChannelIdByTopic[topic] ?? null)
              ),
            }))
          ),
          Promise.allSettled(
            currentTopics.map(async (topic) => ({
              topic,
              channels: await api.listJoinedPrivateChannels(topic),
            }))
          ),
          currentThread
            ? api.listThread(currentActiveTopic, currentThread, null, 50)
            : Promise.resolve(null),
          api.listDirectMessages(),
          api.getSyncStatus(),
        ]);
        const [
          discoveryResult,
          communityConfigResult,
          communityStatusesResult,
          ticketResult,
          profileResult,
          authorViewResult,
          profileTimelineResult,
          authorTimelineResult,
          directMessageTimelineResult,
          directMessageStatusResult,
          ownedReactionAssetsResult,
          bookmarkedReactionAssetsResult,
          bookmarkedPostsResult,
          recentReactionsResult,
          followingConnectionsResult,
          followedConnectionsResult,
          mutedConnectionsResult,
        ] = await Promise.allSettled([
          api.getDiscoveryConfig(),
          api.getCommunityNodeConfig(),
          api.getCommunityNodeStatuses(),
          api.getLocalPeerTicket(),
          api.getMyProfile(),
          currentSelectedAuthorPubkey
            ? api.getAuthorSocialView(currentSelectedAuthorPubkey)
            : Promise.resolve(null),
          api.listProfileTimeline(status.local_author_pubkey, null, 50),
          currentSelectedAuthorPubkey
            ? api.listProfileTimeline(currentSelectedAuthorPubkey, null, 50)
            : Promise.resolve(null),
          currentDirectMessagePaneOpen && currentSelectedDirectMessagePeerPubkey
            ? api.listDirectMessageMessages(currentSelectedDirectMessagePeerPubkey, null, 100)
            : Promise.resolve(null),
          currentDirectMessagePaneOpen && currentSelectedDirectMessagePeerPubkey
            ? api.getDirectMessageStatus(currentSelectedDirectMessagePeerPubkey)
            : Promise.resolve(null),
          api.listMyCustomReactionAssets(),
          api.listBookmarkedCustomReactions(),
          api.listBookmarkedPosts(),
          api.listRecentReactions(8),
          api.listSocialConnections('following'),
          api.listSocialConnections('followed'),
          api.listSocialConnections('muted'),
        ]);
        if (requestId !== loadTopicsRequestRef.current) {
          return;
        }
        startTransition(() => {
          setTimelinesByTopic(
            Object.fromEntries(timelineViews.map(({ topic, timeline }) => [topic, timeline.items]))
          );
          setPublicTimelinesByTopic(
            Object.fromEntries(
              publicTimelineViews.map(({ topic, timeline }) => [topic, timeline.items])
            )
          );
          setLiveSessionsByTopic((current) => {
            const next = { ...current };
            for (const result of liveViewsResult) {
              if (result.status === 'fulfilled') {
                next[result.value.topic] = result.value.sessions;
              }
            }
            return next;
          });
          setGameRoomsByTopic((current) => {
            const next = { ...current };
            for (const result of gameViewsResult) {
              if (result.status === 'fulfilled') {
                next[result.value.topic] = result.value.rooms;
              }
            }
            return next;
          });
          setJoinedChannelsByTopic((current) => {
            const next = { ...current };
            for (const result of joinedChannelViewsResult) {
              if (result.status === 'fulfilled') {
                next[result.value.topic] = result.value.channels;
              }
            }
            return next;
          });
          setLivePanelStateByTopic((current) => {
            const next = { ...current };
            for (const [index, result] of liveViewsResult.entries()) {
              if (result.status === 'fulfilled') {
                next[result.value.topic] = {
                  status: 'ready',
                  error: null,
                };
              } else {
                next[currentTopics[index]] = {
                  status: 'error',
                  error: messageFromError(
                    result.reason,
                    translate('common:errors.failedToLoadLiveSessions')
                  ),
                };
              }
            }
            return next;
          });
          setGamePanelStateByTopic((current) => {
            const next = { ...current };
            for (const [index, result] of gameViewsResult.entries()) {
              if (result.status === 'fulfilled') {
                next[result.value.topic] = {
                  status: 'ready',
                  error: null,
                };
              } else {
                next[currentTopics[index]] = {
                  status: 'error',
                  error: messageFromError(
                    result.reason,
                    translate('common:errors.failedToLoadGameRooms')
                  ),
                };
              }
            }
            return next;
          });
          setChannelPanelStateByTopic((current) => {
            const next = { ...current };
            for (const [index, result] of joinedChannelViewsResult.entries()) {
              if (result.status === 'fulfilled') {
                next[result.value.topic] = {
                  status: 'ready',
                  error: null,
                };
              } else {
                next[currentTopics[index]] = {
                  status: 'error',
                  error: messageFromError(
                    result.reason,
                    translate('common:errors.failedToLoadPrivateChannels')
                  ),
                };
              }
            }
            return next;
          });
          setDirectMessages(directMessagesView);
          setKnownAuthorsByPubkey((current) =>
            mergeKnownAuthors(current, directMessagesView.map(authorViewFromDirectMessageConversation))
          );
          setSyncStatus(status);
          if (discoveryResult.status === 'fulfilled') {
            setDiscoveryConfig(discoveryResult.value);
            if (!currentDiscoveryEditorDirty) {
              setDiscoverySeedInput(seedPeersToEditorValue(discoveryResult.value));
            }
          }
          if (communityConfigResult.status === 'fulfilled') {
            setCommunityNodeConfig(communityConfigResult.value);
            if (!currentCommunityNodeEditorDirty) {
              setCommunityNodeInput(communityNodesToEditorValue(communityConfigResult.value));
            }
          }
          if (communityStatusesResult.status === 'fulfilled') {
            setCommunityNodeStatuses((current) =>
              mergeCommunityNodeStatuses(current, communityStatusesResult.value)
            );
          }
          if (ticketResult.status === 'fulfilled') {
            setLocalPeerTicket(ticketResult.value);
          }
          if (ownedReactionAssetsResult.status === 'fulfilled') {
            setOwnedReactionAssets(ownedReactionAssetsResult.value);
          }
          if (bookmarkedReactionAssetsResult.status === 'fulfilled') {
            setBookmarkedReactionAssets(bookmarkedReactionAssetsResult.value);
          }
          if (bookmarkedPostsResult.status === 'fulfilled') {
            setBookmarkedPosts(bookmarkedPostsResult.value);
          }
          if (recentReactionsResult.status === 'fulfilled') {
            setRecentReactions(recentReactionsResult.value);
          }
          if (
            followingConnectionsResult.status === 'fulfilled' &&
            followedConnectionsResult.status === 'fulfilled' &&
            mutedConnectionsResult.status === 'fulfilled'
          ) {
            setSocialConnections({
              following: followingConnectionsResult.value,
              followed: followedConnectionsResult.value,
              muted: mutedConnectionsResult.value,
            });
            setKnownAuthorsByPubkey((current) =>
              mergeKnownAuthors(current, [
                ...followingConnectionsResult.value,
                ...followedConnectionsResult.value,
                ...mutedConnectionsResult.value,
              ])
            );
            setSocialConnectionsPanelState({
              status: 'ready',
              error: null,
            });
          } else {
            setSocialConnections(DEFAULT_SOCIAL_CONNECTIONS);
            setSocialConnectionsPanelState({
              status: 'error',
              error:
                followingConnectionsResult.status === 'rejected'
                  ? messageFromError(
                      followingConnectionsResult.reason,
                      translate('common:errors.failedToLoadSocialConnections')
                    )
                  : followedConnectionsResult.status === 'rejected'
                    ? messageFromError(
                        followedConnectionsResult.reason,
                        translate('common:errors.failedToLoadSocialConnections')
                      )
                    : mutedConnectionsResult.status === 'rejected'
                      ? messageFromError(
                          mutedConnectionsResult.reason,
                          translate('common:errors.failedToLoadSocialConnections')
                        )
                      : null,
            });
          }
          setReactionPanelState({
            status:
              ownedReactionAssetsResult.status === 'fulfilled' &&
              bookmarkedReactionAssetsResult.status === 'fulfilled' &&
              recentReactionsResult.status === 'fulfilled'
                ? 'ready'
                : 'error',
            error:
              ownedReactionAssetsResult.status === 'rejected'
                ? messageFromError(
                    ownedReactionAssetsResult.reason,
                    translate('common:errors.failedToLoadSettings')
                  )
                : bookmarkedReactionAssetsResult.status === 'rejected'
                  ? messageFromError(
                      bookmarkedReactionAssetsResult.reason,
                      translate('common:errors.failedToLoadSettings')
                    )
                  : recentReactionsResult.status === 'rejected'
                    ? messageFromError(
                        recentReactionsResult.reason,
                        translate('common:errors.failedToLoadSettings')
                      )
                  : null,
          });
          if (profileResult.status === 'fulfilled') {
            setLocalProfile(profileResult.value);
            if (!currentProfileDirty) {
              setProfileDraft(profileInputFromProfile(profileResult.value));
            }
            if (profileTimelineResult.status === 'fulfilled') {
              setProfileTimeline(profileTimelineResult.value.items);
              setProfileError(null);
              setProfilePanelState({
                status: 'ready',
                error: null,
              });
            } else {
              const nextProfileError = messageFromError(
                profileTimelineResult.reason,
                translate('common:errors.failedToLoadProfile')
              );
              setProfileTimeline([]);
              setProfileError(nextProfileError);
              setProfilePanelState({
                status: 'error',
                error: nextProfileError,
              });
            }
          } else {
            const nextProfileError = messageFromError(
              profileResult.reason,
              translate('common:errors.failedToLoadProfile')
            );
            setProfileTimeline([]);
            setProfileError(nextProfileError);
            setProfilePanelState({
              status: 'error',
              error: nextProfileError,
            });
          }
          if (!currentSelectedAuthorPubkey) {
            setSelectedAuthor(null);
            setSelectedAuthorTimeline([]);
            setAuthorError(null);
          } else if (
            authorViewResult.status === 'fulfilled' &&
            authorTimelineResult.status === 'fulfilled'
          ) {
            setSelectedAuthor(authorViewResult.value);
            setSelectedAuthorTimeline(authorTimelineResult.value?.items ?? []);
            setAuthorError(null);
            if (authorViewResult.value) {
              setKnownAuthorsByPubkey((current) =>
                mergeKnownAuthors(current, [authorViewResult.value])
              );
            }
          } else {
            setSelectedAuthorTimeline([]);
            setAuthorError(
              messageFromError(
                authorViewResult.status === 'rejected'
                  ? authorViewResult.reason
                  : authorTimelineResult.status === 'rejected'
                    ? authorTimelineResult.reason
                    : null,
                translate('common:errors.failedToLoadAuthor')
              )
            );
          }
          if (!currentDirectMessagePaneOpen) {
            setSelectedDirectMessagePeerPubkey(null);
            setDirectMessageError(null);
          } else if (!currentSelectedDirectMessagePeerPubkey) {
            setDirectMessageError(null);
          } else if (
            directMessageTimelineResult.status === 'fulfilled' &&
            directMessageStatusResult.status === 'fulfilled'
          ) {
            setDirectMessageTimelineByPeer((current) => ({
              ...current,
              [currentSelectedDirectMessagePeerPubkey]: directMessageTimelineResult.value?.items ?? [],
            }));
            setDirectMessageStatusByPeer((current) => ({
              ...current,
              [currentSelectedDirectMessagePeerPubkey]: directMessageStatusResult.value!,
            }));
            setDirectMessageError(null);
          } else {
            setDirectMessageTimelineByPeer((current) => ({
              ...current,
              [currentSelectedDirectMessagePeerPubkey]: [],
            }));
            setDirectMessageError(
              messageFromError(
                directMessageTimelineResult.status === 'rejected'
                  ? directMessageTimelineResult.reason
                  : directMessageStatusResult.status === 'rejected'
                    ? directMessageStatusResult.reason
                    : null,
                'failed to load direct messages'
              )
            );
          }
          if (threadView) {
            setThread(threadView.items);
          } else if (!currentThread) {
            setThread([]);
          }
          setError(null);
        });
      } catch (loadError) {
        if (requestId !== loadTopicsRequestRef.current) {
          return;
        }
        setError(
          loadError instanceof Error
            ? loadError.message
            : translate('common:errors.failedToLoadTopic')
        );
      }
    },
    [
      api,
      setAuthorError,
      setChannelPanelStateByTopic,
      setCommunityNodeConfig,
      setCommunityNodeInput,
      setCommunityNodeStatuses,
      setDirectMessageError,
      setDirectMessages,
      setDirectMessageStatusByPeer,
      setDirectMessageTimelineByPeer,
      setSelectedDirectMessagePeerPubkey,
      setDiscoveryConfig,
      setDiscoverySeedInput,
      setError,
      setGamePanelStateByTopic,
      setGameRoomsByTopic,
      setJoinedChannelsByTopic,
      setLivePanelStateByTopic,
      setLiveSessionsByTopic,
      setLocalPeerTicket,
      setLocalProfile,
      setKnownAuthorsByPubkey,
      setOwnedReactionAssets,
      setProfileTimeline,
      setProfileDraft,
      setProfileError,
      setProfilePanelState,
      setBookmarkedReactionAssets,
      setBookmarkedPosts,
      setRecentReactions,
      setReactionPanelState,
      setSocialConnections,
      setSocialConnectionsPanelState,
      setSelectedAuthor,
      setSelectedAuthorTimeline,
      setSyncStatus,
      setThread,
      setTimelinesByTopic,
      setPublicTimelinesByTopic,
      storeApi,
    ]
  );

  useEffect(() => {
    let disposed = false;

    const refresh = async () => {
      if (disposed) {
        return;
      }
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    };

    void refresh();
    const intervalId = window.setInterval(() => {
      void refresh();
    }, REFRESH_INTERVAL_MS);

    return () => {
      disposed = true;
      window.clearInterval(intervalId);
    };
  }, [activeTopic, loadTopics, selectedAuthorPubkey, selectedThread, trackedTopics]);

  useEffect(() => {
    const remoteObjectUrls = remoteObjectUrlRef.current;
    const draftPreviewUrls = draftPreviewUrlRef.current;
    const directMessageDraftPreviewUrls = directMessageDraftPreviewUrlRef.current;

    return () => {
      for (const url of remoteObjectUrls.values()) {
        URL.revokeObjectURL(url);
      }
      remoteObjectUrls.clear();
      for (const url of draftPreviewUrls.values()) {
        URL.revokeObjectURL(url);
      }
      draftPreviewUrls.clear();
      for (const url of directMessageDraftPreviewUrls.values()) {
        URL.revokeObjectURL(url);
      }
      directMessageDraftPreviewUrls.clear();
    };
  }, []);

  useEffect(() => {
    setGameDrafts((current) => {
      const next = { ...current };
      for (const room of activeGameRooms) {
        if (!next[room.room_id]) {
          next[room.room_id] = createGameEditorDraft(room);
        }
      }
      return next;
    });
  }, [activeGameRooms, setGameDrafts]);

  useEffect(() => {
    if (!selectedPrivateChannelId) {
      return;
    }
    const selectedStillJoined = activeJoinedChannels.some(
      (channel) => channel.channel_id === selectedPrivateChannelId
    );
    if (selectedStillJoined) {
      return;
    }
    setSelectedChannelIdByTopic((current) => ({
      ...current,
      [activeTopic]: null,
    }));
    setComposeChannelByTopic((current) =>
      current[activeTopic]?.kind === 'private_channel' &&
      current[activeTopic].channel_id === selectedPrivateChannelId
        ? {
            ...current,
            [activeTopic]: PUBLIC_CHANNEL_REF,
          }
        : current
    );
    setTimelineScopeByTopic((current) =>
      current[activeTopic]?.kind === 'channel' &&
      current[activeTopic].channel_id === selectedPrivateChannelId
        ? {
            ...current,
            [activeTopic]: PUBLIC_TIMELINE_SCOPE,
          }
        : current
    );
  }, [
    activeJoinedChannels,
    activeTopic,
    selectedPrivateChannelId,
    setComposeChannelByTopic,
    setSelectedChannelIdByTopic,
    setTimelineScopeByTopic,
  ]);

  useEffect(() => {
    let disposed = false;

    for (const attachment of previewableMediaAttachments) {
      if (typeof mediaObjectUrls[attachment.hash] === 'string') {
        continue;
      }

      const nextAttempt = (mediaFetchAttemptRef.current.get(attachment.hash) ?? 0) + 1;
      mediaFetchAttemptRef.current.set(attachment.hash, nextAttempt);
      logMediaDebug('info', 'remote media fetch start', {
        attempt: nextAttempt,
        hash: attachment.hash,
        mime: attachment.mime,
        role: attachment.role,
        status: attachment.status,
      });

      void api
        .getBlobMediaPayload(attachment.hash, attachment.mime)
        .then((payload) => {
          const nextUrl = payload ? createObjectUrlFromPayload(payload) : null;
          if (disposed) {
            if (nextUrl) {
              URL.revokeObjectURL(nextUrl);
            }
            return;
          }
          if (!nextUrl) {
            logMediaDebug('warn', 'remote media fetch missing', {
              attempt: nextAttempt,
              hash: attachment.hash,
              mime: attachment.mime,
              role: attachment.role,
              status: attachment.status,
            });
            return;
          }

          logMediaDebug('info', 'remote media fetch hit', {
            attempt: nextAttempt,
            bytes_base64_length: payload?.bytes_base64.length ?? 0,
            hash: attachment.hash,
            mime: attachment.mime,
            object_url: nextUrl,
            role: attachment.role,
            status: attachment.status,
          });

          setMediaObjectUrls((current) => {
            if (current[attachment.hash] !== undefined) {
              if (nextUrl) {
                URL.revokeObjectURL(nextUrl);
              }
              return current;
            }
            if (nextUrl) {
              remoteObjectUrlRef.current.set(attachment.hash, nextUrl);
            }
            return {
              ...current,
              [attachment.hash]: nextUrl,
            };
          });
        })
        .catch((fetchError: unknown) => {
          if (disposed) {
            return;
          }
          logMediaDebug('warn', 'remote media fetch error', {
            attempt: nextAttempt,
            error: fetchError instanceof Error ? fetchError.message : 'unknown error',
            hash: attachment.hash,
            mime: attachment.mime,
            role: attachment.role,
            status: attachment.status,
          });
        });
    }

    return () => {
      disposed = true;
    };
  }, [api, mediaObjectUrls, previewableMediaAttachments, setMediaObjectUrls]);

  const setNavOpen = useCallback((open: boolean, restoreToTrigger = false) => {
    setShellChromeState((current) => ({
      ...current,
      navOpen: open,
    }));
    if (!open && restoreToTrigger) {
      window.requestAnimationFrame(() => {
        navTriggerRef.current?.focus();
      });
    }
  }, [setShellChromeState]);

  const setSettingsOpen = useCallback((open: boolean, restoreToTrigger = false) => {
    setShellChromeState((current) => ({
      ...current,
      settingsOpen: open,
    }));
    if (!open && restoreToTrigger) {
      window.requestAnimationFrame(() => {
        settingsTriggerRef.current?.focus();
      });
    }
    syncRoute(open ? 'push' : 'replace', {
      settingsOpen: open,
    });
  }, [setShellChromeState, syncRoute]);

  function setPrimarySectionRef(section: PrimarySection) {
    return (element: HTMLElement | null) => {
      primarySectionRefs.current[section] = element;
    };
  }

  function focusPrimarySection(section: PrimarySection) {
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: section,
      profileMode: section === 'profile' ? 'overview' : current.profileMode,
      profileConnectionsView: section === 'profile' ? 'following' : current.profileConnectionsView,
      navOpen: false,
    }));
    setSelectedThread(null);
    setThread([]);
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setAuthorError(null);
    setDirectMessagePaneOpen(section === 'messages');
    setSelectedDirectMessagePeerPubkey(null);
    setDirectMessageError(null);
    window.requestAnimationFrame(() => {
      primarySectionRefs.current[section]?.focus();
    });
    syncRoute('push', {
      primarySection: section,
      profileMode: section === 'profile' ? 'overview' : undefined,
      profileConnectionsView: section === 'profile' ? 'following' : undefined,
      selectedAuthorPubkey: null,
      selectedDirectMessagePeerPubkey: null,
      selectedThread: null,
    });
  }

  function focusTimelineView(view: TimelineWorkspaceView) {
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'timeline',
      timelineView: view,
      navOpen: false,
    }));
    if (view === 'bookmarks') {
      setSelectedThread(null);
      setThread([]);
      setReplyTarget(null);
      setRepostTarget(null);
      setSelectedAuthorPubkey(null);
      setSelectedAuthor(null);
      setSelectedAuthorTimeline([]);
      setAuthorError(null);
      setDirectMessagePaneOpen(false);
      setSelectedDirectMessagePeerPubkey(null);
      setDirectMessageError(null);
    }
    window.requestAnimationFrame(() => {
      primarySectionRefs.current.timeline?.focus();
    });
    syncRoute('push', {
      primarySection: 'timeline',
      timelineView: view,
      selectedAuthorPubkey: view === 'bookmarks' ? null : undefined,
      selectedThread: view === 'bookmarks' ? null : undefined,
      selectedDirectMessagePeerPubkey: view === 'bookmarks' ? null : undefined,
    });
  }

  const closeAuthorPane = useCallback(() => {
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setSelectedAuthorTimeline([]);
    setAuthorError(null);
    syncRoute('replace', {
      selectedAuthorPubkey: null,
    });
  }, [setAuthorError, setSelectedAuthor, setSelectedAuthorTimeline, setSelectedAuthorPubkey, syncRoute]);

  const closeThreadPane = useCallback(() => {
    setSelectedThread(null);
    setThread([]);
    setReplyTarget(null);
    setRepostTarget(null);
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setSelectedAuthorTimeline([]);
    setAuthorError(null);
    syncRoute('replace', {
      selectedThread: null,
      selectedAuthorPubkey: null,
    });
  }, [
    setAuthorError,
    setReplyTarget,
    setRepostTarget,
    setSelectedAuthor,
    setSelectedAuthorTimeline,
    setSelectedAuthorPubkey,
    setSelectedThread,
    setThread,
    syncRoute,
  ]);

  const openDirectMessageList = useCallback((historyMode: 'push' | 'replace' = 'push') => {
    setReplyTarget(null);
    setRepostTarget(null);
    setSelectedThread(null);
    setThread([]);
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setSelectedAuthorTimeline([]);
    setAuthorError(null);
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'messages',
      navOpen: false,
    }));
    setDirectMessagePaneOpen(true);
    setSelectedDirectMessagePeerPubkey(null);
    setDirectMessageError(null);
    syncRoute(historyMode, {
      primarySection: 'messages',
      selectedAuthorPubkey: null,
      selectedDirectMessagePeerPubkey: null,
      selectedThread: null,
    });
  }, [
    setAuthorError,
    setDirectMessageError,
    setDirectMessagePaneOpen,
    setReplyTarget,
    setRepostTarget,
    setShellChromeState,
    setSelectedAuthor,
    setSelectedAuthorPubkey,
    setSelectedAuthorTimeline,
    setSelectedDirectMessagePeerPubkey,
    setSelectedThread,
    setThread,
    syncRoute,
  ]);

  const openDirectMessagePane = useCallback(async (
    peerPubkey: string,
    options?: { historyMode?: 'push' | 'replace'; normalizeOnError?: boolean }
  ) => {
    try {
      const [conversation, timeline, status] = await Promise.all([
        api.openDirectMessage(peerPubkey),
        api.listDirectMessageMessages(peerPubkey, null, 100),
        api.getDirectMessageStatus(peerPubkey),
      ]);
      setReplyTarget(null);
      setRepostTarget(null);
      setSelectedThread(null);
      setThread([]);
      setSelectedAuthorPubkey(null);
      setSelectedAuthor(null);
      setSelectedAuthorTimeline([]);
      setAuthorError(null);
      setDirectMessages((current) => {
        const remaining = current.filter((entry) => entry.peer_pubkey !== conversation.peer_pubkey);
        return [conversation, ...remaining];
      });
      setDirectMessageTimelineByPeer((current) => ({
        ...current,
        [peerPubkey]: timeline.items,
      }));
      setDirectMessageStatusByPeer((current) => ({
        ...current,
        [peerPubkey]: status,
      }));
      setKnownAuthorsByPubkey((current) =>
        mergeKnownAuthors(current, [authorViewFromDirectMessageConversation(conversation)])
      );
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: 'messages',
        navOpen: false,
      }));
      setDirectMessagePaneOpen(true);
      setSelectedDirectMessagePeerPubkey(peerPubkey);
      setDirectMessageError(null);
      syncRoute(options?.historyMode ?? 'push', {
        primarySection: 'messages',
        selectedAuthorPubkey: null,
        selectedDirectMessagePeerPubkey: peerPubkey,
        selectedThread: null,
      });
    } catch (openError) {
      const nextError = messageFromError(openError, 'failed to open direct message');
      setDirectMessageError(nextError);
      if (options?.normalizeOnError) {
        setDirectMessagePaneOpen(true);
        setSelectedDirectMessagePeerPubkey(null);
        syncRoute('replace', {
          primarySection: 'messages',
          selectedDirectMessagePeerPubkey: null,
        });
      }
    }
  }, [
    api,
    setAuthorError,
    setDirectMessageError,
    setDirectMessagePaneOpen,
    setDirectMessages,
    setDirectMessageStatusByPeer,
    setDirectMessageTimelineByPeer,
    setKnownAuthorsByPubkey,
    setReplyTarget,
    setRepostTarget,
    setShellChromeState,
    setSelectedAuthor,
    setSelectedAuthorPubkey,
    setSelectedAuthorTimeline,
    setSelectedDirectMessagePeerPubkey,
    setSelectedThread,
    setThread,
    syncRoute,
  ]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') {
        return;
      }
      if (shellChromeState.settingsOpen) {
        event.preventDefault();
        setSettingsOpen(false, true);
        return;
      }
      if (selectedAuthorPubkey) {
        event.preventDefault();
        closeAuthorPane();
        return;
      }
      if (selectedThread) {
        event.preventDefault();
        closeThreadPane();
        return;
      }
      if (shellChromeState.navOpen) {
        event.preventDefault();
        setNavOpen(false, true);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [
    closeAuthorPane,
    closeThreadPane,
    setNavOpen,
    setSettingsOpen,
    shellChromeState.navOpen,
    shellChromeState.settingsOpen,
    selectedAuthorPubkey,
    selectedThread,
  ]);

  useEffect(() => {
    const shouldFocusSection = didSyncRouteSectionRef.current;
    didSyncRouteSectionRef.current = true;
    setShellChromeState((current) =>
      current.activePrimarySection === routeSection
        ? current
        : {
            ...current,
            activePrimarySection: routeSection,
          }
    );
    if (!shouldFocusSection) {
      return;
    }
    window.requestAnimationFrame(() => {
      primarySectionRefs.current[routeSection]?.focus();
    });
  }, [routeSection, setShellChromeState]);

  function nextDraftId(): string {
    draftSequenceRef.current += 1;
    return `draft-${draftSequenceRef.current}`;
  }

  function rememberDraftPreview(item: DraftMediaItem) {
    draftPreviewUrlRef.current.set(item.id, item.preview_url);
  }

  function releaseDraftPreview(itemId: string) {
    const previewUrl = draftPreviewUrlRef.current.get(itemId);
    if (!previewUrl) {
      return;
    }
    URL.revokeObjectURL(previewUrl);
    draftPreviewUrlRef.current.delete(itemId);
  }

  function releaseAllDraftPreviews() {
    for (const [itemId, previewUrl] of draftPreviewUrlRef.current.entries()) {
      URL.revokeObjectURL(previewUrl);
      draftPreviewUrlRef.current.delete(itemId);
    }
  }

  function rememberDirectMessageDraftPreview(item: DraftMediaItem) {
    directMessageDraftPreviewUrlRef.current.set(item.id, item.preview_url);
  }

  function releaseDirectMessageDraftPreview(itemId: string) {
    const previewUrl = directMessageDraftPreviewUrlRef.current.get(itemId);
    if (!previewUrl) {
      return;
    }
    URL.revokeObjectURL(previewUrl);
    directMessageDraftPreviewUrlRef.current.delete(itemId);
  }

  function releaseAllDirectMessageDraftPreviews() {
    for (const [itemId, previewUrl] of directMessageDraftPreviewUrlRef.current.entries()) {
      URL.revokeObjectURL(previewUrl);
      directMessageDraftPreviewUrlRef.current.delete(itemId);
    }
  }

  async function buildImageDraftItem(file: File): Promise<DraftMediaItem> {
    const attachment = await fileToCreateAttachment(file, 'image_original');
    return {
      id: nextDraftId(),
      source_name: file.name,
      preview_url: URL.createObjectURL(file),
      attachments: [attachment],
    };
  }

  async function buildVideoDraftItem(file: File): Promise<DraftMediaItem> {
    const posterFile = await generateVideoPoster(file);
    return {
      id: nextDraftId(),
      source_name: file.name,
      preview_url: URL.createObjectURL(posterFile),
      attachments: [
        await fileToCreateAttachment(file, 'video_manifest'),
        await blobToCreateAttachment(posterFile, posterFile.name, 'video_poster'),
      ],
    };
  }

  function clearThreadContext() {
    setSelectedThread(null);
    setThread([]);
    setReplyTarget(null);
    setRepostTarget(null);
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setSelectedAuthorTimeline([]);
    setAuthorError(null);
  }

  function handleProfileFieldChange(
    field: 'displayName' | 'name' | 'about',
    value: string
  ) {
    const nextField: keyof ProfileInput = field === 'displayName' ? 'display_name' : field;
    setProfileDraft((current) => ({
      ...current,
      [nextField]: value,
    }));
    setProfileDirty(true);
  }

  async function handleProfileAvatarSelection(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) {
      return;
    }
    const pictureUpload = await fileToCreateAttachment(file, 'profile_avatar');
    const nextPreviewUrl = URL.createObjectURL(file);
    setProfileAvatarPreviewUrl((current) => {
      if (current) {
        URL.revokeObjectURL(current);
      }
      return nextPreviewUrl;
    });
    setProfileAvatarInputKey((value) => value + 1);
    setProfileDraft((current) => ({
      ...current,
      picture: null,
      picture_upload: pictureUpload,
      clear_picture: false,
    }));
    setProfileDirty(true);
    setProfileError(null);
  }

  function handleClearProfileAvatar() {
    setProfileAvatarPreviewUrl((current) => {
      if (current) {
        URL.revokeObjectURL(current);
      }
      return null;
    });
    setProfileAvatarInputKey((value) => value + 1);
    setProfileDraft((current) => ({
      ...current,
      picture: null,
      picture_upload: null,
      clear_picture: true,
    }));
    setProfileDirty(true);
    setProfileError(null);
  }

  function resetProfileDraft() {
    if (!localProfile) {
      return;
    }
    setProfileAvatarPreviewUrl((current) => {
      if (current) {
        URL.revokeObjectURL(current);
      }
      return null;
    });
    setProfileAvatarInputKey((value) => value + 1);
    setProfileDraft(profileInputFromProfile(localProfile));
    setProfileDirty(false);
    setProfileError(null);
    setProfilePanelState({
      status: 'ready',
      error: null,
    });
  }

  const openProfileOverview = useCallback(() => {
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'profile',
      profileMode: 'overview',
    }));
    syncRoute('push', {
      primarySection: 'profile',
      profileMode: 'overview',
    });
  }, [setShellChromeState, syncRoute]);

  const openProfileEditor = useCallback(() => {
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'profile',
      profileMode: 'edit',
    }));
    syncRoute('push', {
      primarySection: 'profile',
      profileMode: 'edit',
    });
  }, [setShellChromeState, syncRoute]);

  const openProfileConnections = useCallback(
    (view: ProfileConnectionsView = 'following') => {
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: 'profile',
        profileMode: 'connections',
        profileConnectionsView: view,
      }));
      syncRoute('push', {
        primarySection: 'profile',
        profileMode: 'connections',
        profileConnectionsView: view,
      });
    },
    [setShellChromeState, syncRoute]
  );

  function handleSelectPrivateChannel(topicId: string, channelId: string) {
    setSelectedChannelIdByTopic((current) => ({
      ...current,
      [topicId]: channelId,
    }));
    setTimelineScopeByTopic((current) => ({
      ...current,
      [topicId]: {
        kind: 'channel',
        channel_id: channelId,
      },
    }));
    setComposeChannelByTopic((current) => ({
      ...current,
      [topicId]: {
        kind: 'private_channel',
        channel_id: channelId,
      },
    }));
    setActiveTopic(topicId);
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'timeline',
      navOpen: false,
    }));
    window.requestAnimationFrame(() => {
      syncRoute('replace', {
        activeTopic: topicId,
        primarySection: 'timeline',
        timelineScope: {
          kind: 'channel',
          channel_id: channelId,
        },
        composeTarget: {
          kind: 'private_channel',
          channel_id: channelId,
        },
      });
    });
  }

  async function handleSaveProfile(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setProfileSaving(true);
    try {
      const profile = await api.setMyProfile(profileDraft);
      setProfileAvatarPreviewUrl((current) => {
        if (current) {
          URL.revokeObjectURL(current);
        }
        return null;
      });
      setProfileAvatarInputKey((value) => value + 1);
      setLocalProfile(profile);
      setProfileDraft(profileInputFromProfile(profile));
      setProfileDirty(false);
      setProfileError(null);
      setProfilePanelState({
        status: 'ready',
        error: null,
      });
      setShellChromeState((current) => ({
        ...current,
        profileMode: 'overview',
      }));
      await loadTopics(trackedTopics, activeTopic, selectedThread);
      syncRoute('replace', {
        primarySection: 'profile',
        profileMode: 'overview',
      });
    } catch (saveError) {
      const nextProfileError = messageFromError(
        saveError,
        translate('common:errors.failedToSaveProfile')
      );
      setProfileError(nextProfileError);
      setProfilePanelState({
        status: 'error',
        error: nextProfileError,
      });
    } finally {
      setProfileSaving(false);
    }
  }

  async function handleAddTopic() {
    const nextTopic = topicInput.trim();
    if (!nextTopic) {
      return;
    }
    const nextTopics = trackedTopics.includes(nextTopic)
      ? trackedTopics
      : [...trackedTopics, nextTopic];
    setTrackedTopics(nextTopics);
    setActiveTopic(nextTopic);
    setTopicInput('');
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'timeline',
      navOpen: false,
    }));
    clearThreadContext();
    syncRoute('replace', {
      activeTopic: nextTopic,
      primarySection: 'timeline',
    });
    await loadTopics(nextTopics, nextTopic, null);
  }

  async function handleSelectTopic(topic: string) {
    setActiveTopic(topic);
    setSelectedChannelIdByTopic((current) => ({
      ...current,
      [topic]: null,
    }));
    setTimelineScopeByTopic((current) => ({
      ...current,
      [topic]: PUBLIC_TIMELINE_SCOPE,
    }));
    setComposeChannelByTopic((current) => ({
      ...current,
      [topic]: PUBLIC_CHANNEL_REF,
    }));
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'timeline',
      navOpen: false,
    }));
    clearThreadContext();
    syncRoute('replace', {
      activeTopic: topic,
      primarySection: 'timeline',
      timelineScope: PUBLIC_TIMELINE_SCOPE,
      composeTarget: PUBLIC_CHANNEL_REF,
    });
    await loadTopics(trackedTopics, topic, null);
  }

  async function handleOpenOriginalTopic(topicId: string) {
    const nextTopics = trackedTopics.includes(topicId) ? trackedTopics : [...trackedTopics, topicId];
    setTrackedTopics(nextTopics);
    setActiveTopic(topicId);
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'timeline',
      navOpen: false,
    }));
    clearThreadContext();
    syncRoute('replace', {
      activeTopic: topicId,
      primarySection: 'timeline',
      timelineScope: privateTimelineScope(selectedChannelIdByTopic[topicId] ?? null),
      composeTarget: privateComposeTarget(selectedChannelIdByTopic[topicId] ?? null),
      selectedAuthorPubkey: null,
      selectedThread: null,
    });
    await loadTopics(nextTopics, topicId, null);
  }

  async function handleRemoveTopic(topic: string) {
    if (trackedTopics.length === 1) {
      return;
    }
    const nextTopics = trackedTopics.filter((value) => value !== topic);
    const nextActiveTopic = activeTopic === topic ? nextTopics[0] : activeTopic;
    await api.unsubscribeTopic(topic);
    setTrackedTopics(nextTopics);
    setActiveTopic(nextActiveTopic);
    setShellChromeState((current) => ({
      ...current,
      navOpen: false,
    }));
    clearThreadContext();
    syncRoute('replace', {
      activeTopic: nextActiveTopic,
    });
    await loadTopics(nextTopics, nextActiveTopic, null);
  }

  async function handleCreatePrivateChannel(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!channelLabelInput.trim()) {
      setChannelError(translate('channels:errors.channelLabelRequired'));
      return;
    }
    setChannelActionPending('create');
    try {
      const channel = await api.createPrivateChannel(
        activeTopic,
        channelLabelInput.trim(),
        channelAudienceInput
      );
      setJoinedChannelsByTopic((current) => ({
        ...current,
        [activeTopic]: upsertJoinedChannel(current[activeTopic] ?? [], channel),
      }));
      setChannelPanelStateByTopic((current) => ({
        ...current,
        [activeTopic]: {
          status: 'ready',
          error: null,
        },
      }));
      setChannelLabelInput('');
      setChannelAudienceInput('invite_only');
      setChannelError(null);
      setTimelineScopeByTopic((current) => ({
        ...current,
        [activeTopic]: {
          kind: 'channel',
          channel_id: channel.channel_id,
        },
      }));
      setSelectedChannelIdByTopic((current) => ({
        ...current,
        [activeTopic]: channel.channel_id,
      }));
      setComposeChannelByTopic((current) => ({
        ...current,
        [activeTopic]: {
          kind: 'private_channel',
          channel_id: channel.channel_id,
        },
      }));
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: 'timeline',
        navOpen: false,
      }));
      syncRoute('replace', {
        activeTopic,
        composeTarget: {
          kind: 'private_channel',
          channel_id: channel.channel_id,
        },
        primarySection: 'timeline',
        timelineScope: {
          kind: 'channel',
          channel_id: channel.channel_id,
        },
      });
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (channelCreateError) {
      setChannelError(
        messageFromError(channelCreateError, translate('channels:errors.failedCreateChannel'))
      );
    } finally {
      setChannelActionPending(null);
    }
  }

  async function handleShareChannelAccess() {
    if (!activePrivateChannel) {
      setChannelError(translate('channels:errors.selectChannelForShare'));
      return;
    }
    setChannelActionPending('share');
    try {
      const access = await api.exportChannelAccessToken(activeTopic, activePrivateChannel.channel_id, null);
      setInviteOutput(access.token);
      setInviteOutputLabel(access.kind);
      setChannelError(null);
    } catch (shareError) {
      setChannelError(
        messageFromError(shareError, translate('channels:errors.failedShareChannel'))
      );
    } finally {
      setChannelActionPending(null);
    }
  }

  async function activateImportedPrivateChannel(
    topicId: string,
    channelId: string,
    placeholderChannel?: JoinedPrivateChannelView
  ) {
    const nextTopics = trackedTopics.includes(topicId) ? trackedTopics : [...trackedTopics, topicId];
    setTrackedTopics(nextTopics);
    setActiveTopic(topicId);
    if (placeholderChannel) {
      setJoinedChannelsByTopic((current) => ({
        ...current,
        [topicId]: upsertJoinedChannel(current[topicId] ?? [], placeholderChannel),
      }));
      setChannelPanelStateByTopic((current) => ({
        ...current,
        [topicId]: {
          status: 'ready',
          error: null,
        },
      }));
    }
    setSelectedChannelIdByTopic((current) => ({
      ...current,
      [topicId]: channelId,
    }));
    setTimelineScopeByTopic((current) => ({
      ...current,
      [topicId]: {
        kind: 'channel',
        channel_id: channelId,
      },
    }));
    setComposeChannelByTopic((current) => ({
      ...current,
      [topicId]: {
        kind: 'private_channel',
        channel_id: channelId,
      },
    }));
    setInviteTokenInput('');
    setInviteOutput(null);
    setChannelError(null);
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'timeline',
      navOpen: false,
    }));
    clearThreadContext();
    syncRoute('replace', {
      activeTopic: topicId,
      composeTarget: {
        kind: 'private_channel',
        channel_id: channelId,
      },
      primarySection: 'timeline',
      timelineScope: {
        kind: 'channel',
        channel_id: channelId,
      },
    });
    await loadTopics(nextTopics, topicId, null);
  }

  async function handleJoinChannelAccess(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!inviteTokenInput.trim()) {
      setChannelError(translate('channels:errors.inviteTokenRequired'));
      return;
    }
    setChannelActionPending('join');
    try {
      const preview = await api.importChannelAccessToken(inviteTokenInput.trim());
      await activateImportedPrivateChannel(
        preview.topic_id,
        preview.channel_id,
        joinedChannelFromAccessTokenPreview(preview)
      );
    } catch (joinError) {
      setChannelError(messageFromError(joinError, translate('channels:errors.failedJoinChannel')));
    } finally {
      setChannelActionPending(null);
    }
  }

  async function handlePublish(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmedComposer = composer.trim();
    const attachments = draftMediaItems.flatMap((item) => item.attachments);
    if (repostTarget) {
      const sourceTopic = publishedTopicIdForPost(repostTarget);
      if (!sourceTopic) {
        setComposerError(translate('common:errors.failedToPublish'));
        return;
      }
      if (!trimmedComposer) {
        setComposerError(translate('common:errors.quoteRepostRequiresCommentary'));
        return;
      }

      try {
        await api.createRepost(activeTopic, sourceTopic, repostTarget.object_id, trimmedComposer);
        releaseAllDraftPreviews();
        setComposer('');
        setDraftMediaItems([]);
        setAttachmentInputKey((value) => value + 1);
        setComposerError(null);
        setReplyTarget(null);
        setRepostTarget(null);
        setComposeDialogOpen(false);
        setSelectedThread(null);
        setThread([]);
        setShellChromeState((current) => ({
          ...current,
          activePrimarySection: 'timeline',
        }));
        await loadTopics(trackedTopics, activeTopic, null);
        syncRoute('replace', {
          primarySection: 'timeline',
          selectedThread: null,
        });
      } catch (publishError) {
        setComposerError(
          publishError instanceof Error
            ? publishError.message
            : translate('common:errors.failedToPublish')
        );
      }
      return;
    }

    if (!trimmedComposer && attachments.length === 0) {
      return;
    }

    try {
      await api.createPost(
        activeTopic,
        trimmedComposer,
        replyTarget?.object_id ?? null,
        attachments,
        activeComposeChannel
      );
      releaseAllDraftPreviews();
      setComposer('');
      setDraftMediaItems([]);
      setAttachmentInputKey((value) => value + 1);
      setComposerError(null);
      setComposeDialogOpen(false);
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: 'timeline',
      }));
      await loadTopics(trackedTopics, activeTopic, selectedThread);
      setReplyTarget(null);
      setRepostTarget(null);
      syncRoute('replace', {
        primarySection: 'timeline',
      });
    } catch (publishError) {
      setComposerError(
        publishError instanceof Error
          ? publishError.message
          : translate('common:errors.failedToPublish')
      );
    }
  }

  async function handleAttachmentSelection(event: ChangeEvent<HTMLInputElement>) {
    const files = Array.from(event.target.files ?? []);
    if (files.length === 0) {
      return;
    }

    const nextItems: DraftMediaItem[] = [];
    const failures: string[] = [];

    for (const file of files) {
      try {
        if (file.type.startsWith('image/')) {
          nextItems.push(await buildImageDraftItem(file));
          continue;
        }
        if (file.type.startsWith('video/')) {
          nextItems.push(await buildVideoDraftItem(file));
          continue;
        }
        failures.push(translate('common:errors.unsupportedAttachmentType', { name: file.name }));
      } catch (attachmentError) {
        failures.push(
          attachmentError instanceof Error
            ? attachmentError.message
            : translate('common:errors.failedToGenerateVideoPoster')
        );
      }
    }

    if (nextItems.length > 0) {
      nextItems.forEach(rememberDraftPreview);
      setDraftMediaItems((current) => [...current, ...nextItems]);
    }

    setComposerError(failures.length > 0 ? failures[0] : null);
    setAttachmentInputKey((value) => value + 1);
  }

  function handleRemoveDraftAttachment(itemId: string) {
    releaseDraftPreview(itemId);
    setDraftMediaItems((current) => current.filter((item) => item.id !== itemId));
  }

  async function handleDirectMessageAttachmentSelection(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) {
      return;
    }

    try {
      const nextItem = file.type.startsWith('image/')
        ? await buildImageDraftItem(file)
        : file.type.startsWith('video/')
          ? await buildVideoDraftItem(file)
          : null;
      if (!nextItem) {
        setDirectMessageError(
          translate('common:errors.unsupportedAttachmentType', { name: file.name })
        );
      } else {
        releaseAllDirectMessageDraftPreviews();
        rememberDirectMessageDraftPreview(nextItem);
        setDirectMessageDraftMediaItems([nextItem]);
        setDirectMessageError(null);
      }
    } catch (attachmentError) {
      setDirectMessageError(
        messageFromError(attachmentError, translate('common:errors.failedToGenerateVideoPoster'))
      );
    } finally {
      setDirectMessageAttachmentInputKey((value) => value + 1);
    }
  }

  function handleRemoveDirectMessageDraftAttachment(itemId: string) {
    releaseDirectMessageDraftPreview(itemId);
    setDirectMessageDraftMediaItems((current) => current.filter((item) => item.id !== itemId));
  }

  async function handleSendDirectMessage(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedDirectMessagePeerPubkey) {
      return;
    }
    const trimmedComposer = directMessageComposer.trim();
    const attachments = directMessageDraftMediaItems.flatMap((item) => item.attachments);
    if (!trimmedComposer && attachments.length === 0) {
      return;
    }
    setDirectMessageSending(true);
    try {
      await api.sendDirectMessage(
        selectedDirectMessagePeerPubkey,
        trimmedComposer || null,
        attachments,
        null
      );
      releaseAllDirectMessageDraftPreviews();
      setDirectMessageComposer('');
      setDirectMessageDraftMediaItems([]);
      setDirectMessageAttachmentInputKey((value) => value + 1);
      setDirectMessageError(null);
      await openDirectMessagePane(selectedDirectMessagePeerPubkey, { historyMode: 'replace' });
    } catch (sendError) {
      setDirectMessageError(messageFromError(sendError, 'failed to send direct message'));
    } finally {
      setDirectMessageSending(false);
    }
  }

  async function handleDeleteDirectMessageMessage(peerPubkey: string, messageId: string) {
    try {
      await api.deleteDirectMessageMessage(peerPubkey, messageId);
      await openDirectMessagePane(peerPubkey, { historyMode: 'replace' });
    } catch (deleteError) {
      setDirectMessageError(messageFromError(deleteError, 'failed to delete direct message'));
    }
  }

  async function handleClearDirectMessage(peerPubkey: string) {
    try {
      await api.clearDirectMessage(peerPubkey);
      await openDirectMessagePane(peerPubkey, { historyMode: 'replace' });
    } catch (clearError) {
      setDirectMessageError(messageFromError(clearError, 'failed to clear direct message'));
    }
  }

  function patchReactionState(reactionState: ReactionStateView) {
    setTimelinesByTopic((current) =>
      Object.fromEntries(
        Object.entries(current).map(([topic, posts]) => [
          topic,
          patchReactionStateIntoPosts(posts, reactionState),
        ])
      )
    );
    setPublicTimelinesByTopic((current) =>
      Object.fromEntries(
        Object.entries(current).map(([topic, posts]) => [
          topic,
          patchReactionStateIntoPosts(posts, reactionState),
        ])
      )
    );
    setThread((current) => patchReactionStateIntoPosts(current, reactionState));
    setProfileTimeline((current) => patchReactionStateIntoPosts(current, reactionState));
    setSelectedAuthorTimeline((current) => patchReactionStateIntoPosts(current, reactionState));
  }

  async function handleToggleReaction(post: PostView, reactionKey: ReactionKeyInput) {
    const topicId = publishedTopicIdForPost(post);
    if (!topicId) {
      setError(translate('common:errors.failedToPublish'));
      return;
    }
    try {
      const nextState = await api.toggleReaction(
        topicId,
        post.object_id,
        reactionKey,
        post.channel_id ? { kind: 'private_channel', channel_id: post.channel_id } : { kind: 'public' }
      );
      patchReactionState(nextState);
      try {
        setRecentReactions(await api.listRecentReactions(8));
      } catch {
        // Keep the toggled state even if the quick-reaction history refresh misses.
      }
      setError(null);
    } catch (reactionError) {
      setError(messageFromError(reactionError, translate('common:errors.failedToPublish')));
    }
  }

  async function handleCreateCustomReactionAsset(
    file: File,
    cropRect: CustomReactionCropRect,
    searchKey: string
  ) {
    setReactionCreatePending(true);
    try {
      const upload = await fileToCreateAttachment(file, 'image_original');
      const asset = await api.createCustomReactionAsset(upload, cropRect, searchKey);
      setOwnedReactionAssets((current) => [asset, ...current.filter((item) => item.asset_id !== asset.asset_id)]);
      setReactionPanelState({
        status: 'ready',
        error: null,
      });
    } catch (reactionError) {
      setReactionPanelState({
        status: 'error',
        error: messageFromError(reactionError, translate('common:errors.failedToPublish')),
      });
    } finally {
      setReactionCreatePending(false);
    }
  }

  async function handleBookmarkCustomReaction(asset: CustomReactionAssetView) {
    try {
      const bookmarked = await api.bookmarkCustomReaction(asset);
      setBookmarkedReactionAssets((current) => [
        bookmarked,
        ...current.filter((item) => item.asset_id !== bookmarked.asset_id),
      ]);
      setReactionPanelState({
        status: 'ready',
        error: null,
      });
    } catch (bookmarkError) {
      setReactionPanelState({
        status: 'error',
        error: messageFromError(bookmarkError, translate('common:errors.failedToPublish')),
      });
    }
  }

  async function handleRemoveBookmarkedCustomReaction(assetId: string) {
    try {
      await api.removeBookmarkedCustomReaction(assetId);
      setBookmarkedReactionAssets((current) => current.filter((item) => item.asset_id !== assetId));
      setReactionPanelState({
        status: 'ready',
        error: null,
      });
    } catch (bookmarkError) {
      setReactionPanelState({
        status: 'error',
        error: messageFromError(bookmarkError, translate('common:errors.failedToPublish')),
      });
    }
  }

  async function handleToggleBookmarkedPost(post: PostView) {
    const topicId = publishedTopicIdForPost(post);
    if (!topicId) {
      setError(translate('common:errors.failedToUpdateBookmark'));
      return;
    }
    try {
      if (bookmarkedPostIds.has(post.object_id)) {
        await api.removeBookmarkedPost(post.object_id);
        setBookmarkedPosts((current) =>
          current.filter((item) => item.post.object_id !== post.object_id)
        );
      } else {
        const bookmarked = await api.bookmarkPost(topicId, post.object_id);
        setBookmarkedPosts((current) => [
          bookmarked,
          ...current.filter((item) => item.post.object_id !== bookmarked.post.object_id),
        ]);
      }
      setError(null);
    } catch (bookmarkError) {
      setError(messageFromError(bookmarkError, translate('common:errors.failedToUpdateBookmark')));
    }
  }

  const openThread = useCallback(async (threadId: string, options?: OpenThreadOptions) => {
    const topic = options?.topic ?? activeTopic;
    try {
      const threadView = await api.listThread(topic, threadId, null, 50);
      if (options?.normalizeOnEmpty && threadView.items.length === 0) {
        startTransition(() => {
          setSelectedThread(null);
          setThread([]);
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
          setDirectMessagePaneOpen(false);
          setSelectedDirectMessagePeerPubkey(null);
          setDirectMessageError(null);
        });
        syncRoute('replace', {
          activeTopic: topic,
          directMessagePaneOpen: false,
          selectedAuthorPubkey: null,
          selectedThread: null,
        });
        return;
      }
      startTransition(() => {
        setActiveTopic(topic);
        setSelectedThread(threadId);
        setThread(threadView.items);
        setSelectedAuthorPubkey(null);
        setSelectedAuthor(null);
        setAuthorError(null);
        setDirectMessagePaneOpen(false);
        setSelectedDirectMessagePeerPubkey(null);
        setDirectMessageError(null);
        setError(null);
      });
      syncRoute(options?.historyMode ?? 'push', {
        activeTopic: topic,
        directMessagePaneOpen: false,
        selectedAuthorPubkey: null,
        selectedThread: threadId,
      });
    } catch (threadError) {
      const nextError =
        threadError instanceof Error
          ? threadError.message
          : translate('common:errors.failedToLoadThread');
      setError(nextError);
      if (options?.normalizeOnEmpty) {
        startTransition(() => {
          setSelectedThread(null);
          setThread([]);
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
          setDirectMessagePaneOpen(false);
          setSelectedDirectMessagePeerPubkey(null);
          setDirectMessageError(null);
        });
        syncRoute('replace', {
          activeTopic: topic,
          directMessagePaneOpen: false,
          selectedAuthorPubkey: null,
          selectedThread: null,
        });
      }
    }
  }, [
    activeTopic,
    api,
    setActiveTopic,
    setAuthorError,
    setDirectMessageError,
    setDirectMessagePaneOpen,
    setError,
    setSelectedAuthor,
    setSelectedAuthorPubkey,
    setSelectedDirectMessagePeerPubkey,
    setSelectedThread,
    setThread,
    syncRoute,
  ]);

  function beginReply(post: PostView) {
    const threadId = post.root_id ?? post.object_id;
    setRepostTarget(null);
    setReplyTarget(post);
    setComposeDialogOpen(true);
    setSelectedThread(threadId);
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setAuthorError(null);
    syncRoute('push', {
      selectedThread: threadId,
      selectedAuthorPubkey: null,
    });
    void openThread(threadId, { historyMode: 'replace' });
  }

  function clearReply() {
    setReplyTarget(null);
    setRepostTarget(null);
  }

  function clearRepost() {
    setRepostTarget(null);
  }

  function openNewPostDialog() {
    clearReply();
    clearRepost();
    setComposeDialogOpen(true);
  }

  function openFloatingActionDialog() {
    if (shellChromeState.activePrimarySection === 'live') {
      setLiveCreateDialogOpen(true);
      return;
    }
    if (shellChromeState.activePrimarySection === 'game') {
      setGameCreateDialogOpen(true);
      return;
    }
    openNewPostDialog();
  }

  async function handleSimpleRepost(post: PostView) {
    const sourceTopic = publishedTopicIdForPost(post);
    if (!sourceTopic || !canCreateRepostFromPost(post)) {
      setComposerError(translate('common:errors.failedToPublish'));
      return;
    }

    try {
      await api.createRepost(activeTopic, sourceTopic, post.object_id, null);
      setComposerError(null);
      setReplyTarget(null);
      setRepostTarget(null);
      setSelectedThread(null);
      setThread([]);
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: 'timeline',
      }));
      await loadTopics(trackedTopics, activeTopic, null);
      syncRoute('replace', {
        primarySection: 'timeline',
        selectedThread: null,
      });
    } catch (repostError) {
      setComposerError(
        repostError instanceof Error
          ? repostError.message
          : translate('common:errors.failedToPublish')
      );
    }
  }

  function beginQuoteRepost(post: PostView) {
    if (!canCreateRepostFromPost(post)) {
      return;
    }
    releaseAllDraftPreviews();
    setDraftMediaItems([]);
    setAttachmentInputKey((value) => value + 1);
    setComposer('');
    setComposerError(null);
    setReplyTarget(null);
    setRepostTarget(post);
    setComposeDialogOpen(true);
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setAuthorError(null);
    syncRoute('replace', {
      selectedAuthorPubkey: null,
    });
  }

  const openAuthorDetail = useCallback(async (
    authorPubkey: string,
    options?: OpenAuthorOptions
  ) => {
    try {
      const socialView = await api.getAuthorSocialView(authorPubkey);
      const nextThreadId = options?.fromThread ? (options.threadId ?? selectedThread) : null;
      setSelectedAuthorPubkey(authorPubkey);
      setSelectedAuthor(socialView);
      setKnownAuthorsByPubkey((current) => mergeKnownAuthors(current, [socialView]));
      setSelectedAuthorTimeline([]);
      setAuthorError(null);
      setDirectMessagePaneOpen(false);
      setSelectedDirectMessagePeerPubkey(null);
      setDirectMessageError(null);
      if (!options?.fromThread) {
        setSelectedThread(null);
        setThread([]);
      }
      syncRoute(options?.historyMode ?? 'push', {
        selectedThread: nextThreadId,
        selectedAuthorPubkey: authorPubkey,
      });
    } catch (detailError) {
      const nextError =
        detailError instanceof Error
          ? detailError.message
          : translate('common:errors.failedToLoadAuthor');
      setAuthorError(nextError);
      if (options?.normalizeOnError) {
        setSelectedAuthorPubkey(null);
        setSelectedAuthor(null);
        setSelectedAuthorTimeline([]);
        if (!options?.fromThread) {
          setSelectedThread(null);
          setThread([]);
        }
        syncRoute('replace', {
          selectedThread: options?.fromThread ? (options.threadId ?? selectedThread) : null,
          selectedAuthorPubkey: null,
        });
      }
    }
  }, [
    api,
    setAuthorError,
    setKnownAuthorsByPubkey,
    setSelectedAuthor,
    setSelectedAuthorTimeline,
    setSelectedAuthorPubkey,
    setDirectMessageError,
    setDirectMessagePaneOpen,
    setSelectedDirectMessagePeerPubkey,
    setSelectedThread,
    setThread,
    selectedThread,
    syncRoute,
  ]);

  async function handleRelationshipAction(authorPubkey: string, following: boolean) {
    try {
      const nextView = following
        ? await api.unfollowAuthor(authorPubkey)
        : await api.followAuthor(authorPubkey);
      setKnownAuthorsByPubkey((current) => mergeKnownAuthors(current, [nextView]));
      if (selectedAuthorPubkey === authorPubkey) {
        setSelectedAuthor(nextView);
        setAuthorError(null);
      }
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (relationshipError) {
      setAuthorError(
        relationshipError instanceof Error
          ? relationshipError.message
          : translate('common:errors.failedToUpdateFollowState')
      );
    }
  }

  async function handleMuteAction(authorPubkey: string, muted: boolean) {
    try {
      const nextView = muted
        ? await api.unmuteAuthor(authorPubkey)
        : await api.muteAuthor(authorPubkey);
      setKnownAuthorsByPubkey((current) => mergeKnownAuthors(current, [nextView]));
      if (selectedAuthorPubkey === authorPubkey) {
        setSelectedAuthor(nextView);
        setAuthorError(null);
      }
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (muteError) {
      setAuthorError(
        muteError instanceof Error
          ? muteError.message
          : translate('common:errors.failedToUpdateMuteState')
      );
    }
  }

  async function handleSaveDiscoverySeeds() {
    try {
      const seedEntries = discoverySeedInput
        .split('\n')
        .map((entry) => entry.trim())
        .filter(Boolean);
      const nextConfig = await api.setDiscoverySeeds(seedEntries);
      setDiscoveryConfig(nextConfig);
      setDiscoverySeedInput(seedPeersToEditorValue(nextConfig));
      setDiscoveryEditorDirty(false);
      setDiscoveryError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
      syncRoute('replace');
    } catch (saveError) {
      setDiscoveryError(
        saveError instanceof Error
          ? saveError.message
          : translate('common:errors.failedToUpdateDiscoverySeeds')
      );
    }
  }

  async function handleSaveCommunityNodes() {
    try {
      const baseUrls = communityNodeInput
        .split('\n')
        .map((entry) => entry.trim())
        .filter(Boolean);
      const nextConfig = await api.setCommunityNodeConfig(baseUrls);
      setCommunityNodeConfig(nextConfig);
      setCommunityNodeInput(communityNodesToEditorValue(nextConfig));
      setCommunityNodeEditorDirty(false);
      setCommunityNodeError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
      syncRoute('replace');
    } catch (saveError) {
      setCommunityNodeError(
        saveError instanceof Error
          ? saveError.message
          : translate('common:errors.failedToUpdateCommunityNodes')
      );
    }
  }

  async function handleClearCommunityNodes() {
    try {
      await api.clearCommunityNodeConfig();
      setCommunityNodeConfig(DEFAULT_COMMUNITY_NODE_CONFIG);
      setCommunityNodeStatuses([]);
      setCommunityNodeInput('');
      setCommunityNodeEditorDirty(false);
      setCommunityNodeError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
      syncRoute('replace');
    } catch (clearError) {
      setCommunityNodeError(
        clearError instanceof Error
          ? clearError.message
          : translate('common:errors.failedToClearCommunityNodes')
      );
    }
  }

  async function handleAuthenticateCommunityNode(baseUrl: string) {
    try {
      const nextStatus = await api.authenticateCommunityNode(baseUrl);
      setCommunityNodeStatuses((current) => upsertCommunityNodeStatus(current, nextStatus));
      setCommunityNodeConfig((current) => syncCommunityNodeConfigWithStatus(current, nextStatus));
      setCommunityNodeError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (authError) {
      setCommunityNodeError(
        authError instanceof Error
          ? authError.message
          : translate('common:errors.failedToAuthenticateCommunityNode')
      );
    }
  }

  async function handleClearCommunityNodeToken(baseUrl: string) {
    try {
      const nextStatus = await api.clearCommunityNodeToken(baseUrl);
      setCommunityNodeStatuses((current) => upsertCommunityNodeStatus(current, nextStatus));
      setCommunityNodeConfig((current) => syncCommunityNodeConfigWithStatus(current, nextStatus));
      setCommunityNodeError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (clearError) {
      setCommunityNodeError(
        clearError instanceof Error
          ? clearError.message
          : translate('common:errors.failedToClearCommunityNodeToken')
      );
    }
  }

  async function handleRefreshCommunityNode(baseUrl: string) {
    try {
      const nextStatus = await api.refreshCommunityNodeMetadata(baseUrl);
      setCommunityNodeStatuses((current) => upsertCommunityNodeStatus(current, nextStatus));
      setCommunityNodeConfig((current) => syncCommunityNodeConfigWithStatus(current, nextStatus));
      setCommunityNodeError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (refreshError) {
      setCommunityNodeError(
        refreshError instanceof Error
          ? refreshError.message
          : translate('common:errors.failedToRefreshCommunityNode')
      );
    }
  }

  async function handleFetchCommunityNodeConsents(baseUrl: string) {
    try {
      const nextStatus = await api.getCommunityNodeConsentStatus(baseUrl);
      setCommunityNodeStatuses((current) => upsertCommunityNodeStatus(current, nextStatus));
      setCommunityNodeConfig((current) => syncCommunityNodeConfigWithStatus(current, nextStatus));
      setCommunityNodeError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (consentError) {
      setCommunityNodeError(
        consentError instanceof Error
          ? consentError.message
          : translate('common:errors.failedToFetchConsentStatus')
      );
    }
  }

  async function handleAcceptCommunityNodeConsents(baseUrl: string) {
    try {
      const nextStatus = await api.acceptCommunityNodeConsents(baseUrl, []);
      setCommunityNodeStatuses((current) => upsertCommunityNodeStatus(current, nextStatus));
      setCommunityNodeConfig((current) => syncCommunityNodeConfigWithStatus(current, nextStatus));
      setCommunityNodeError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (consentError) {
      setCommunityNodeError(
        consentError instanceof Error
          ? consentError.message
          : translate('common:errors.failedToAcceptConsents')
      );
    }
  }

  async function handleImportPeer() {
    if (!peerTicket.trim()) {
      return;
    }
    try {
      await api.importPeerTicket(peerTicket.trim());
      setPeerTicket('');
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (importError) {
      setError(
        importError instanceof Error
          ? importError.message
          : translate('common:errors.failedToImportPeer')
      );
    }
  }

  async function handleCreateLiveSession(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!liveTitle.trim()) {
      setLiveError(translate('live:errors.titleRequired'));
      return;
    }
    setLiveCreatePending(true);
    try {
      await api.createLiveSession(
        activeTopic,
        liveTitle.trim(),
        liveDescription.trim(),
        activeComposeChannel
      );
      setLiveTitle('');
      setLiveDescription('');
      setLiveError(null);
      setLiveCreateDialogOpen(false);
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: 'live',
      }));
      syncRoute('replace', {
        primarySection: 'live',
      });
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (liveCreateError) {
      setLiveError(messageFromError(liveCreateError, translate('live:errors.failedCreate')));
    } finally {
      setLiveCreatePending(false);
    }
  }

  async function handleJoinLiveSession(sessionId: string) {
    setLivePendingBySessionId((current) => ({
      ...current,
      [sessionId]: true,
    }));
    try {
      await api.joinLiveSession(activeTopic, sessionId);
      setLiveError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (joinError) {
      setLiveError(messageFromError(joinError, translate('live:errors.failedJoin')));
    } finally {
      setLivePendingBySessionId((current) => {
        const next = { ...current };
        delete next[sessionId];
        return next;
      });
    }
  }

  async function handleLeaveLiveSession(sessionId: string) {
    setLivePendingBySessionId((current) => ({
      ...current,
      [sessionId]: true,
    }));
    try {
      await api.leaveLiveSession(activeTopic, sessionId);
      setLiveError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (leaveError) {
      setLiveError(messageFromError(leaveError, translate('live:errors.failedLeave')));
    } finally {
      setLivePendingBySessionId((current) => {
        const next = { ...current };
        delete next[sessionId];
        return next;
      });
    }
  }

  async function handleEndLiveSession(sessionId: string) {
    setLivePendingBySessionId((current) => ({
      ...current,
      [sessionId]: true,
    }));
    try {
      await api.endLiveSession(activeTopic, sessionId);
      setLiveError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (endError) {
      setLiveError(messageFromError(endError, translate('live:errors.failedEnd')));
    } finally {
      setLivePendingBySessionId((current) => {
        const next = { ...current };
        delete next[sessionId];
        return next;
      });
    }
  }

  async function handleCreateGameRoom(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const participants = Array.from(
      new Set(
        gameParticipantsInput
          .split(',')
          .map((value) => value.trim())
          .filter((value) => value.length > 0)
      )
    );
    if (!gameTitle.trim()) {
      setGameError(translate('game:errors.titleRequired'));
      return;
    }
    if (participants.length < 2) {
      setGameError(translate('game:errors.participantsRequired'));
      return;
    }
    setGameCreatePending(true);
    try {
      await api.createGameRoom(
        activeTopic,
        gameTitle.trim(),
        gameDescription.trim(),
        participants,
        activeComposeChannel
      );
      setGameTitle('');
      setGameDescription('');
      setGameParticipantsInput('');
      setGameError(null);
      setGameCreateDialogOpen(false);
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: 'game',
      }));
      syncRoute('replace', {
        primarySection: 'game',
      });
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (createError) {
      setGameError(messageFromError(createError, translate('game:errors.failedCreate')));
    } finally {
      setGameCreatePending(false);
    }
  }

  function updateGameDraft(
    roomId: string,
    update: (draft: GameEditorDraft) => GameEditorDraft
  ) {
    setGameDrafts((current) => {
      const existingRoom = activeGameRooms.find((room) => room.room_id === roomId);
      const draft = current[roomId] ?? (existingRoom ? createGameEditorDraft(existingRoom) : null);
      if (!draft) {
        return current;
      }
      return {
        ...current,
        [roomId]: update(draft),
      };
    });
  }

  async function handleUpdateGameRoom(roomId: string) {
    const room = activeGameRooms.find((candidate) => candidate.room_id === roomId);
    if (!room) {
      return;
    }
    const draft = gameDrafts[room.room_id] ?? createGameEditorDraft(room);
    const scores: GameScoreView[] = [];
    for (const score of room.scores) {
      const rawScore = draft.scores[score.participant_id] ?? String(score.score);
      const parsed = Number.parseInt(rawScore, 10);
      if (Number.isNaN(parsed)) {
        setGameError(translate('game:errors.invalidScore', { label: score.label }));
        return;
      }
      scores.push({
        participant_id: score.participant_id,
        label: score.label,
        score: parsed,
      });
    }
    setGameSavingByRoomId((current) => ({
      ...current,
      [room.room_id]: true,
    }));
    try {
      await api.updateGameRoom(
        activeTopic,
        room.room_id,
        draft.status,
        draft.phase_label.trim() || null,
        scores
      );
      setGameError(null);
      setGameDrafts((current) => {
        const next = { ...current };
        delete next[room.room_id];
        return next;
      });
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (updateError) {
      setGameError(messageFromError(updateError, translate('game:errors.failedUpdate')));
    } finally {
      setGameSavingByRoomId((current) => {
        const next = { ...current };
        delete next[room.room_id];
        return next;
      });
    }
  }

  useEffect(() => {
    const currentUrl = `${location.pathname}${location.search}`;
    if (pendingRouteUrlRef.current && pendingRouteUrlRef.current !== currentUrl) {
      return;
    }
    pendingRouteUrlRef.current = null;

    if (!parsePrimarySectionPath(location.pathname)) {
      navigate(`${PRIMARY_SECTION_PATHS.timeline}${location.search}`, { replace: true });
      return;
    }

    const params = new URLSearchParams(location.search);
    const requestedTopic = params.get('topic')?.trim() ?? null;
    const requestedChannelParam = params.get('channel')?.trim() ?? null;
    const requestedTimelineView = params.get('timelineView');
    const requestedTimelineScopeValue = params.get('timelineScope');
    const requestedComposeTargetValue = params.get('composeTarget');
    const requestedSettingsSection = params.get('settings');
    const requestedContext = params.get('context');
    const requestedProfileMode = params.get('profileMode');
    const requestedConnectionsView = params.get('connectionsView');
    const requestedThreadId = params.get('threadId');
    const requestedAuthorPubkey = params.get('authorPubkey');
    const requestedPeerPubkey = params.get('peerPubkey');

    let nextTopic = activeTopic;
    let shouldReload = false;
    let shouldNormalize = false;

    if (requestedTopic) {
      if (trackedTopics.includes(requestedTopic)) {
        if (requestedTopic !== activeTopic) {
          nextTopic = requestedTopic;
          setActiveTopic(requestedTopic);
          shouldReload = true;
        }
      } else {
        shouldNormalize = true;
      }
    } else {
      shouldNormalize = true;
    }

    const nextTimelineView =
      routeSection === 'timeline' && requestedTimelineView === 'bookmarks' ? 'bookmarks' : 'feed';
    const joinedChannelsForTopic = joinedChannelsByTopic[nextTopic] ?? [];
    const currentSelectedChannelIdForTopic = selectedChannelIdByTopic[nextTopic] ?? null;
    let nextSelectedChannelId = currentSelectedChannelIdForTopic;
    if (nextTimelineView !== 'bookmarks') {
      nextSelectedChannelId = requestedChannelParam;
      if (!nextSelectedChannelId) {
        const legacyRequestedChannel = [requestedComposeTargetValue, requestedTimelineScopeValue]
          .filter((value): value is string => Boolean(value))
          .map((value) => {
            if (value.startsWith('channel:')) {
              return value.slice('channel:'.length);
            }
            return null;
          })
          .find((value): value is string => value !== null);
        if (legacyRequestedChannel) {
          nextSelectedChannelId = legacyRequestedChannel;
        }
      }
    } else if (requestedChannelParam) {
      shouldNormalize = true;
    }
    if (requestedTimelineScopeValue || requestedComposeTargetValue) {
      shouldNormalize = true;
    }
    if (
      nextTimelineView !== 'bookmarks' &&
      nextSelectedChannelId &&
      !joinedChannelsForTopic.some((channel) => channel.channel_id === nextSelectedChannelId)
    ) {
      shouldNormalize = true;
      nextSelectedChannelId = null;
    }

    if (currentSelectedChannelIdForTopic !== nextSelectedChannelId) {
      setSelectedChannelIdByTopic((current) => ({
        ...current,
        [nextTopic]: nextSelectedChannelId,
      }));
      setTimelineScopeByTopic((current) => ({
        ...current,
        [nextTopic]: privateTimelineScope(nextSelectedChannelId),
      }));
      setComposeChannelByTopic((current) => ({
        ...current,
        [nextTopic]: privateComposeTarget(nextSelectedChannelId),
      }));
      shouldReload = true;
    }

    if (requestedContext === 'dm' && routeSection !== 'messages') {
      window.requestAnimationFrame(() => {
        syncRoute('replace', {
          activeTopic: nextTopic,
          primarySection: 'messages',
          selectedAuthorPubkey: null,
          selectedDirectMessagePeerPubkey:
            requestedPeerPubkey && isHex64(requestedPeerPubkey) ? requestedPeerPubkey : null,
          selectedThread: null,
        });
      });
      return;
    }

    const nextSettingsOpen = isSettingsSection(requestedSettingsSection);
    const nextSettingsSection = isSettingsSection(requestedSettingsSection)
      ? requestedSettingsSection
      : shellChromeState.activeSettingsSection;
    const nextProfileMode =
      routeSection === 'profile'
        ? requestedProfileMode === 'edit'
          ? 'edit'
          : requestedProfileMode === 'connections'
            ? 'connections'
            : 'overview'
        : 'overview';
    const nextProfileConnectionsView =
      routeSection === 'profile' && requestedProfileMode === 'connections'
        ? isProfileConnectionsView(requestedConnectionsView)
          ? requestedConnectionsView
          : 'following'
        : shellChromeState.profileConnectionsView;

    if (
      shellChromeState.activePrimarySection !== routeSection ||
      shellChromeState.timelineView !== nextTimelineView ||
      shellChromeState.activeSettingsSection !== nextSettingsSection ||
      shellChromeState.settingsOpen !== nextSettingsOpen ||
      shellChromeState.profileMode !== nextProfileMode ||
      shellChromeState.profileConnectionsView !== nextProfileConnectionsView
    ) {
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: routeSection,
        timelineView: nextTimelineView,
        activeSettingsSection: nextSettingsSection,
        settingsOpen: nextSettingsOpen,
        profileMode: nextProfileMode,
        profileConnectionsView: nextProfileConnectionsView,
      }));
    }

    if (requestedTimelineView && requestedTimelineView !== 'bookmarks') {
      shouldNormalize = true;
    }
    if (requestedTimelineView && routeSection !== 'timeline') {
      shouldNormalize = true;
    }
    if (requestedSettingsSection && !isSettingsSection(requestedSettingsSection)) {
      shouldNormalize = true;
    }
    if (
      requestedProfileMode &&
      requestedProfileMode !== 'edit' &&
      requestedProfileMode !== 'connections'
    ) {
      shouldNormalize = true;
    }
    if (requestedProfileMode && routeSection !== 'profile') {
      shouldNormalize = true;
    }
    if (
      requestedConnectionsView &&
      (requestedProfileMode !== 'connections' ||
        !isProfileConnectionsView(requestedConnectionsView))
    ) {
      shouldNormalize = true;
    }
    if (routeSection === 'messages' && requestedContext) {
      shouldNormalize = true;
    }

    if (nextTimelineView === 'bookmarks') {
      if (requestedContext) {
        shouldNormalize = true;
      }
      if (selectedThread) {
        setSelectedThread(null);
        setThread([]);
        setReplyTarget(null);
        setRepostTarget(null);
      }
      if (selectedAuthorPubkey) {
        setSelectedAuthorPubkey(null);
        setSelectedAuthor(null);
        setAuthorError(null);
      }
      if (directMessagePaneOpen) {
        setDirectMessagePaneOpen(false);
      }
      if (selectedDirectMessagePeerPubkey) {
        setSelectedDirectMessagePeerPubkey(null);
      }
      setDirectMessageError(null);
    }
    if (routeSection === 'messages') {
      if (requestedThreadId || requestedAuthorPubkey) {
        shouldNormalize = true;
      }
      if (selectedThread) {
        setSelectedThread(null);
        setThread([]);
        setReplyTarget(null);
        setRepostTarget(null);
      }
      if (selectedAuthorPubkey) {
        setSelectedAuthorPubkey(null);
        setSelectedAuthor(null);
        setAuthorError(null);
      }
      if (!directMessagePaneOpen) {
        setDirectMessagePaneOpen(true);
      }
      if (!requestedPeerPubkey) {
        if (selectedDirectMessagePeerPubkey) {
          setSelectedDirectMessagePeerPubkey(null);
        }
        setDirectMessageError(null);
      } else if (!isHex64(requestedPeerPubkey)) {
        shouldNormalize = true;
        if (selectedDirectMessagePeerPubkey) {
          setSelectedDirectMessagePeerPubkey(null);
        }
      } else if (
        requestedPeerPubkey !== selectedDirectMessagePeerPubkey ||
        !directMessagePaneOpen
      ) {
        void openDirectMessagePane(requestedPeerPubkey, {
          historyMode: 'replace',
          normalizeOnError: true,
        });
      }
    } else if (nextTimelineView !== 'bookmarks' && requestedContext === 'thread') {
      const threadReadyForNestedAuthor =
        requestedThreadId !== null &&
        requestedThreadId.length > 0 &&
        requestedThreadId === selectedThread &&
        thread.length > 0;

      if (!requestedThreadId) {
        shouldNormalize = true;
        if (selectedThread || selectedAuthorPubkey) {
          setSelectedThread(null);
          setThread([]);
          setReplyTarget(null);
          setRepostTarget(null);
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
        }
      } else if (requestedThreadId !== selectedThread || thread.length === 0) {
        void openThread(requestedThreadId, {
          historyMode: 'replace',
          normalizeOnEmpty: true,
          topic: nextTopic,
        });
      }
      if (!requestedAuthorPubkey) {
        if (selectedAuthorPubkey) {
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
        }
      } else if (!isHex64(requestedAuthorPubkey)) {
        shouldNormalize = true;
        if (selectedAuthorPubkey) {
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
        }
      } else if (!threadReadyForNestedAuthor) {
        if (selectedAuthorPubkey) {
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
        }
      } else if (
        requestedAuthorPubkey !== selectedAuthorPubkey ||
        !selectedAuthor ||
        requestedThreadId !== selectedThread
      ) {
        void openAuthorDetail(requestedAuthorPubkey, {
          fromThread: true,
          historyMode: 'replace',
          normalizeOnError: true,
          threadId: requestedThreadId,
        });
      }
    } else if (nextTimelineView !== 'bookmarks' && requestedContext === 'author') {
      if (requestedThreadId) {
        shouldNormalize = true;
      }
      if (selectedThread) {
        setSelectedThread(null);
        setThread([]);
        setReplyTarget(null);
        setRepostTarget(null);
      }
      if (!requestedAuthorPubkey) {
        shouldNormalize = true;
        if (selectedAuthorPubkey) {
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
        }
      } else if (!isHex64(requestedAuthorPubkey)) {
        shouldNormalize = true;
        if (selectedAuthorPubkey) {
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
        }
      } else if (requestedAuthorPubkey !== selectedAuthorPubkey || !selectedAuthor) {
        void openAuthorDetail(requestedAuthorPubkey, {
          historyMode: 'replace',
          normalizeOnError: true,
        });
      }
    } else if (nextTimelineView !== 'bookmarks' && requestedContext) {
      shouldNormalize = true;
      if (selectedThread || selectedAuthorPubkey) {
        setSelectedThread(null);
        setThread([]);
        setReplyTarget(null);
        setRepostTarget(null);
        setSelectedAuthorPubkey(null);
        setSelectedAuthor(null);
        setAuthorError(null);
      }
      if (directMessagePaneOpen || selectedDirectMessagePeerPubkey) {
        setDirectMessagePaneOpen(false);
        setSelectedDirectMessagePeerPubkey(null);
        setDirectMessageError(null);
      }
    } else {
      if (requestedThreadId || requestedAuthorPubkey || requestedPeerPubkey) {
        shouldNormalize = true;
      }
      if (selectedThread || selectedAuthorPubkey || directMessagePaneOpen || selectedDirectMessagePeerPubkey) {
        setSelectedThread(null);
        setThread([]);
        setReplyTarget(null);
        setRepostTarget(null);
        setSelectedAuthorPubkey(null);
        setSelectedAuthor(null);
        setAuthorError(null);
        setDirectMessagePaneOpen(false);
        setSelectedDirectMessagePeerPubkey(null);
        setDirectMessageError(null);
      }
    }

    if (shouldReload) {
      void loadTopics(trackedTopics, nextTopic, requestedContext === 'thread' ? requestedThreadId : null);
    }

    if (shouldNormalize) {
      window.requestAnimationFrame(() => {
        syncRoute('replace');
      });
    }
  }, [
    activeTopic,
    composeChannelByTopic,
    joinedChannelsByTopic,
    loadTopics,
    location.pathname,
    location.search,
    navigate,
    openAuthorDetail,
    openDirectMessagePane,
    openThread,
    routeSection,
    directMessagePaneOpen,
    selectedAuthor,
    selectedAuthorPubkey,
    selectedDirectMessagePeerPubkey,
    selectedChannelIdByTopic,
    selectedThread,
    setAuthorError,
    setDirectMessageError,
    setDirectMessagePaneOpen,
    setSelectedThread,
    setActiveTopic,
    setComposeChannelByTopic,
    setSelectedChannelIdByTopic,
    setSelectedAuthor,
    setSelectedAuthorPubkey,
    setSelectedDirectMessagePeerPubkey,
    setShellChromeState,
    setReplyTarget,
    setRepostTarget,
    setThread,
    setTimelineScopeByTopic,
    shellChromeState.activePrimarySection,
    shellChromeState.timelineView,
    shellChromeState.activeSettingsSection,
    shellChromeState.profileMode,
    shellChromeState.profileConnectionsView,
    shellChromeState.settingsOpen,
    syncRoute,
    thread.length,
    timelineScopeByTopic,
    trackedTopics,
  ]);

  const buildPostCardView = useCallback(
    (post: PostView, context: 'timeline' | 'thread'): PostCardView => {
      const primaryImage = selectPrimaryImage(post);
      const videoPoster = selectVideoPoster(post);
      const videoManifest = selectVideoManifest(post);
      const mediaKind = primaryImage ? 'image' : videoManifest || videoPoster ? 'video' : null;
      const mediaMetaAttachment =
        mediaKind === 'video' ? videoManifest ?? videoPoster : primaryImage;
      const reservedHashes = new Set<string>();
      if (primaryImage) {
        reservedHashes.add(primaryImage.hash);
      }
      if (videoPoster) {
        reservedHashes.add(videoPoster.hash);
      }
      if (videoManifest) {
        reservedHashes.add(videoManifest.hash);
      }
      const extraAttachmentCount = post.attachments.filter(
        (attachment) => !reservedHashes.has(attachment.hash)
      ).length;
      const imagePreviewSrc =
        primaryImage && typeof mediaObjectUrls[primaryImage.hash] === 'string'
          ? mediaObjectUrls[primaryImage.hash]
          : null;
      const videoPosterPreviewSrc =
        videoPoster && typeof mediaObjectUrls[videoPoster.hash] === 'string'
          ? mediaObjectUrls[videoPoster.hash]
          : null;
      const videoPlaybackSrc =
        videoManifest && typeof mediaObjectUrls[videoManifest.hash] === 'string'
          ? mediaObjectUrls[videoManifest.hash]
          : null;
      const videoUnsupportedOnClient = Boolean(
        videoManifest && unsupportedVideoManifests[videoManifest.hash]
      );
      const logPlaybackEvent =
        (eventName: string) => (event: SyntheticEvent<HTMLVideoElement>) => {
          const video = event.currentTarget;
          logMediaDebug(eventName === 'error' ? 'warn' : 'info', `playback ${eventName}`, {
            manifest_hash: videoManifest?.hash ?? null,
            mime: videoManifest?.mime ?? null,
            post_id: post.object_id,
            poster_hash: videoPoster?.hash ?? null,
            playback_src: videoPlaybackSrc,
            ...mediaElementDebugFields(video),
            video_height: video.videoHeight || null,
            video_width: video.videoWidth || null,
          });
          if (eventName === 'error' && videoManifest) {
            setUnsupportedVideoManifests((current) => {
              if (current[videoManifest.hash]) {
                return current;
              }
              return {
                ...current,
                [videoManifest.hash]: true,
              };
            });
          }
        };
      const mediaStatusLabel =
        mediaKind === 'video'
          ? videoUnsupportedOnClient
            ? translate('common:media.unsupportedOnClient')
            : videoPlaybackSrc
              ? translate('common:media.playableVideo')
              : videoPosterPreviewSrc
                ? translate('common:media.posterReady')
                : translate('common:media.syncingPoster')
          : mediaKind === 'image'
            ? imagePreviewSrc
              ? translate('common:media.imageReady')
              : translate('common:media.syncingImage')
            : null;
      const publishedTopicId = publishedTopicIdForPost(post);
      const threadTargetId =
        post.object_kind === 'repost' && !isQuoteRepost(post) && post.repost_of
          ? post.repost_of.root_id ?? post.repost_of.source_object_id
          : post.root_id ?? post.object_id;
      const threadTopicId =
        post.object_kind === 'repost' && !isQuoteRepost(post) && post.repost_of
          ? post.repost_of.source_topic_id
          : publishedTopicId;
      const knownAuthor =
        post.author_pubkey === syncStatus.local_author_pubkey
          ? localProfile
          : knownAuthorsByPubkey[post.author_pubkey] ?? null;

      return {
        post,
        context,
        authorLabel: authorDisplayLabel(
          post.author_pubkey,
          post.author_display_name,
          post.author_name
        ),
        authorPicture:
          post.author_pubkey === syncStatus.local_author_pubkey || knownAuthor
            ? resolveProfilePictureSrc(knownAuthor, mediaObjectUrls)
            : null,
        relationshipLabel: strongestRelationshipLabel(post),
        audienceChipLabel: post.channel_id
          ? activeJoinedChannels.find((channel) => channel.channel_id === post.channel_id)?.label ??
            localizeAudienceLabel(post.audience_label)
          : localizeAudienceLabel(post.audience_label),
        threadTargetId,
        threadTopicId,
        canReply: post.is_threadable ?? (post.object_kind !== 'repost' || isQuoteRepost(post)),
        canRepost: canCreateRepostFromPost(post),
        media: {
          objectId: post.object_id,
          kind: mediaKind,
          statusLabel: mediaStatusLabel,
          extraAttachmentCount,
          state:
            mediaKind === 'video'
              ? videoPlaybackSrc || videoPosterPreviewSrc
                ? 'ready'
                : 'loading'
              : mediaKind === 'image'
                ? imagePreviewSrc
                  ? 'ready'
                  : 'loading'
                : 'loading',
          metaMime: mediaMetaAttachment?.mime ?? null,
          metaBytesLabel: mediaMetaAttachment ? formatBytes(mediaMetaAttachment.bytes, locale) : null,
          imagePreviewSrc,
          videoPosterPreviewSrc,
          videoPlaybackSrc,
          videoUnsupportedOnClient,
          videoProps:
            mediaKind === 'video' && videoPlaybackSrc && !videoUnsupportedOnClient
              ? {
                  onCanPlay: logPlaybackEvent('canplay'),
                  onDurationChange: logPlaybackEvent('durationchange'),
                  onError: logPlaybackEvent('error'),
                  onLoadedData: logPlaybackEvent('loadeddata'),
                  onLoadedMetadata: logPlaybackEvent('loadedmetadata'),
                  onLoadStart: logPlaybackEvent('loadstart'),
                  onPlaying: logPlaybackEvent('playing'),
                }
              : undefined,
        },
      };
    },
    [
      activeJoinedChannels,
      knownAuthorsByPubkey,
      localProfile,
      locale,
      mediaObjectUrls,
      setUnsupportedVideoManifests,
      syncStatus.local_author_pubkey,
      unsupportedVideoManifests,
    ]
  );

  const activeTimelinePostViews = useMemo(
    () => activeTimeline.map((post) => buildPostCardView(post, 'timeline')),
    [activeTimeline, buildPostCardView]
  );
  const bookmarkedPostIds = useMemo(
    () => new Set(bookmarkedPosts.map((item) => item.post.object_id)),
    [bookmarkedPosts]
  );
  const bookmarkedTimelinePostViews = useMemo(
    () => bookmarkedPosts.map((item) => buildPostCardView(item.post, 'timeline')),
    [bookmarkedPosts, buildPostCardView]
  );
  const profileTimelinePostViews = useMemo(
    () => profileTimeline.map((post) => buildPostCardView(post, 'timeline')),
    [buildPostCardView, profileTimeline]
  );
  const selectedAuthorTimelinePostViews = useMemo(
    () => selectedAuthorTimeline.map((post) => buildPostCardView(post, 'timeline')),
    [buildPostCardView, selectedAuthorTimeline]
  );
  const threadPostViews = useMemo(
    () => thread.map((post) => buildPostCardView(post, 'thread')),
    [buildPostCardView, thread]
  );
  const composerSourcePreview = useMemo(
    () =>
      replyTarget
        ? buildPostCardView(replyTarget, 'timeline')
        : repostTarget
          ? buildPostCardView(repostTarget, 'timeline')
          : null,
    [buildPostCardView, replyTarget, repostTarget]
  );
  const topicNavItems = useMemo<TopicDiagnosticSummary[]>(
    () =>
      trackedTopics.map((topic) => ({
        topic,
        active: topic === activeTopic,
        publicActive: topic === activeTopic && (selectedChannelIdByTopic[topic] ?? null) === null,
        removable: trackedTopics.length > 1,
        connectionLabel: topicConnectionLabel(topicDiagnostics[topic]),
        peerCount: topicDiagnostics[topic]?.peer_count ?? 0,
        lastReceivedLabel: formatLastReceivedLabel(topicDiagnostics[topic]?.last_received_at, locale),
        channels:
          topic === activeTopic
            ? (joinedChannelsByTopic[topic] ?? []).map((channel) => ({
                channelId: channel.channel_id,
                label: channel.label,
                audienceKind: channel.audience_kind,
                active: selectedChannelIdByTopic[topic] === channel.channel_id,
              }))
            : [],
      })),
    [activeTopic, joinedChannelsByTopic, locale, selectedChannelIdByTopic, topicDiagnostics, trackedTopics]
  );
  const composerDraftViews = useMemo<ComposerDraftMediaView[]>(
    () =>
      draftMediaItems.map((item) => ({
        id: item.id,
        sourceName: item.source_name,
        previewUrl: item.preview_url,
        attachments: item.attachments.map((attachment) => ({
          key: `${attachment.role ?? attachment.mime}-${attachment.file_name ?? item.source_name}`,
          label: attachment.role ?? translate('common:fallbacks.attachment'),
          mime: attachment.mime,
          byteSizeLabel: formatBytes(attachment.byte_size, locale),
        })),
      })),
    [draftMediaItems, locale]
  );
  const directMessageDraftViews = useMemo<ComposerDraftMediaView[]>(
    () =>
      directMessageDraftMediaItems.map((item) => ({
        id: item.id,
        sourceName: item.source_name,
        previewUrl: item.preview_url,
        attachments: item.attachments.map((attachment) => ({
          key: `${attachment.role ?? attachment.mime}-${attachment.file_name ?? item.source_name}`,
          label: attachment.role ?? translate('common:fallbacks.attachment'),
          mime: attachment.mime,
          byteSizeLabel: formatBytes(attachment.byte_size, locale),
        })),
      })),
    [directMessageDraftMediaItems, locale]
  );
  const threadPanelState = useMemo<ThreadPanelState>(
    () => ({
      selectedThreadId: selectedThread,
      summary: selectedThread
        ? t('shell:context.threadSummary', { count: formatCount(thread.length) })
        : t('shell:context.threadEmpty'),
      emptyCopy: t('shell:context.threadEmpty'),
    }),
    [selectedThread, t, thread.length]
  );
  const resolvedSelectedAuthor = useMemo(
    () =>
      selectedAuthor
        ? knownAuthorsByPubkey[selectedAuthor.author_pubkey] ?? selectedAuthor
        : null,
    [knownAuthorsByPubkey, selectedAuthor]
  );
  const authorDetailView = useMemo<AuthorDetailView>(
    () => ({
      author: resolvedSelectedAuthor,
      displayLabel: resolvedSelectedAuthor
        ? authorDisplayLabel(
            resolvedSelectedAuthor.author_pubkey,
            resolvedSelectedAuthor.display_name,
            resolvedSelectedAuthor.name
          )
        : t('common:fallbacks.authorDetail'),
      pictureSrc: resolveProfilePictureSrc(resolvedSelectedAuthor, mediaObjectUrls),
      summary: resolvedSelectedAuthor
        ? {
            label: strongestRelationshipLabel(resolvedSelectedAuthor),
            following: resolvedSelectedAuthor.following,
            followedBy: resolvedSelectedAuthor.followed_by,
            mutual: resolvedSelectedAuthor.mutual,
            friendOfFriend: resolvedSelectedAuthor.friend_of_friend,
            muted: resolvedSelectedAuthor.muted,
            viaPubkeys: resolvedSelectedAuthor.friend_of_friend_via_pubkeys.map(shortPubkey),
            isSelf: resolvedSelectedAuthor.author_pubkey === syncStatus.local_author_pubkey,
            canFollow: resolvedSelectedAuthor.author_pubkey !== syncStatus.local_author_pubkey,
            followActionLabel: resolvedSelectedAuthor.following ? 'Unfollow' : 'Follow',
            muteActionLabel: resolvedSelectedAuthor.muted ? 'Unmute' : 'Mute',
          }
        : null,
      canMessage: Boolean(
        resolvedSelectedAuthor &&
          resolvedSelectedAuthor.author_pubkey !== syncStatus.local_author_pubkey &&
          resolvedSelectedAuthor.mutual
      ),
      authorError,
    }),
    [authorError, mediaObjectUrls, resolvedSelectedAuthor, syncStatus.local_author_pubkey, t]
  );
  const navRailHeader = (
    <div className='shell-nav-status'>
      <div className='shell-status-badges'>
        <StatusBadge
          label={syncStatusBadgeLabel(syncStatus)}
          tone={syncStatusBadgeTone(syncStatus)}
        />
        <StatusBadge label={`${formatCount(syncStatus.peer_count)} ${t('settings:connectivity.metrics.peers').toLowerCase()}`} />
        <StatusBadge
          label={
            syncStatus.discovery.mode === 'seeded_dht'
              ? t('shell:navigation.seededDht')
              : t('shell:navigation.staticPeers')
          }
        />
        {syncStatus.pending_events > 0 ? (
          <StatusBadge
            label={`${formatCount(syncStatus.pending_events)} ${t('settings:connectivity.metrics.pending').toLowerCase()}`}
            tone='warning'
          />
        ) : null}
      </div>
      <Button
        ref={settingsTriggerRef}
        className='shell-settings-button shell-icon-button'
        variant='ghost'
        size='icon'
        type='button'
        aria-label={
          shellChromeState.settingsOpen
            ? t('shell:settingsDrawer.close')
            : t('shell:settingsDrawer.open')
        }
        aria-controls={SHELL_SETTINGS_ID}
        aria-expanded={shellChromeState.settingsOpen}
        data-testid='shell-settings-trigger'
        onClick={() => setSettingsOpen(!shellChromeState.settingsOpen)}
      >
        <Settings className='size-5' aria-hidden='true' />
      </Button>
    </div>
  );

  const topicList = (
    <TopicNavList
      items={topicNavItems}
      onSelectTopic={(topic) => void handleSelectTopic(topic)}
      onSelectChannel={(topic, channelId) => {
        handleSelectPrivateChannel(topic, channelId);
      }}
      onRemoveTopic={(topic) => void handleRemoveTopic(topic)}
    />
  );
  const channelAction = (
    <div className='shell-nav-channel-actions'>
      <Button
        className='shell-icon-button shell-nav-channel-action'
        variant='secondary'
        size='icon'
        type='button'
        aria-label={t('channels:title')}
        onClick={() => setChannelDialogOpen(true)}
      >
        <Lock className='size-4' aria-hidden='true' />
      </Button>
    </div>
  );

  const connectivityPanelView = useMemo<ConnectivityPanelView>(
    () => ({
      status: 'ready' as const,
      summaryLabel: syncStatusBadgeLabel(syncStatus),
      panelError: error,
      metrics: [
        {
          label: t('settings:connectivity.metrics.connected'),
          value: syncStatus.connected ? t('common:states.yes') : t('common:states.no'),
          tone: syncStatus.connected ? 'accent' : 'warning',
        },
        {
          label: t('settings:connectivity.metrics.peers'),
          value: formatCount(syncStatus.peer_count),
        },
        {
          label: t('settings:connectivity.metrics.pending'),
          value: formatCount(syncStatus.pending_events),
          tone: syncStatus.pending_events > 0 ? 'warning' : 'default',
        },
      ],
      diagnostics: [
        {
          label: t('settings:connectivity.diagnostics.configuredPeers'),
          value: formatListLabel(syncStatus.configured_peers),
          monospace: true,
        },
        {
          label: t('settings:connectivity.diagnostics.connectionDetail'),
          value: syncStatus.status_detail || t('settings:connectivity.summaryDetailFallback'),
        },
        {
          label: t('settings:connectivity.diagnostics.effectivePeers'),
          value: formatListLabel(effectivePeerIds),
          monospace: true,
        },
        {
          label: t('settings:connectivity.diagnostics.lastError'),
          value: syncStatus.last_error ?? t('common:fallbacks.none'),
          tone: syncStatus.last_error ? 'danger' : 'default',
        },
      ],
      localPeerTicket: localPeerTicket ?? '',
      peerTicketInput: peerTicket,
      topics: trackedTopics.map((topic) => {
        const diagnostic = topicDiagnostics[topic];
        return {
          topic,
          summary: t('settings:connectivity.summary', {
            status: translateTopicConnectionText(topicConnectionLabel(diagnostic)),
            count: diagnostic?.peer_count ?? 0,
          }),
          lastReceivedLabel: formatLastReceivedLabel(diagnostic?.last_received_at, locale),
          expectedPeerCount: diagnostic?.configured_peer_ids.length ?? 0,
          missingPeerCount: diagnostic?.missing_peer_ids.length ?? 0,
          statusDetail:
            diagnostic?.status_detail ?? t('settings:connectivity.summaryDetailFallback'),
          connectedPeersLabel: formatListLabel(diagnostic?.connected_peers ?? []),
          relayAssistedPeersLabel: formatListLabel(diagnostic?.assist_peer_ids ?? []),
          configuredPeersLabel: formatListLabel(diagnostic?.configured_peer_ids ?? []),
          missingPeersLabel: formatListLabel(diagnostic?.missing_peer_ids ?? []),
          lastError: diagnostic?.last_error ?? null,
        };
      }),
    }),
    [
      effectivePeerIds,
      error,
      localPeerTicket,
      locale,
      peerTicket,
      syncStatus,
      t,
      topicDiagnostics,
      trackedTopics,
    ]
  );
  const appearancePanelView = useMemo<AppearancePanelView>(
    () => ({
      selectedTheme: theme,
      selectedLocale: locale,
      options: [
        {
          value: 'dark',
          label: t('settings:appearance.themeOptions.dark.label'),
          description: t('settings:appearance.themeOptions.dark.description'),
        },
        {
          value: 'light',
          label: t('settings:appearance.themeOptions.light.label'),
          description: t('settings:appearance.themeOptions.light.description'),
        },
      ],
      localeOptions: [
        {
          value: 'en',
          label: t('settings:appearance.languageOptions.en'),
        },
        {
          value: 'ja',
          label: t('settings:appearance.languageOptions.ja'),
        },
        {
          value: 'zh-CN',
          label: t('settings:appearance.languageOptions.zh-CN'),
        },
      ],
    }),
    [locale, t, theme]
  );
  const discoveryPanelView = useMemo<DiscoveryPanelView>(
    () => ({
      status: 'ready' as const,
      summaryLabel: syncStatus.discovery.mode,
      panelError: null,
      metrics: [
        { label: t('settings:discovery.metrics.mode'), value: syncStatus.discovery.mode },
        {
          label: t('settings:discovery.metrics.connect'),
          value: syncStatus.discovery.connect_mode,
          tone: syncStatus.discovery.connect_mode === 'direct_or_relay' ? 'accent' : 'default',
        },
        {
          label: t('settings:discovery.metrics.envLock'),
          value: discoveryConfig.env_locked ? t('common:states.yes') : t('common:states.no'),
          tone: discoveryConfig.env_locked ? 'warning' : 'default',
        },
      ],
      diagnostics: [
        {
          label: t('settings:discovery.diagnostics.localEndpointId'),
          value: syncStatus.discovery.local_endpoint_id || t('common:fallbacks.unknown'),
          monospace: true,
        },
        {
          label: t('settings:discovery.diagnostics.connectedPeers'),
          value: formatListLabel(syncStatus.discovery.connected_peer_ids),
          monospace: true,
        },
        {
          label: t('settings:discovery.diagnostics.relayAssistedPeers'),
          value: formatListLabel(syncStatus.discovery.assist_peer_ids),
          monospace: true,
        },
        {
          label: t('settings:discovery.diagnostics.manualTicketPeers'),
          value: formatListLabel(syncStatus.discovery.manual_ticket_peer_ids),
          monospace: true,
        },
        {
          label: t('settings:discovery.diagnostics.communityBootstrapPeers'),
          value: formatListLabel(syncStatus.discovery.bootstrap_seed_peer_ids),
          monospace: true,
        },
        {
          label: t('settings:discovery.diagnostics.configuredSeedIds'),
          value: formatListLabel(syncStatus.discovery.configured_seed_peer_ids),
          monospace: true,
        },
        {
          label: t('settings:discovery.diagnostics.discoveryError'),
          value: discoveryError ?? syncStatus.discovery.last_discovery_error ?? t('common:fallbacks.none'),
          tone:
            discoveryError || syncStatus.discovery.last_discovery_error ? 'danger' : 'default',
        },
      ],
      seedPeersInput: discoverySeedInput,
      seedPeersMessage: discoveryConfig.env_locked
        ? t('settings:discovery.messages.viewLocked')
        : discoveryEditorDirty
          ? t('settings:discovery.messages.unsaved')
          : t('settings:discovery.messages.saved'),
      seedPeersMessageTone: discoveryConfig.env_locked ? ('default' as const) : ('default' as const),
      envLocked: discoveryConfig.env_locked,
    }),
    [
      discoveryConfig.env_locked,
      discoveryEditorDirty,
      discoveryError,
      discoverySeedInput,
      syncStatus.discovery.assist_peer_ids,
      syncStatus.discovery.bootstrap_seed_peer_ids,
      syncStatus.discovery.configured_seed_peer_ids,
      syncStatus.discovery.connect_mode,
      syncStatus.discovery.connected_peer_ids,
      syncStatus.discovery.last_discovery_error,
      syncStatus.discovery.local_endpoint_id,
      syncStatus.discovery.manual_ticket_peer_ids,
      syncStatus.discovery.mode,
      t,
    ]
  );
  const communityNodePanelView = useMemo<CommunityNodePanelView>(
    () => ({
      status: 'ready' as const,
      summaryLabel: t('settings:communityNode.summary', { count: communityNodeStatuses.length }),
      panelError: communityNodeError,
      baseUrlsInput: communityNodeInput,
      editorMessage: communityNodeEditorDirty
        ? t('settings:communityNode.editorMessage.unsaved')
        : t('settings:communityNode.editorMessage.saved'),
      editorMessageTone: 'default' as const,
      nodes: communityNodeConfig.nodes.map((node) => {
        const status = communityNodeStatusByBaseUrl[node.base_url];
        return {
          baseUrl: node.base_url,
          diagnostics: [
            {
              label: t('settings:communityNode.diagnostics.auth'),
              value: communityNodeAuthLabel(status),
            },
            {
              label: t('settings:communityNode.diagnostics.consent'),
              value: communityNodeConsentLabel(status),
            },
            {
              label: t('settings:communityNode.diagnostics.connectivityUrls'),
              value: communityNodeConnectivityUrlsLabel(status),
              monospace: true,
            },
            {
              label: t('settings:communityNode.diagnostics.sessionActivation'),
              value: communityNodeSessionActivationLabel(status),
            },
            {
              label: t('settings:communityNode.diagnostics.nextStep'),
              value: communityNodeNextStepLabel(status),
            },
            {
              label: t('settings:communityNode.diagnostics.lastError'),
              value: status?.last_error ?? t('common:fallbacks.none'),
              tone: status?.last_error ? 'danger' : 'default',
            },
          ],
          lastError: status?.last_error ?? null,
        };
      }),
    }),
    [
      communityNodeConfig.nodes,
      communityNodeEditorDirty,
      communityNodeError,
      communityNodeInput,
      communityNodeStatusByBaseUrl,
      communityNodeStatuses.length,
      t,
    ]
  );
  const reactionsPanelView = useMemo<ReactionsPanelView>(
    () => ({
      status: reactionPanelState.status,
      summaryLabel: t('settings:reactions.summary', {
        owned: ownedReactionAssets.length,
        saved: bookmarkedReactionAssets.length,
      }),
      panelError: reactionPanelState.error,
      ownedAssets: ownedReactionAssets,
      bookmarkedAssets: bookmarkedReactionAssets,
    }),
    [
      bookmarkedReactionAssets,
      ownedReactionAssets,
      reactionPanelState.error,
      reactionPanelState.status,
      t,
    ]
  );
  const primarySectionItems = useMemo(
    () =>
      PRIMARY_SECTION_ITEMS.map((item) => ({
        ...item,
        label: t(`shell:primarySections.${item.id}`),
      })),
    [t]
  );
  const timelineViewItems = useMemo<Array<{ id: TimelineWorkspaceView; label: string }>>(
    () => [
      { id: 'feed', label: t('shell:workspace.feed') },
      { id: 'bookmarks', label: t('shell:workspace.bookmarks') },
    ],
    [t]
  );
  const settingsSectionCopy = useMemo(
    () =>
      SETTINGS_SECTION_COPY.map((section) => ({
        ...section,
        label: t(`shell:settingsSections.${section.id}.label`),
        description: t(`shell:settingsSections.${section.id}.description`),
      })),
    [t]
  );

  const settingsSections = [
    {
      ...settingsSectionCopy[0],
      content: (
        <AppearancePanel
          view={appearancePanelView}
          onThemeChange={onThemeChange}
          onLocaleChange={(nextLocale: SupportedLocale) => {
            void i18nInstance.changeLanguage(nextLocale);
          }}
        />
      ),
    },
    {
      ...settingsSectionCopy[1],
      content: (
        <ConnectivityPanel
          view={connectivityPanelView}
          onPeerTicketInputChange={setPeerTicket}
          onImportPeer={() => void handleImportPeer()}
        />
      ),
    },
    {
      ...settingsSectionCopy[2],
      content: (
        <DiscoveryPanel
          view={discoveryPanelView}
          saveDisabled={discoveryConfig.env_locked || !discoveryEditorDirty}
          resetDisabled={!discoveryEditorDirty}
          onSeedPeersChange={(value) => {
            setDiscoverySeedInput(value);
            setDiscoveryEditorDirty(true);
          }}
          onSave={() => void handleSaveDiscoverySeeds()}
          onReset={() => {
            setDiscoverySeedInput(seedPeersToEditorValue(discoveryConfig));
            setDiscoveryEditorDirty(false);
            setDiscoveryError(null);
          }}
        />
      ),
    },
    {
      ...settingsSectionCopy[3],
      content: (
        <CommunityNodePanel
          view={communityNodePanelView}
          saveDisabled={!communityNodeEditorDirty}
          resetDisabled={!communityNodeEditorDirty}
          clearDisabled={communityNodeConfig.nodes.length === 0}
          onBaseUrlsChange={(value) => {
            setCommunityNodeInput(value);
            setCommunityNodeEditorDirty(true);
          }}
          onSaveNodes={() => void handleSaveCommunityNodes()}
          onReset={() => {
            setCommunityNodeInput(communityNodesToEditorValue(communityNodeConfig));
            setCommunityNodeEditorDirty(false);
            setCommunityNodeError(null);
          }}
          onClearNodes={() => void handleClearCommunityNodes()}
          onAuthenticate={(baseUrl) => void handleAuthenticateCommunityNode(baseUrl)}
          onFetchConsents={(baseUrl) => void handleFetchCommunityNodeConsents(baseUrl)}
          onAcceptConsents={(baseUrl) => void handleAcceptCommunityNodeConsents(baseUrl)}
          onRefresh={(baseUrl) => void handleRefreshCommunityNode(baseUrl)}
          onClearToken={(baseUrl) => void handleClearCommunityNodeToken(baseUrl)}
        />
      ),
    },
    {
      ...settingsSectionCopy[4],
      content: (
        <ReactionsPanel
          view={reactionsPanelView}
          creating={reactionCreatePending}
          mediaObjectUrls={mediaObjectUrls}
          onCreateAsset={(file, cropRect, searchKey) =>
            void handleCreateCustomReactionAsset(file, cropRect, searchKey)
          }
          onRemoveBookmark={(assetId) => void handleRemoveBookmarkedCustomReaction(assetId)}
        />
      ),
    },
  ];

  const profileAuthorLabel = authorDisplayLabel(
    syncStatus.local_author_pubkey,
    localProfile?.display_name,
    localProfile?.name
  );
  const messagesWorkspace = (
    <>
      <Card className='shell-workspace-card'>
        <div className='panel-header'>
          <div>
            <h3>Messages</h3>
            <small>{formatCount(directMessages.length)} conversations</small>
          </div>
          {selectedDirectMessagePeerPubkey ? (
            <Button variant='secondary' type='button' onClick={() => openDirectMessageList('replace')}>
              All
            </Button>
          ) : null}
        </div>
        {directMessageError ? <Notice tone='destructive'>{directMessageError}</Notice> : null}
        {directMessages.length === 0 ? (
          <p className='empty'>No direct messages yet.</p>
        ) : (
          <ul className='post-list'>
            {directMessages.map((conversation) => {
              const label = authorDisplayLabel(
                conversation.peer_pubkey,
                conversation.peer_display_name,
                conversation.peer_name
              );
              const selected = conversation.peer_pubkey === selectedDirectMessagePeerPubkey;
              return (
                <li key={conversation.peer_pubkey}>
                  <article className='post-card'>
                    <div className='post-meta'>
                      <span>{label}</span>
                      <span>
                        {conversation.last_message_at
                          ? formatLocalizedTime(conversation.last_message_at, locale)
                          : t('common:fallbacks.noEvents')}
                      </span>
                    </div>
                    <div className='post-body'>
                      <strong className='post-title'>
                        {conversation.last_message_preview ?? t('common:fallbacks.none')}
                      </strong>
                    </div>
                    <div className='post-actions'>
                      <Button
                        variant={selected ? 'primary' : 'secondary'}
                        type='button'
                        onClick={() => void openDirectMessagePane(conversation.peer_pubkey)}
                      >
                        Open
                      </Button>
                    </div>
                  </article>
                </li>
              );
            })}
          </ul>
        )}
      </Card>

      {selectedDirectMessagePeerPubkey ? (
        <>
          <Card className='shell-workspace-card'>
            <div className='shell-workspace-header'>
              <div className='shell-workspace-summary'>
                <span className='relationship-badge'>
                  {selectedDirectMessageConversation
                    ? authorDisplayLabel(
                        selectedDirectMessageConversation.peer_pubkey,
                        selectedDirectMessageConversation.peer_display_name,
                        selectedDirectMessageConversation.peer_name
                      )
                    : selectedDirectMessagePeerPubkey}
                </span>
                {selectedDirectMessageStatus ? (
                  <span className='relationship-badge relationship-badge-direct'>
                    {selectedDirectMessageStatus.send_enabled
                      ? `peers ${formatCount(selectedDirectMessageStatus.peer_count)}`
                      : 'send disabled'}
                  </span>
                ) : null}
              </div>
              <div className='post-actions'>
                <Button
                  variant='secondary'
                  type='button'
                  onClick={() =>
                    void openDirectMessagePane(selectedDirectMessagePeerPubkey, {
                      historyMode: 'replace',
                    })
                  }
                >
                  {t('common:actions.refresh')}
                </Button>
                <Button
                  variant='secondary'
                  type='button'
                  disabled={selectedDirectMessageTimeline.length === 0}
                  onClick={() => void handleClearDirectMessage(selectedDirectMessagePeerPubkey)}
                >
                  {t('common:actions.clear')}
                </Button>
              </div>
            </div>
          </Card>

          <Card className='shell-workspace-card'>
            {selectedDirectMessageTimeline.length === 0 ? (
              <p className='empty'>No messages yet.</p>
            ) : (
              <ul className='post-list'>
                {selectedDirectMessageTimeline.map((message) => {
                  const image = selectPrimaryImageAttachment(message.attachments);
                  const poster = selectVideoPosterAttachment(message.attachments);
                  const video = selectVideoManifestAttachment(message.attachments);
                  const imageSrc = image ? mediaObjectUrls[image.hash] ?? null : null;
                  const posterSrc = poster ? mediaObjectUrls[poster.hash] ?? null : null;
                  const videoSrc = video ? mediaObjectUrls[video.hash] ?? null : null;
                  const videoUnsupported = Boolean(video && unsupportedVideoManifests[video.hash]);
                  return (
                    <li key={message.message_id}>
                      <article className='post-card'>
                        <div className='post-meta'>
                          <span>{message.outgoing ? 'You' : 'Peer'}</span>
                          <span>{formatLocalizedTime(message.created_at, locale)}</span>
                          <span className='reply-chip'>
                            {message.delivered ? 'Delivered' : 'Pending'}
                          </span>
                        </div>
                        {message.text ? (
                          <div className='post-body'>
                            <strong className='post-title'>{message.text}</strong>
                          </div>
                        ) : null}
                        {image ? (
                          imageSrc ? (
                            <div className='draft-preview-frame'>
                              <img
                                className='draft-preview-image'
                                src={imageSrc}
                                alt={t('common:media.imageAlt')}
                              />
                            </div>
                          ) : (
                            <small>{t('common:media.syncingImage')}</small>
                          )
                        ) : null}
                        {video ? (
                          videoSrc && !videoUnsupported ? (
                            <video
                              className='post-card-video'
                              controls
                              playsInline
                              poster={posterSrc ?? undefined}
                              src={videoSrc}
                            />
                          ) : posterSrc ? (
                            <div className='draft-preview-frame'>
                              <img
                                className='draft-preview-image'
                                src={posterSrc}
                                alt={t('common:media.videoPosterAlt')}
                              />
                            </div>
                          ) : (
                            <small>{t('common:media.syncingPoster')}</small>
                          )
                        ) : null}
                        <div className='post-actions'>
                          <Button
                            variant='secondary'
                            type='button'
                            onClick={() =>
                              void handleDeleteDirectMessageMessage(
                                selectedDirectMessagePeerPubkey,
                                message.message_id
                              )
                            }
                          >
                            {t('common:actions.clear')}
                          </Button>
                        </div>
                      </article>
                    </li>
                  );
                })}
              </ul>
            )}
          </Card>

          <Card className='shell-workspace-card'>
            {selectedDirectMessageStatus && !selectedDirectMessageStatus.send_enabled ? (
              <Notice tone='warning'>
                Direct message send is disabled until the relationship is mutual again.
              </Notice>
            ) : null}
            <form className='composer' onSubmit={(event) => void handleSendDirectMessage(event)}>
              <Textarea
                value={directMessageComposer}
                onChange={(event) => setDirectMessageComposer(event.target.value)}
                placeholder='Write a message'
                disabled={directMessageSending || !selectedDirectMessageStatus?.send_enabled}
              />
              <Label className='file-field file-field-compact'>
                <span>{t('common:fallbacks.attachment')}</span>
                <Input
                  key={directMessageAttachmentInputKey}
                  aria-label={t('common:fallbacks.attachment')}
                  type='file'
                  accept='image/*,video/*'
                  disabled={directMessageSending || !selectedDirectMessageStatus?.send_enabled}
                  onChange={(event) => {
                    void handleDirectMessageAttachmentSelection(event);
                  }}
                />
              </Label>
              <ComposerDraftPreviewList
                items={directMessageDraftViews}
                onRemove={handleRemoveDirectMessageDraftAttachment}
              />
              <div className='topic-diagnostic topic-diagnostic-secondary'>
                <span>
                  pending outbox {formatCount(selectedDirectMessageStatus?.pending_outbox_count ?? 0)}
                </span>
              </div>
              <Button
                type='submit'
                disabled={directMessageSending || !selectedDirectMessageStatus?.send_enabled}
              >
                {directMessageSending ? 'Sending...' : 'Send'}
              </Button>
            </form>
          </Card>
        </>
      ) : null}
    </>
  );
  const detailPaneStack = (
    <>
      {selectedThread ? (
        <ContextPane
          paneId={`${SHELL_CONTEXT_ID}-thread`}
          title={t('shell:context.thread')}
          summary={threadPanelState.summary}
          showBackdrop={!selectedAuthorPubkey}
          stackIndex={0}
          onClose={closeThreadPane}
        >
          <ThreadPanel
            state={threadPanelState}
            posts={threadPostViews}
            onOpenAuthor={(authorPubkey) =>
              void openAuthorDetail(authorPubkey, {
                fromThread: true,
                threadId: selectedThread,
              })
            }
            onOpenThread={(threadId) => void openThread(threadId)}
            onOpenThreadInTopic={(threadId, topicId) => void openThread(threadId, { topic: topicId })}
            onReply={beginReply}
            onRepost={(post) => void handleSimpleRepost(post)}
            onQuoteRepost={beginQuoteRepost}
            localAuthorPubkey={syncStatus.local_author_pubkey}
            mediaObjectUrls={mediaObjectUrls}
            ownedReactionAssets={ownedReactionAssets}
            bookmarkedReactionAssets={bookmarkedReactionAssets}
            recentReactions={recentReactions}
            onToggleReaction={(post, reactionKey) => void handleToggleReaction(post, reactionKey)}
            onBookmarkCustomReaction={(asset) => void handleBookmarkCustomReaction(asset)}
          />
        </ContextPane>
      ) : null}
      {selectedAuthorPubkey ? (
        <ContextPane
          paneId={`${SHELL_CONTEXT_ID}-author`}
          title={t('shell:context.author')}
          summary={
            selectedAuthor
              ? authorDetailView.displayLabel
              : t('common:fallbacks.selectAuthor')
          }
          showBackdrop={true}
          stackIndex={selectedThread ? 1 : 0}
          onClose={closeAuthorPane}
        >
          <div className='shell-main-stack'>
            <AuthorDetailCard
              view={authorDetailView}
              localAuthorPubkey={syncStatus.local_author_pubkey}
              onToggleRelationship={(authorPubkey, following) =>
                void handleRelationshipAction(authorPubkey, following)
              }
              onToggleMute={(authorPubkey, muted) => void handleMuteAction(authorPubkey, muted)}
              onOpenDirectMessage={(authorPubkey) => void openDirectMessagePane(authorPubkey)}
            />
            <Card className='shell-workspace-card'>
              <TimelineFeed
                posts={selectedAuthorTimelinePostViews}
                emptyCopy={t('profile:feed.noAuthorPosts')}
                onOpenAuthor={(authorPubkey) => void openAuthorDetail(authorPubkey)}
                onOpenThread={(threadId) => void openThread(threadId)}
                onOpenThreadInTopic={(threadId, topicId) => void openThread(threadId, { topic: topicId })}
                onReply={beginReply}
                readOnly={true}
                onOpenOriginalTopic={(topicId) => void handleOpenOriginalTopic(topicId)}
              />
            </Card>
          </div>
        </ContextPane>
      ) : null}
    </>
  );

  return (
    <>
      <ShellFrame
        skipTargetId={SHELL_WORKSPACE_ID}
        topBar={<ShellTopBar activeTopic={activeTopic} />}
        navRail={
          <ShellNavRail
            railId={SHELL_NAV_ID}
            open={shellChromeState.navOpen}
            onOpenChange={(open) => setNavOpen(open, !open)}
            headerContent={navRailHeader}
            addTopicControl={
              <Label>
                <span>{t('shell:navigation.addTopic')}</span>
                <div className='topic-input-row'>
                  <Input
                    value={topicInput}
                    onChange={(event) => setTopicInput(event.target.value)}
                    placeholder={t('shell:navigation.placeholder')}
                  />
                  <Button variant='secondary' onClick={() => void handleAddTopic()}>
                    {t('common:actions.add')}
                  </Button>
                </div>
              </Label>
            }
            channelAction={channelAction}
            channelSummary={
              activePrivateChannel
                ? `${activePrivateChannel.label} · ${translateAudienceKindLabel(activePrivateChannel.audience_kind)}`
                : t('common:audience.public')
            }
            topicList={topicList}
            topicCount={syncStatus.subscribed_topics.length}
          />
        }
        workspace={
          <div className='shell-main-stack'>
            <Card className='shell-workspace-card shell-workspace-header-card'>
              <TimelineWorkspaceHeader
                activeSection={shellChromeState.activePrimarySection}
                items={primarySectionItems}
                onSelectSection={focusPrimarySection}
              />
            </Card>

            <section
              className='shell-section'
              ref={setPrimarySectionRef(shellChromeState.activePrimarySection)}
              tabIndex={-1}
              onFocusCapture={() =>
                setShellChromeState((current) => ({
                  ...current,
                  activePrimarySection: routeSection,
                }))
              }
            >
              {shellChromeState.activePrimarySection === 'timeline' ? (
                <>
                  <Card className='shell-workspace-card'>
                    <div className='shell-workspace-header'>
                      <div className='shell-workspace-summary'>
                        <div className='shell-workspace-tabs' role='tablist' aria-label={t('shell:workspace.timelineViews')}>
                          {timelineViewItems.map((item) => (
                            <button
                              key={item.id}
                              className={`shell-tab${
                                shellChromeState.timelineView === item.id ? ' shell-tab-active' : ''
                              }`}
                              role='tab'
                              type='button'
                              aria-selected={shellChromeState.timelineView === item.id}
                              onClick={() => focusTimelineView(item.id)}
                            >
                              {item.label}
                            </button>
                          ))}
                        </div>
                        {shellChromeState.timelineView === 'feed' ? (
                          <>
                            <span className='relationship-badge'>
                              {t('common:audience.viewing', {
                                audience: audienceLabelForTimelineScope(
                                  activeTimelineScope,
                                  activeJoinedChannels
                                ),
                              })}
                            </span>
                            <span className='relationship-badge relationship-badge-direct'>
                              {t('common:audience.posting', {
                                audience: activeComposeAudienceLabel,
                              })}
                            </span>
                          </>
                        ) : (
                          <span className='relationship-badge'>
                            {t('shell:workspace.savedCount', {
                              count: bookmarkedTimelinePostViews.length,
                            })}
                          </span>
                        )}
                      </div>
                      <Button
                        variant='secondary'
                        type='button'
                        onClick={() => void loadTopics(trackedTopics, activeTopic, selectedThread)}
                      >
                        {t('common:actions.refresh')}
                      </Button>
                    </div>
                    {composerError ? <Notice tone='destructive'>{composerError}</Notice> : null}
                  </Card>
                  <Card className='shell-workspace-card'>
                    {shellChromeState.timelineView === 'feed' ? (
                      <TimelineFeed
                        posts={activeTimelinePostViews}
                        emptyCopy={t('shell:workspace.noPosts')}
                        onOpenAuthor={(authorPubkey) => void openAuthorDetail(authorPubkey)}
                        onOpenThread={(threadId) => void openThread(threadId)}
                        onOpenThreadInTopic={(threadId, topicId) => void openThread(threadId, { topic: topicId })}
                        onReply={beginReply}
                        onRepost={(post) => void handleSimpleRepost(post)}
                        onQuoteRepost={beginQuoteRepost}
                        localAuthorPubkey={syncStatus.local_author_pubkey}
                        mediaObjectUrls={mediaObjectUrls}
                        ownedReactionAssets={ownedReactionAssets}
                        bookmarkedReactionAssets={bookmarkedReactionAssets}
                        recentReactions={recentReactions}
                        onToggleReaction={(post, reactionKey) => void handleToggleReaction(post, reactionKey)}
                        onBookmarkCustomReaction={(asset) => void handleBookmarkCustomReaction(asset)}
                        showBookmarkAction={true}
                        bookmarkedPostIds={bookmarkedPostIds}
                        onToggleBookmark={(post) => void handleToggleBookmarkedPost(post)}
                      />
                    ) : (
                      <TimelineFeed
                        posts={bookmarkedTimelinePostViews}
                        emptyCopy={t('shell:workspace.noBookmarks')}
                        onOpenAuthor={(authorPubkey) => void openAuthorDetail(authorPubkey)}
                        onOpenThread={(threadId) => void openThread(threadId)}
                        onOpenThreadInTopic={(threadId, topicId) => void openThread(threadId, { topic: topicId })}
                        onReply={beginReply}
                        onRepost={(post) => void handleSimpleRepost(post)}
                        onQuoteRepost={beginQuoteRepost}
                        localAuthorPubkey={syncStatus.local_author_pubkey}
                        mediaObjectUrls={mediaObjectUrls}
                        ownedReactionAssets={ownedReactionAssets}
                        bookmarkedReactionAssets={bookmarkedReactionAssets}
                        recentReactions={recentReactions}
                        onToggleReaction={(post, reactionKey) => void handleToggleReaction(post, reactionKey)}
                        onBookmarkCustomReaction={(asset) => void handleBookmarkCustomReaction(asset)}
                        showBookmarkAction={true}
                        bookmarkedPostIds={bookmarkedPostIds}
                        onToggleBookmark={(post) => void handleToggleBookmarkedPost(post)}
                      />
                    )}
                  </Card>
                </>
              ) : null}

              {shellChromeState.activePrimarySection === 'live' ? (
                <>
                  <Card className='shell-workspace-card'>
                    <div className='panel-header'>
                      <div>
                        <h3>{t('live:title')}</h3>
                        <small>{t('live:summary', { count: liveSessionListItems.length })}</small>
                      </div>
                    </div>
                    {activeLivePanelState.status === 'loading' ? (
                      <Notice>{t('live:loading')}</Notice>
                    ) : null}
                    {activeLivePanelState.status === 'error' &&
                    (liveError ?? activeLivePanelState.error) ? (
                      <Notice tone='destructive'>{liveError ?? activeLivePanelState.error}</Notice>
                    ) : null}
                  </Card>
                  <Card className='shell-workspace-card'>
                    {liveSessionListItems.length === 0 && activeLivePanelState.status === 'ready' ? (
                      <p className='empty-state'>{t('live:empty')}</p>
                    ) : null}
                    <ul className='post-list'>
                      {liveSessionListItems.map(({ session, isOwner, pending }) => (
                        <li key={session.session_id}>
                          <article className='post-card' aria-busy={pending}>
                            <div className='post-meta'>
                              <span>{session.title}</span>
                              <span>{translateLiveStatus(session.status)}</span>
                              <span className='reply-chip'>{localizeAudienceLabel(session.audience_label)}</span>
                            </div>
                            <div className='post-body'>
                              <strong className='post-title'>
                                {session.description || t('common:fallbacks.noDescription')}
                              </strong>
                            </div>
                            <small>{session.session_id}</small>
                            <div className='topic-diagnostic topic-diagnostic-secondary'>
                              <span>{t('common:labels.viewers')}: {formatCount(session.viewer_count)}</span>
                              <span>
                                {t('common:labels.started')}: {formatLocalizedTime(session.started_at)}
                              </span>
                            </div>
                            {session.ended_at ? (
                              <div className='topic-diagnostic topic-diagnostic-secondary'>
                                <span>
                                  {t('common:labels.ended')}: {formatLocalizedTime(session.ended_at)}
                                </span>
                              </div>
                            ) : null}
                            <div className='post-actions'>
                              {session.joined_by_me ? (
                                <Button
                                  variant='secondary'
                                  type='button'
                                  disabled={pending}
                                  onClick={() => void handleLeaveLiveSession(session.session_id)}
                                >
                                  {t('common:actions.leave')}
                                </Button>
                              ) : (
                                <Button
                                  variant='secondary'
                                  type='button'
                                  disabled={pending || session.status === 'Ended'}
                                  onClick={() => void handleJoinLiveSession(session.session_id)}
                                >
                                  {t('common:actions.join')}
                                </Button>
                              )}
                              {isOwner ? (
                                <Button
                                  variant='secondary'
                                  type='button'
                                  disabled={pending || session.status === 'Ended'}
                                  onClick={() => void handleEndLiveSession(session.session_id)}
                                >
                                  {t('common:actions.end')}
                                </Button>
                              ) : null}
                            </div>
                          </article>
                        </li>
                      ))}
                    </ul>
                  </Card>
                </>
              ) : null}

              {shellChromeState.activePrimarySection === 'game' ? (
                <>
                  <Card className='shell-workspace-card'>
                    <div className='panel-header'>
                      <div>
                        <h3>{t('game:title')}</h3>
                        <small>{t('game:summary', { count: activeGameRooms.length })}</small>
                      </div>
                    </div>
                    {activeGamePanelState.status === 'loading' ? (
                      <Notice>{t('game:loading')}</Notice>
                    ) : null}
                    {activeGamePanelState.status === 'error' &&
                    (gameError ?? activeGamePanelState.error) ? (
                      <Notice tone='destructive'>{gameError ?? activeGamePanelState.error}</Notice>
                    ) : null}
                  </Card>
                  <Card className='shell-workspace-card'>
                    {activeGameRooms.length === 0 && activeGamePanelState.status === 'ready' ? (
                      <p className='empty-state'>{t('game:empty')}</p>
                    ) : null}
                    <ul className='post-list'>
                      {activeGameRooms.map((room) => {
                        const draft = gameDraftViews[room.room_id];
                        const isOwner = room.host_pubkey === syncStatus.local_author_pubkey;
                        const pending = Boolean(gameSavingByRoomId[room.room_id]);

                        return (
                          <li key={room.room_id}>
                            <article className='post-card' aria-busy={pending}>
                              <div className='post-meta'>
                                <span>{room.title}</span>
                                <span>{translateGameStatus(room.status)}</span>
                                <span className='reply-chip'>{localizeAudienceLabel(room.audience_label)}</span>
                              </div>
                              <div className='post-body'>
                                <strong className='post-title'>
                                  {room.description || t('common:fallbacks.noDescription')}
                                </strong>
                              </div>
                              <small>{room.room_id}</small>
                              <div className='topic-diagnostic topic-diagnostic-secondary'>
                                <span>{t('common:labels.phase')}: {room.phase_label ?? t('common:fallbacks.none')}</span>
                                <span>
                                  {t('common:labels.updated')}: {formatLocalizedTime(room.updated_at)}
                                </span>
                              </div>
                              <ul className='draft-attachment-list'>
                                {room.scores.map((score) => (
                                  <li
                                    key={score.participant_id}
                                    className='draft-attachment-item score-row'
                                  >
                                    <div className='draft-attachment-content'>
                                      <strong>{score.label}</strong>
                                    </div>
                                    {isOwner ? (
                                      <Input
                                        aria-label={`${room.room_id}-${score.label}-score`}
                                        value={
                                          draft?.scores[score.participant_id] ?? String(score.score)
                                        }
                                        disabled={pending}
                                        onChange={(event) =>
                                          updateGameDraft(room.room_id, (current) => ({
                                            ...current,
                                            scores: {
                                              ...current.scores,
                                              [score.participant_id]: event.target.value,
                                            },
                                          }))
                                        }
                                      />
                                    ) : (
                                      <span>{score.score}</span>
                                    )}
                                  </li>
                                ))}
                              </ul>
                              {isOwner && draft ? (
                                <div className='composer composer-compact'>
                                  <Label>
                                    <span>{t('game:fields.status')}</span>
                                    <Select
                                      aria-label={`${room.room_id}-status`}
                                      value={draft.status}
                                      disabled={pending}
                                      onChange={(event) =>
                                        updateGameDraft(room.room_id, (current) => ({
                                          ...current,
                                          status: event.target.value as GameRoomStatus,
                                        }))
                                      }
                                    >
                                      <option value='Waiting'>{t('game:statuses.Waiting')}</option>
                                      <option value='Running'>{t('game:statuses.Running')}</option>
                                      <option value='Paused'>{t('game:statuses.Paused')}</option>
                                      <option value='Ended'>{t('game:statuses.Ended')}</option>
                                    </Select>
                                  </Label>
                                  <Label>
                                    <span>{t('game:fields.phase')}</span>
                                    <Input
                                      aria-label={`${room.room_id}-phase`}
                                      value={draft.phaseLabel}
                                      disabled={pending}
                                      onChange={(event) =>
                                        updateGameDraft(room.room_id, (current) => ({
                                          ...current,
                                          phase_label: event.target.value,
                                        }))
                                      }
                                    />
                                  </Label>
                                  <Button
                                    variant='secondary'
                                    type='button'
                                    disabled={pending}
                                    onClick={() => void handleUpdateGameRoom(room.room_id)}
                                  >
                                    {t('game:actions.saveRoom')}
                                  </Button>
                                </div>
                              ) : null}
                            </article>
                          </li>
                        );
                      })}
                    </ul>
                  </Card>
                </>
              ) : null}

              {shellChromeState.activePrimarySection === 'messages' ? messagesWorkspace : null}

              {shellChromeState.activePrimarySection === 'profile' ? (
                <>
                  {profileMode === 'edit' ? (
                    <ProfileEditorPanel
                      authorLabel={profileAuthorLabel}
                      status={profilePanelState.status}
                      saving={profileSaving}
                      dirty={profileDirty}
                      error={profileError ?? profilePanelState.error}
                      fields={profileEditorFields}
                      picturePreviewSrc={profileEditorPictureSrc}
                      hasPicture={profileEditorHasPicture}
                      pictureInputKey={profileAvatarInputKey}
                      onFieldChange={handleProfileFieldChange}
                      onPictureSelect={(event) => {
                        void handleProfileAvatarSelection(event);
                      }}
                      onPictureClear={handleClearProfileAvatar}
                      onBack={openProfileOverview}
                      onSave={handleSaveProfile}
                      onReset={resetProfileDraft}
                    />
                  ) : profileMode === 'connections' ? (
                    <ProfileConnectionsPanel
                      activeView={profileConnectionsView}
                      items={activeSocialConnectionViews}
                      localAuthorPubkey={syncStatus.local_author_pubkey}
                      status={socialConnectionsPanelState.status}
                      error={socialConnectionsPanelState.error}
                      onSelectView={openProfileConnections}
                      onToggleRelationship={(authorPubkey, following) =>
                        void handleRelationshipAction(authorPubkey, following)
                      }
                      onToggleMute={(authorPubkey, muted) =>
                        void handleMuteAction(authorPubkey, muted)
                      }
                      onBack={openProfileOverview}
                    />
                  ) : (
                    <ProfileOverviewPanel
                      authorLabel={profileAuthorLabel}
                      about={localProfile?.about ?? null}
                      picture={resolveProfilePictureSrc(localProfile, mediaObjectUrls)}
                      status={profilePanelState.status}
                      error={profileError ?? profilePanelState.error}
                      postCount={profileTimelinePostViews.length}
                      followingCount={socialConnections.following.length}
                      followedCount={socialConnections.followed.length}
                      mutedCount={socialConnections.muted.length}
                      onEdit={openProfileEditor}
                      onOpenFollowing={() => openProfileConnections('following')}
                      onOpenFollowed={() => openProfileConnections('followed')}
                      onOpenMuted={() => openProfileConnections('muted')}
                    />
                  )}
                  {profileMode !== 'connections' ? (
                    <Card className='shell-workspace-card'>
                      <TimelineFeed
                        posts={profileTimelinePostViews}
                        emptyCopy={t('profile:feed.noOwnPosts')}
                        onOpenAuthor={(authorPubkey) => void openAuthorDetail(authorPubkey)}
                        onOpenThread={(threadId) => void openThread(threadId)}
                        onOpenThreadInTopic={(threadId, topicId) => void openThread(threadId, { topic: topicId })}
                        onReply={beginReply}
                        readOnly={true}
                        onOpenOriginalTopic={(topicId) => void handleOpenOriginalTopic(topicId)}
                      />
                    </Card>
                  ) : null}
                </>
              ) : null}
            </section>
          </div>
        }
        detailPaneStack={detailPaneStack}
        detailPaneCount={(selectedThread ? 1 : 0) + (selectedAuthorPubkey ? 1 : 0)}
        mobileFooter={
          <Button
            ref={navTriggerRef}
            variant='secondary'
            type='button'
            aria-label={
              shellChromeState.navOpen
                ? t('shell:navigation.close')
                : t('shell:navigation.open')
            }
            aria-controls={SHELL_NAV_ID}
            aria-expanded={shellChromeState.navOpen}
            data-testid='shell-nav-trigger'
            onClick={() => setNavOpen(!shellChromeState.navOpen)}
          >
            <PanelLeftOpen className='size-5' aria-hidden='true' />
            {t('shell:navigation.topicsButton')}
          </Button>
        }
      />

      <Dialog open={channelDialogOpen} onOpenChange={setChannelDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('channels:title')}</DialogTitle>
            <DialogDescription>{activeTopic}</DialogDescription>
          </DialogHeader>
          <DialogBody>
            <PrivateChannelPanel
              status={activeChannelPanelState.status}
              error={channelError ?? activeChannelPanelState.error}
              pendingAction={channelActionPending}
              channelLabel={channelLabelInput}
              channelAudience={channelAudienceInput}
              channelAudienceOptions={channelAudienceOptions}
              inviteTokenInput={inviteTokenInput}
              inviteOutput={inviteOutput}
              inviteOutputLabel={inviteOutputLabel}
              channels={privateChannelListItems}
              selectedChannel={activePrivateChannel}
              onChannelLabelChange={setChannelLabelInput}
              onChannelAudienceChange={setChannelAudienceInput}
              onInviteTokenChange={setInviteTokenInput}
              onCreateChannel={(event) => void handleCreatePrivateChannel(event)}
              onJoin={(event) => void handleJoinChannelAccess(event)}
              onSelectChannel={(channelId) => handleSelectPrivateChannel(activeTopic, channelId)}
              onShare={() => void handleShareChannelAccess()}
            />
          </DialogBody>
        </DialogContent>
      </Dialog>

      <Dialog open={composeDialogOpen} onOpenChange={setComposeDialogOpen}>
        <DialogContent className='shell-compose-dialog'>
          <DialogHeader>
            <DialogTitle>
              {replyTarget
                ? t('common:actions.reply')
                : repostTarget
                  ? t('common:actions.quoteRepost')
                  : t('common:actions.publish')}
            </DialogTitle>
            <DialogDescription>
              {t('common:labels.audience')}: {activeComposeAudienceLabel}
            </DialogDescription>
          </DialogHeader>
          <DialogBody>
            <ComposerPanel
              value={composer}
              onChange={(event) => setComposer(event.target.value)}
              onSubmit={handlePublish}
              attachmentInputKey={attachmentInputKey}
              onAttachmentSelection={(event) => {
                void handleAttachmentSelection(event);
              }}
              draftMediaItems={composerDraftViews}
              onRemoveDraftAttachment={handleRemoveDraftAttachment}
              composerError={composerError}
              audienceLabel={activeComposeAudienceLabel}
              sourcePreview={composerSourcePreview}
              replyTarget={
                replyTarget
                  ? {
                      content: replyTarget.content,
                      audienceLabel: replyTarget.audience_label,
                    }
                  : null
              }
              repostTarget={
                repostTarget
                  ? {
                      content: repostTarget.content,
                      authorLabel: authorDisplayLabel(
                        repostTarget.author_pubkey,
                        repostTarget.author_display_name,
                        repostTarget.author_name
                      ),
                    }
                  : null
              }
              onClearReply={clearReply}
              onClearRepost={clearRepost}
              attachmentsDisabled={Boolean(repostTarget)}
            />
          </DialogBody>
        </DialogContent>
      </Dialog>

      <Dialog open={liveCreateDialogOpen} onOpenChange={setLiveCreateDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('live:actions.start')}</DialogTitle>
            <DialogDescription>
              {t('common:labels.audience')}: {activeComposeAudienceLabel}
            </DialogDescription>
          </DialogHeader>
          <DialogBody>
            <form
              className='composer composer-compact'
              onSubmit={handleCreateLiveSession}
              aria-busy={liveCreatePending}
            >
              <Label>
                <span>{t('live:fields.title')}</span>
                <Input
                  value={liveTitle}
                  onChange={(event) => setLiveTitle(event.target.value)}
                  placeholder={t('live:fields.placeholders.title')}
                  disabled={liveCreatePending}
                />
              </Label>
              <Label>
                <span>{t('live:fields.description')}</span>
                <Textarea
                  value={liveDescription}
                  onChange={(event) => setLiveDescription(event.target.value)}
                  placeholder={t('live:fields.placeholders.description')}
                  disabled={liveCreatePending}
                />
              </Label>
              {liveError ? <p className='error error-inline'>{liveError}</p> : null}
              <Button type='submit' disabled={liveCreatePending}>
                {t('live:actions.start')}
              </Button>
            </form>
          </DialogBody>
        </DialogContent>
      </Dialog>

      <Dialog open={gameCreateDialogOpen} onOpenChange={setGameCreateDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('game:actions.createRoom')}</DialogTitle>
            <DialogDescription>
              {t('common:labels.audience')}: {activeComposeAudienceLabel}
            </DialogDescription>
          </DialogHeader>
          <DialogBody>
            <form
              className='composer composer-compact'
              onSubmit={handleCreateGameRoom}
              aria-busy={gameCreatePending}
            >
              <Label>
                <span>{t('game:fields.title')}</span>
                <Input
                  value={gameTitle}
                  onChange={(event) => setGameTitle(event.target.value)}
                  placeholder={t('game:fields.placeholders.title')}
                  disabled={gameCreatePending}
                />
              </Label>
              <Label>
                <span>{t('game:fields.description')}</span>
                <Textarea
                  value={gameDescription}
                  onChange={(event) => setGameDescription(event.target.value)}
                  placeholder={t('game:fields.placeholders.description')}
                  disabled={gameCreatePending}
                />
              </Label>
              <Label>
                <span>{t('game:fields.participants')}</span>
                <Input
                  value={gameParticipantsInput}
                  onChange={(event) => setGameParticipantsInput(event.target.value)}
                  placeholder={t('game:fields.placeholders.participants')}
                  disabled={gameCreatePending}
                />
              </Label>
              {gameError ? <p className='error error-inline'>{gameError}</p> : null}
              <Button type='submit' disabled={gameCreatePending}>
                {t('game:actions.createRoom')}
              </Button>
            </form>
          </DialogBody>
        </DialogContent>
      </Dialog>

      {showFloatingActionButton ? (
        <Button
          className='shell-fab'
          variant='primary'
          size='icon'
          type='button'
          data-testid='shell-fab'
          aria-label={floatingActionLabel}
          onClick={openFloatingActionDialog}
        >
          <Plus className='size-5' aria-hidden='true' />
        </Button>
      ) : null}

      <SettingsDrawer
        drawerId={SHELL_SETTINGS_ID}
        open={shellChromeState.settingsOpen}
        onOpenChange={(open) => setSettingsOpen(open, !open)}
        activeSection={shellChromeState.activeSettingsSection}
        onSectionChange={(section) =>
          {
            setShellChromeState((current) => ({
              ...current,
              activeSettingsSection: section,
            }));
            syncRoute('replace', {
              settingsOpen: true,
              settingsSection: section,
            });
          }
        }
        sections={settingsSections}
      />
    </>
  );
}

export function App(props: AppProps) {
  const [store] = useState<DesktopShellStoreApi>(() => createDesktopShellStore());
  const [theme, setTheme] = useState<DesktopTheme>(() => readDesktopTheme());

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  useEffect(() => {
    writeDesktopTheme(theme);
  }, [theme]);

  return (
    <DesktopShellStoreContext.Provider value={store}>
      <HashRouter>
        <DesktopShellPage {...props} theme={theme} onThemeChange={setTheme} />
      </HashRouter>
    </DesktopShellStoreContext.Provider>
  );
}
