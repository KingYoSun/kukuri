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
import { PanelLeftOpen, Settings } from 'lucide-react';

import { AuthorDetailCard } from '@/components/core/AuthorDetailCard';
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
import { AppearancePanel } from '@/components/settings/AppearancePanel';
import { CommunityNodePanel } from '@/components/settings/CommunityNodePanel';
import { ConnectivityPanel } from '@/components/settings/ConnectivityPanel';
import { DiscoveryPanel } from '@/components/settings/DiscoveryPanel';
import {
  type AppearancePanelView,
  type CommunityNodePanelView,
  type ConnectivityPanelView,
  type DiscoveryPanelView,
} from '@/components/settings/types';
import {
  type ExtendedPanelStatus,
  type GameDraftView,
  type InviteOutputLabel,
  type PrivateChannelPendingAction,
} from '@/components/extended/types';
import { ContextPane } from '@/components/shell/ContextPane';
import { ShellFrame } from '@/components/shell/ShellFrame';
import { ShellNavRail } from '@/components/shell/ShellNavRail';
import { SettingsDrawer } from '@/components/shell/SettingsDrawer';
import { ShellTopBar } from '@/components/shell/ShellTopBar';
import {
  type PrimarySection,
  type ProfileWorkspaceMode,
  type SettingsSection,
  type ShellChromeState,
} from '@/components/shell/types';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { Select } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';

import {
  AuthorSocialView,
  AttachmentView,
  BlobMediaPayload,
  ChannelAudienceKind,
  ChannelRef,
  CommunityNodeConfig,
  CommunityNodeNodeStatus,
  CreateAttachmentInput,
  DesktopApi,
  DiscoveryConfig,
  FriendOnlyGrantPreview,
  FriendPlusSharePreview,
  GameRoomStatus,
  GameRoomView,
  GameScoreView,
  JoinedPrivateChannelView,
  LiveSessionView,
  PostView,
  Profile,
  ProfileInput,
  PrivateChannelInvitePreview,
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
  profileDraft: ProfileInput;
  profileDirty: boolean;
  profileError: string | null;
  profilePanelState: AsyncPanelState;
  profileSaving: boolean;
  selectedAuthorPubkey: string | null;
  selectedAuthor: AuthorSocialView | null;
  authorError: string | null;
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
  selectedAuthorPubkey?: string | null;
  selectedThread?: string | null;
  settingsOpen?: boolean;
  settingsSection?: SettingsSection;
  timelineScope?: TimelineScope;
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
    profileDraft: {},
    profileDirty: false,
    profileError: null,
    profilePanelState: DEFAULT_ASYNC_PANEL_STATE,
    profileSaving: false,
    selectedAuthorPubkey: null,
    selectedAuthor: null,
    authorError: null,
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
    error: null,
    shellChromeState: {
      activePrimarySection: 'timeline',
      activeSettingsSection: 'connectivity',
      profileMode: 'overview',
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
    id: 'channels',
    label: 'Channels',
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
];

function isSettingsSection(value: string | null): value is SettingsSection {
  return (
    value === 'appearance' ||
    value === 'connectivity' ||
    value === 'discovery' ||
    value === 'community-node'
  );
}

const PRIMARY_SECTION_PATHS: Record<PrimarySection, string> = {
  timeline: '/timeline',
  channels: '/channels',
  live: '/live',
  game: '/game',
  profile: '/profile',
};

function parsePrimarySectionPath(pathname: string): PrimarySection | null {
  const normalizedPath = pathname === '/' ? '/timeline' : pathname;
  const match = (
    Object.entries(PRIMARY_SECTION_PATHS) as Array<[PrimarySection, string]>
  ).find(([, path]) => path === normalizedPath);
  return match?.[0] ?? null;
}

function translate(key: string, options?: Record<string, unknown>): string {
  return i18n.t(key, options) as string;
}

function selectPrimaryImage(post: PostView): AttachmentView | null {
  return post.attachments.find((attachment) => attachment.role === 'image_original') ?? null;
}

function selectVideoPoster(post: PostView): AttachmentView | null {
  return post.attachments.find((attachment) => attachment.role === 'video_poster') ?? null;
}

function selectVideoManifest(post: PostView): AttachmentView | null {
  return (
    post.attachments.find(
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
  };
}

function authorDisplayLabel(
  authorPubkey: string,
  displayName?: string | null,
  name?: string | null
): string {
  return displayName?.trim() || name?.trim() || shortPubkey(authorPubkey);
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

function inviteOutputSummaryLabel(label: InviteOutputLabel): string {
  if (label === 'grant') {
    return translate('channels:latestGrant');
  }
  if (label === 'share') {
    return translate('channels:latestShare');
  }
  return translate('channels:latestInvite');
}

function channelPolicyDescription(audienceKind: JoinedPrivateChannelView['audience_kind']) {
  if (audienceKind === 'friend_only') {
    return translate('channels:policies.friend_only');
  }
  if (audienceKind === 'friend_plus') {
    return translate('channels:policies.friend_plus');
  }
  return translate('channels:policies.invite_only');
}

function channelRefValue(channelRef: ChannelRef): string {
  return channelRef.kind === 'public' ? 'public' : `channel:${channelRef.channel_id}`;
}

function channelRefFromValue(value: string): ChannelRef {
  if (value.startsWith('channel:')) {
    return {
      kind: 'private_channel',
      channel_id: value.slice('channel:'.length),
    };
  }
  return PUBLIC_CHANNEL_REF;
}

function timelineScopeValue(scope: TimelineScope): string {
  if (scope.kind === 'channel') {
    return `channel:${scope.channel_id}`;
  }
  return scope.kind;
}

function timelineScopeFromValue(value: string): TimelineScope {
  if (value.startsWith('channel:')) {
    return {
      kind: 'channel',
      channel_id: value.slice('channel:'.length),
    };
  }
  if (value === 'all_joined') {
    return { kind: 'all_joined' };
  }
  return PUBLIC_TIMELINE_SCOPE;
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

function translateBooleanLabel(value: boolean): string {
  return value ? translate('common:states.yes') : translate('common:states.no');
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

function joinedChannelFromInvitePreview(
  preview: PrivateChannelInvitePreview
): JoinedPrivateChannelView {
  return {
    topic_id: preview.topic_id,
    channel_id: preview.channel_id,
    label: preview.channel_label,
    creator_pubkey: preview.inviter_pubkey,
    owner_pubkey: preview.inviter_pubkey,
    joined_via_pubkey: null,
    audience_kind: 'invite_only',
    is_owner: false,
    current_epoch_id: 'legacy',
    archived_epoch_ids: [],
    sharing_state: 'open',
    rotation_required: false,
    participant_count: 0,
    stale_participant_count: 0,
  };
}

function joinedChannelFromFriendGrantPreview(
  preview: FriendOnlyGrantPreview
): JoinedPrivateChannelView {
  return {
    topic_id: preview.topic_id,
    channel_id: preview.channel_id,
    label: preview.channel_label,
    creator_pubkey: preview.owner_pubkey,
    owner_pubkey: preview.owner_pubkey,
    joined_via_pubkey: null,
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

function joinedChannelFromFriendSharePreview(
  preview: FriendPlusSharePreview
): JoinedPrivateChannelView {
  return {
    topic_id: preview.topic_id,
    channel_id: preview.channel_id,
    label: preview.channel_label,
    creator_pubkey: preview.owner_pubkey,
    owner_pubkey: preview.owner_pubkey,
    joined_via_pubkey: preview.sponsor_pubkey,
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
    profileDraft,
    profileDirty,
    profileError,
    profilePanelState,
    profileSaving,
    selectedAuthorPubkey,
    selectedAuthor,
    authorError,
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
    error,
    shellChromeState,
    setField,
  } = useDesktopShellStore();

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

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
  const setAuthorError = useMemo(() => makeFieldSetter('authorError'), [makeFieldSetter]);
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
  const setError = useMemo(() => makeFieldSetter('error'), [makeFieldSetter]);
  const setShellChromeState = useMemo(
    () => makeFieldSetter('shellChromeState'),
    [makeFieldSetter]
  );
  const draftSequenceRef = useRef(0);
  const mediaFetchAttemptRef = useRef(new Map<string, number>());
  const remoteObjectUrlRef = useRef(new Map<string, string>());
  const draftPreviewUrlRef = useRef(new Map<string, string>());
  const loadTopicsRequestRef = useRef(0);
  const pendingRouteUrlRef = useRef<string | null>(null);
  const didSyncRouteSectionRef = useRef(false);
  const navTriggerRef = useRef<HTMLButtonElement | null>(null);
  const settingsTriggerRef = useRef<HTMLButtonElement | null>(null);
  const primarySectionRefs = useRef<Record<PrimarySection, HTMLElement | null>>({
    timeline: null,
    channels: null,
    live: null,
    game: null,
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
  const activeTimelineScope = useMemo(
    () => timelineScopeByTopic[activeTopic] ?? PUBLIC_TIMELINE_SCOPE,
    [activeTopic, timelineScopeByTopic]
  );
  const activeComposeChannel = useMemo(() => {
    if (replyTarget?.channel_id) {
      return {
        kind: 'private_channel',
        channel_id: replyTarget.channel_id,
      } as ChannelRef;
    }
    return composeChannelByTopic[activeTopic] ?? PUBLIC_CHANNEL_REF;
  }, [activeTopic, composeChannelByTopic, replyTarget]);
  const activeComposeAudienceLabel = useMemo(() => {
    if (replyTarget) {
      return replyTarget.audience_label;
    }
    return audienceLabelForChannelRef(activeComposeChannel, activeJoinedChannels);
  }, [activeComposeChannel, activeJoinedChannels, replyTarget]);
  const profileMode = shellChromeState.profileMode;
  const selectedPrivateChannelId = useMemo(
    () => selectedChannelIdByTopic[activeTopic] ?? null,
    [activeTopic, selectedChannelIdByTopic]
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
  const syncRoute = useCallback((
    mode: 'push' | 'replace' = 'replace',
    overrides?: DesktopShellRouteOverrides
  ) => {
    const search = new URLSearchParams();
    const nextTopic = overrides?.activeTopic ?? activeTopic;
    const nextTimelineScope = overrides?.timelineScope ?? activeTimelineScope;
    const nextComposeTarget = overrides?.composeTarget ?? activeComposeChannel;
    const nextPrimarySection = overrides?.primarySection ?? shellChromeState.activePrimarySection;
    const nextProfileMode = overrides?.profileMode ?? shellChromeState.profileMode;
    const nextSelectedThread = overrides?.selectedThread ?? selectedThread;
    const nextSelectedAuthorPubkey =
      overrides?.selectedAuthorPubkey ?? selectedAuthorPubkey;
    const nextSettingsOpen = overrides?.settingsOpen ?? shellChromeState.settingsOpen;
    const nextSettingsSection =
      overrides?.settingsSection ?? shellChromeState.activeSettingsSection;

    search.set('topic', nextTopic);
    const nextTimelineScopeValue = timelineScopeValue(nextTimelineScope);
    const nextComposeTargetValue = channelRefValue(nextComposeTarget);
    if (nextTimelineScopeValue !== 'public') {
      search.set('timelineScope', nextTimelineScopeValue);
    }
    if (nextComposeTargetValue !== 'public') {
      search.set('composeTarget', nextComposeTargetValue);
    }
    if (nextSelectedThread) {
      search.set('context', 'thread');
      search.set('threadId', nextSelectedThread);
      if (nextSelectedAuthorPubkey) {
        search.set('authorPubkey', nextSelectedAuthorPubkey);
      }
    } else if (nextSelectedAuthorPubkey) {
      search.set('context', 'author');
      search.set('authorPubkey', nextSelectedAuthorPubkey);
    }
    if (nextPrimarySection === 'profile' && nextProfileMode === 'edit') {
      search.set('profileMode', 'edit');
    }
    if (nextSettingsOpen) {
      search.set('settings', nextSettingsSection);
    }

    const nextPath = PRIMARY_SECTION_PATHS[nextPrimarySection];
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
    activeComposeChannel,
    activeTimelineScope,
    activeTopic,
    location.pathname,
    location.search,
    navigate,
    selectedAuthorPubkey,
    selectedThread,
    shellChromeState.activePrimarySection,
    shellChromeState.activeSettingsSection,
    shellChromeState.profileMode,
    shellChromeState.settingsOpen,
  ]);
  const privateChannelListItems = useMemo(
    () =>
      activeJoinedChannels.map((channel) => ({
        channel,
        active: channel.channel_id === selectedPrivateChannelId,
      })),
    [activeJoinedChannels, selectedPrivateChannelId]
  );
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
      picture: profileDraft.picture ?? '',
    }),
    [profileDraft]
  );
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
    for (const post of [...activeTimeline, ...activePublicTimeline, ...thread]) {
      for (const attachment of [
        selectPrimaryImage(post),
        selectVideoPoster(post),
        selectVideoManifest(post),
      ]) {
        if (attachment) {
          attachments.set(attachment.hash, attachment);
        }
      }
    }
    return [...attachments.values()];
  }, [activePublicTimeline, activeTimeline, thread]);

  const loadTopics = useCallback(
    async (currentTopics: string[], currentActiveTopic: string, currentThread: string | null) => {
      const requestId = loadTopicsRequestRef.current + 1;
      loadTopicsRequestRef.current = requestId;
      const currentState = storeApi.getState();
      const currentTimelineScopeByTopic = currentState.timelineScopeByTopic;
      const currentSelectedAuthorPubkey = currentState.selectedAuthorPubkey;
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
          status,
        ] = await Promise.all([
          Promise.all(
            currentTopics.map(async (topic) => ({
              topic,
              timeline: await api.listTimeline(
                topic,
                null,
                50,
                currentTimelineScopeByTopic[topic] ?? PUBLIC_TIMELINE_SCOPE
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
                currentTimelineScopeByTopic[topic] ?? PUBLIC_TIMELINE_SCOPE
              ),
            }))
          ),
          Promise.allSettled(
            currentTopics.map(async (topic) => ({
              topic,
              rooms: await api.listGameRooms(
                topic,
                currentTimelineScopeByTopic[topic] ?? PUBLIC_TIMELINE_SCOPE
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
          api.getSyncStatus(),
        ]);
        const [
          discoveryResult,
          communityConfigResult,
          communityStatusesResult,
          ticketResult,
          profileResult,
          authorViewResult,
        ] = await Promise.allSettled([
          api.getDiscoveryConfig(),
          api.getCommunityNodeConfig(),
          api.getCommunityNodeStatuses(),
          api.getLocalPeerTicket(),
          api.getMyProfile(),
          currentSelectedAuthorPubkey
            ? api.getAuthorSocialView(currentSelectedAuthorPubkey)
            : Promise.resolve(null),
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
          if (profileResult.status === 'fulfilled') {
            setLocalProfile(profileResult.value);
            if (!currentProfileDirty) {
              setProfileDraft(profileInputFromProfile(profileResult.value));
            }
            setProfileError(null);
            setProfilePanelState({
              status: 'ready',
              error: null,
            });
          } else {
            const nextProfileError = messageFromError(
              profileResult.reason,
              translate('common:errors.failedToLoadProfile')
            );
            setProfileError(nextProfileError);
            setProfilePanelState({
              status: 'error',
              error: nextProfileError,
            });
          }
          if (!currentSelectedAuthorPubkey) {
            setSelectedAuthor(null);
            setAuthorError(null);
          } else if (authorViewResult.status === 'fulfilled') {
            setSelectedAuthor(authorViewResult.value);
            setAuthorError(null);
          } else {
            setAuthorError(
              authorViewResult.reason instanceof Error
                ? authorViewResult.reason.message
                : translate('common:errors.failedToLoadAuthor')
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
      setProfileDraft,
      setProfileError,
      setProfilePanelState,
      setSelectedAuthor,
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
  }, [activeTopic, loadTopics, selectedThread, trackedTopics]);

  useEffect(() => {
    const remoteObjectUrls = remoteObjectUrlRef.current;
    const draftPreviewUrls = draftPreviewUrlRef.current;

    return () => {
      for (const url of remoteObjectUrls.values()) {
        URL.revokeObjectURL(url);
      }
      remoteObjectUrls.clear();
      for (const url of draftPreviewUrls.values()) {
        URL.revokeObjectURL(url);
      }
      draftPreviewUrls.clear();
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
      navOpen: false,
    }));
    setSelectedThread(null);
    setThread([]);
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setAuthorError(null);
    window.requestAnimationFrame(() => {
      primarySectionRefs.current[section]?.focus();
    });
    syncRoute('push', {
      primarySection: section,
      profileMode: section === 'profile' ? 'overview' : undefined,
      selectedAuthorPubkey: null,
      selectedThread: null,
    });
  }

  const closeAuthorPane = useCallback(() => {
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setAuthorError(null);
    syncRoute('replace', {
      selectedAuthorPubkey: null,
    });
  }, [setAuthorError, setSelectedAuthor, setSelectedAuthorPubkey, syncRoute]);

  const closeThreadPane = useCallback(() => {
    setSelectedThread(null);
    setThread([]);
    setReplyTarget(null);
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setAuthorError(null);
    syncRoute('replace', {
      selectedThread: null,
      selectedAuthorPubkey: null,
    });
  }, [
    setAuthorError,
    setReplyTarget,
    setSelectedAuthor,
    setSelectedAuthorPubkey,
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
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setAuthorError(null);
  }

  function handleProfileFieldChange(
    field: 'displayName' | 'name' | 'about' | 'picture',
    value: string
  ) {
    const nextField: keyof ProfileInput =
      field === 'displayName'
        ? 'display_name'
        : field === 'picture'
          ? 'picture'
          : field;
    setProfileDraft((current) => ({
      ...current,
      [nextField]: value,
    }));
    setProfileDirty(true);
  }

  function resetProfileDraft() {
    if (!localProfile) {
      return;
    }
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

  function handleSelectPrivateChannel(channelId: string) {
    setSelectedChannelIdByTopic((current) => ({
      ...current,
      [activeTopic]: channelId,
    }));
    setTimelineScopeByTopic((current) => ({
      ...current,
      [activeTopic]: {
        kind: 'channel',
        channel_id: channelId,
      },
    }));
    setComposeChannelByTopic((current) => ({
      ...current,
      [activeTopic]: {
        kind: 'private_channel',
        channel_id: channelId,
      },
    }));
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'channels',
    }));
    window.requestAnimationFrame(() => {
      syncRoute('replace');
    });
  }

  async function handleSaveProfile(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setProfileSaving(true);
    try {
      const profile = await api.setMyProfile(profileDraft);
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
    setShellChromeState((current) => ({
      ...current,
      navOpen: false,
    }));
    clearThreadContext();
    syncRoute('replace', {
      activeTopic: topic,
    });
    await loadTopics(trackedTopics, topic, null);
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

  async function handleTimelineScopeChange(value: string) {
    const nextScope = timelineScopeFromValue(value);
    setTimelineScopeByTopic((current) => ({
      ...current,
      [activeTopic]: nextScope,
    }));
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'timeline',
    }));
    syncRoute('replace', {
      timelineScope: nextScope,
    });
    await loadTopics(trackedTopics, activeTopic, selectedThread);
  }

  function handleComposeChannelChange(value: string) {
    const nextChannelRef = channelRefFromValue(value);
    setComposeChannelByTopic((current) => ({
      ...current,
      [activeTopic]: nextChannelRef,
    }));
    setSelectedChannelIdByTopic((current) => ({
      ...current,
      [activeTopic]:
        nextChannelRef.kind === 'private_channel' ? nextChannelRef.channel_id : current[activeTopic] ?? null,
    }));
    window.requestAnimationFrame(() => {
      syncRoute('replace', {
        composeTarget: nextChannelRef,
      });
    });
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
        activePrimarySection: 'channels',
      }));
      syncRoute('replace', {
        activeTopic,
        composeTarget: {
          kind: 'private_channel',
          channel_id: channel.channel_id,
        },
        primarySection: 'channels',
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

  async function handleCreateInvite() {
    if (!activePrivateChannel) {
      setChannelError(translate('channels:errors.selectChannelForInvite'));
      return;
    }
    setChannelActionPending('invite');
    try {
      const token = await api.exportPrivateChannelInvite(activeTopic, activePrivateChannel.channel_id, null);
      setInviteOutput(token);
      setInviteOutputLabel('invite');
      setChannelError(null);
    } catch (inviteError) {
      setChannelError(
        messageFromError(inviteError, translate('channels:errors.failedCreateInvite'))
      );
    } finally {
      setChannelActionPending(null);
    }
  }

  async function handleCreateGrant() {
    if (!activePrivateChannel) {
      setChannelError(translate('channels:errors.selectChannelForGrant'));
      return;
    }
    setChannelActionPending('grant');
    try {
      const token = await api.exportFriendOnlyGrant(activeTopic, activePrivateChannel.channel_id, null);
      setInviteOutput(token);
      setInviteOutputLabel('grant');
      setChannelError(null);
    } catch (grantError) {
      setChannelError(
        messageFromError(grantError, translate('channels:errors.failedCreateGrant'))
      );
    } finally {
      setChannelActionPending(null);
    }
  }

  async function handleCreateShare() {
    if (!activePrivateChannel) {
      setChannelError(translate('channels:errors.selectChannelForShare'));
      return;
    }
    setChannelActionPending('share');
    try {
      const token = await api.exportFriendPlusShare(activeTopic, activePrivateChannel.channel_id, null);
      setInviteOutput(token);
      setInviteOutputLabel('share');
      setChannelError(null);
    } catch (shareError) {
      setChannelError(
        messageFromError(shareError, translate('channels:errors.failedCreateShare'))
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
      activePrimarySection: 'channels',
    }));
    clearThreadContext();
    syncRoute('replace', {
      activeTopic: topicId,
      composeTarget: {
        kind: 'private_channel',
        channel_id: channelId,
      },
      primarySection: 'channels',
      timelineScope: {
        kind: 'channel',
        channel_id: channelId,
      },
    });
    await loadTopics(nextTopics, topicId, null);
  }

  async function handleJoinInvite(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!inviteTokenInput.trim()) {
      setChannelError(translate('channels:errors.inviteTokenRequired'));
      return;
    }
    setChannelActionPending('join-invite');
    try {
      const preview = await api.importPrivateChannelInvite(inviteTokenInput.trim());
      await activateImportedPrivateChannel(
        preview.topic_id,
        preview.channel_id,
        joinedChannelFromInvitePreview(preview)
      );
    } catch (inviteError) {
      setChannelError(
        messageFromError(inviteError, translate('channels:errors.failedJoinChannel'))
      );
    } finally {
      setChannelActionPending(null);
    }
  }

  async function handleJoinGrant() {
    if (!inviteTokenInput.trim()) {
      setChannelError(translate('channels:errors.grantTokenRequired'));
      return;
    }
    setChannelActionPending('join-grant');
    try {
      const preview = await api.importFriendOnlyGrant(inviteTokenInput.trim());
      await activateImportedPrivateChannel(
        preview.topic_id,
        preview.channel_id,
        joinedChannelFromFriendGrantPreview(preview)
      );
    } catch (grantError) {
      setChannelError(
        messageFromError(grantError, translate('channels:errors.failedJoinFriendsChannel'))
      );
    } finally {
      setChannelActionPending(null);
    }
  }

  async function handleJoinShare() {
    if (!inviteTokenInput.trim()) {
      setChannelError(translate('channels:errors.shareTokenRequired'));
      return;
    }
    setChannelActionPending('join-share');
    try {
      const preview = await api.importFriendPlusShare(inviteTokenInput.trim());
      await activateImportedPrivateChannel(
        preview.topic_id,
        preview.channel_id,
        joinedChannelFromFriendSharePreview(preview)
      );
    } catch (shareError) {
      setChannelError(
        messageFromError(shareError, translate('channels:errors.failedJoinFriendsPlusChannel'))
      );
    } finally {
      setChannelActionPending(null);
    }
  }

  async function handleFreezePrivateChannel() {
    if (!activePrivateChannel) {
      setChannelError(translate('channels:errors.selectChannelForFreeze'));
      return;
    }
    setChannelActionPending('freeze');
    try {
      await api.freezePrivateChannel(activeTopic, activePrivateChannel.channel_id);
      setInviteOutput(null);
      setChannelError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (freezeError) {
      setChannelError(
        messageFromError(freezeError, translate('channels:errors.failedFreezeChannel'))
      );
    } finally {
      setChannelActionPending(null);
    }
  }

  async function handleRotatePrivateChannel() {
    if (!activePrivateChannel) {
      setChannelError(translate('channels:errors.selectChannelForRotate'));
      return;
    }
    setChannelActionPending('rotate');
    try {
      await api.rotatePrivateChannel(activeTopic, activePrivateChannel.channel_id);
      setInviteOutput(null);
      setChannelError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (rotateError) {
      setChannelError(
        messageFromError(rotateError, translate('channels:errors.failedRotateChannel'))
      );
    } finally {
      setChannelActionPending(null);
    }
  }

  async function handlePublish(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const attachments = draftMediaItems.flatMap((item) => item.attachments);
    if (!composer.trim() && attachments.length === 0) {
      return;
    }

    try {
      await api.createPost(
        activeTopic,
        composer.trim(),
        replyTarget?.object_id ?? null,
        attachments,
        activeComposeChannel
      );
      releaseAllDraftPreviews();
      setComposer('');
      setDraftMediaItems([]);
      setAttachmentInputKey((value) => value + 1);
      setComposerError(null);
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: 'timeline',
      }));
      await loadTopics(trackedTopics, activeTopic, selectedThread);
      setReplyTarget(null);
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
        });
        syncRoute('replace', {
          activeTopic: topic,
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
        setError(null);
      });
      syncRoute(options?.historyMode ?? 'push', {
        activeTopic: topic,
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
        });
        syncRoute('replace', {
          activeTopic: topic,
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
    setError,
    setSelectedAuthor,
    setSelectedAuthorPubkey,
    setSelectedThread,
    setThread,
    syncRoute,
  ]);

  function beginReply(post: PostView) {
    const threadId = post.root_id ?? post.object_id;
    setReplyTarget(post);
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
      setAuthorError(null);
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
    setSelectedAuthor,
    setSelectedAuthorPubkey,
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
      setSelectedAuthorPubkey(authorPubkey);
      setSelectedAuthor(nextView);
      setAuthorError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (relationshipError) {
      setAuthorError(
        relationshipError instanceof Error
          ? relationshipError.message
          : translate('common:errors.failedToUpdateFollowState')
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
    const requestedTimelineScopeValue = params.get('timelineScope');
    const requestedComposeTargetValue = params.get('composeTarget');
    const requestedSettingsSection = params.get('settings');
    const requestedContext = params.get('context');
    const requestedProfileMode = params.get('profileMode');
    const requestedThreadId = params.get('threadId');
    const requestedAuthorPubkey = params.get('authorPubkey');

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

    const joinedChannelsForTopic = joinedChannelsByTopic[nextTopic] ?? [];
    const currentTimelineScopeForTopic = timelineScopeByTopic[nextTopic] ?? PUBLIC_TIMELINE_SCOPE;
    const currentComposeTargetForTopic = composeChannelByTopic[nextTopic] ?? PUBLIC_CHANNEL_REF;
    let nextTimelineScope = PUBLIC_TIMELINE_SCOPE;
    let nextComposeTarget = PUBLIC_CHANNEL_REF;

    if (requestedTimelineScopeValue) {
      const parsedTimelineScope = timelineScopeFromValue(requestedTimelineScopeValue);
      if (
        parsedTimelineScope.kind === 'channel' &&
        !joinedChannelsForTopic.some((channel) => channel.channel_id === parsedTimelineScope.channel_id)
      ) {
        shouldNormalize = true;
      } else {
        nextTimelineScope = parsedTimelineScope;
      }
    }

    if (requestedComposeTargetValue) {
      const parsedComposeTarget = channelRefFromValue(requestedComposeTargetValue);
      if (
        parsedComposeTarget.kind === 'private_channel' &&
        !joinedChannelsForTopic.some((channel) => channel.channel_id === parsedComposeTarget.channel_id)
      ) {
        shouldNormalize = true;
      } else {
        nextComposeTarget = parsedComposeTarget;
      }
    }

    if (timelineScopeValue(currentTimelineScopeForTopic) !== timelineScopeValue(nextTimelineScope)) {
      setTimelineScopeByTopic((current) => ({
        ...current,
        [nextTopic]: nextTimelineScope,
      }));
      shouldReload = true;
    }

    if (channelRefValue(currentComposeTargetForTopic) !== channelRefValue(nextComposeTarget)) {
      setComposeChannelByTopic((current) => ({
        ...current,
        [nextTopic]: nextComposeTarget,
      }));
    }

    if (
      nextComposeTarget.kind === 'private_channel' &&
      selectedChannelIdByTopic[nextTopic] !== nextComposeTarget.channel_id
    ) {
      setSelectedChannelIdByTopic((current) => ({
        ...current,
        [nextTopic]: nextComposeTarget.channel_id,
      }));
    }

    if (
      nextTimelineScope.kind === 'channel' &&
      selectedChannelIdByTopic[nextTopic] !== nextTimelineScope.channel_id
    ) {
      setSelectedChannelIdByTopic((current) => ({
        ...current,
        [nextTopic]: nextTimelineScope.channel_id,
      }));
    }

    const nextSettingsOpen = isSettingsSection(requestedSettingsSection);
    const nextSettingsSection = isSettingsSection(requestedSettingsSection)
      ? requestedSettingsSection
      : shellChromeState.activeSettingsSection;
    const nextProfileMode =
      routeSection === 'profile' && requestedProfileMode === 'edit' ? 'edit' : 'overview';

    if (
      shellChromeState.activePrimarySection !== routeSection ||
      shellChromeState.activeSettingsSection !== nextSettingsSection ||
      shellChromeState.settingsOpen !== nextSettingsOpen ||
      shellChromeState.profileMode !== nextProfileMode
    ) {
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: routeSection,
        activeSettingsSection: nextSettingsSection,
        settingsOpen: nextSettingsOpen,
        profileMode: nextProfileMode,
      }));
    }

    if (requestedSettingsSection && !isSettingsSection(requestedSettingsSection)) {
      shouldNormalize = true;
    }
    if (requestedProfileMode && requestedProfileMode !== 'edit') {
      shouldNormalize = true;
    }
    if (requestedProfileMode && routeSection !== 'profile') {
      shouldNormalize = true;
    }

    if (requestedContext === 'thread') {
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
    } else if (requestedContext === 'author') {
      if (requestedThreadId) {
        shouldNormalize = true;
      }
      if (selectedThread) {
        setSelectedThread(null);
        setThread([]);
        setReplyTarget(null);
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
    } else if (requestedContext) {
      shouldNormalize = true;
      if (selectedThread || selectedAuthorPubkey) {
        setSelectedThread(null);
        setThread([]);
        setReplyTarget(null);
        setSelectedAuthorPubkey(null);
        setSelectedAuthor(null);
        setAuthorError(null);
      }
    } else {
      if (requestedThreadId || requestedAuthorPubkey) {
        shouldNormalize = true;
      }
      if (selectedThread || selectedAuthorPubkey) {
        setSelectedThread(null);
        setThread([]);
        setReplyTarget(null);
        setSelectedAuthorPubkey(null);
        setSelectedAuthor(null);
        setAuthorError(null);
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
    openThread,
    routeSection,
    selectedAuthor,
    selectedAuthorPubkey,
    selectedChannelIdByTopic,
    selectedThread,
    setAuthorError,
    setSelectedThread,
    setActiveTopic,
    setComposeChannelByTopic,
    setSelectedChannelIdByTopic,
    setSelectedAuthor,
    setSelectedAuthorPubkey,
    setShellChromeState,
    setReplyTarget,
    setThread,
    setTimelineScopeByTopic,
    shellChromeState.activePrimarySection,
    shellChromeState.activeSettingsSection,
    shellChromeState.profileMode,
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

      return {
        post,
        context,
        authorLabel: authorDisplayLabel(
          post.author_pubkey,
          post.author_display_name,
          post.author_name
        ),
        authorPicture:
          post.author_pubkey === syncStatus.local_author_pubkey
            ? localProfile?.picture ?? null
            : selectedAuthor?.author_pubkey === post.author_pubkey
              ? selectedAuthor.picture ?? null
              : null,
        relationshipLabel: strongestRelationshipLabel(post),
        audienceChipLabel: post.channel_id
          ? activeJoinedChannels.find((channel) => channel.channel_id === post.channel_id)?.label ??
            localizeAudienceLabel(post.audience_label)
          : localizeAudienceLabel(post.audience_label),
        threadTargetId: post.root_id ?? post.object_id,
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
      localProfile?.picture,
      locale,
      mediaObjectUrls,
      selectedAuthor,
      setUnsupportedVideoManifests,
      syncStatus.local_author_pubkey,
      unsupportedVideoManifests,
    ]
  );

  const activeTimelinePostViews = useMemo(
    () => activeTimeline.map((post) => buildPostCardView(post, 'timeline')),
    [activeTimeline, buildPostCardView]
  );
  const profileTimelinePostViews = useMemo(
    () =>
      activePublicTimeline
        .filter(
          (post) =>
            !post.channel_id &&
            post.author_pubkey === syncStatus.local_author_pubkey
        )
        .map((post) => buildPostCardView(post, 'timeline')),
    [activePublicTimeline, buildPostCardView, syncStatus.local_author_pubkey]
  );
  const threadPostViews = useMemo(
    () => thread.map((post) => buildPostCardView(post, 'thread')),
    [buildPostCardView, thread]
  );
  const topicNavItems = useMemo<TopicDiagnosticSummary[]>(
    () =>
      trackedTopics.map((topic) => ({
        topic,
        active: topic === activeTopic,
        removable: trackedTopics.length > 1,
        connectionLabel: topicConnectionLabel(topicDiagnostics[topic]),
        peerCount: topicDiagnostics[topic]?.peer_count ?? 0,
        lastReceivedLabel: formatLastReceivedLabel(topicDiagnostics[topic]?.last_received_at, locale),
      })),
    [activeTopic, locale, topicDiagnostics, trackedTopics]
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
  const authorDetailView = useMemo<AuthorDetailView>(
    () => ({
      author: selectedAuthor,
      displayLabel: selectedAuthor
        ? authorDisplayLabel(
            selectedAuthor.author_pubkey,
            selectedAuthor.display_name,
            selectedAuthor.name
          )
        : t('common:fallbacks.authorDetail'),
      summary: selectedAuthor
        ? {
            label: strongestRelationshipLabel(selectedAuthor),
            following: selectedAuthor.following,
            followedBy: selectedAuthor.followed_by,
            mutual: selectedAuthor.mutual,
            friendOfFriend: selectedAuthor.friend_of_friend,
            viaPubkeys: selectedAuthor.friend_of_friend_via_pubkeys.map(shortPubkey),
            isSelf: selectedAuthor.author_pubkey === syncStatus.local_author_pubkey,
            canFollow: selectedAuthor.author_pubkey !== syncStatus.local_author_pubkey,
            followActionLabel: selectedAuthor.following ? 'Unfollow' : 'Follow',
          }
        : null,
      authorError,
    }),
    [authorError, selectedAuthor, syncStatus.local_author_pubkey, t]
  );
  const timelineViewScopeOptions = useMemo(
    () => [
      { value: 'public', label: t('common:audience.public') },
      { value: 'all_joined', label: t('common:audience.allJoined') },
      ...activeJoinedChannels.map((channel) => ({
        value: `channel:${channel.channel_id}`,
        label: channel.label,
      })),
    ],
    [activeJoinedChannels, t]
  );
  const composeTargetOptions = useMemo(
    () => [
      { value: 'public', label: t('common:audience.public') },
      ...activeJoinedChannels.map((channel) => ({
        value: `channel:${channel.channel_id}`,
        label: channel.label,
      })),
    ],
    [activeJoinedChannels, t]
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
      onRemoveTopic={(topic) => void handleRemoveTopic(topic)}
    />
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
  const primarySectionItems = useMemo(
    () =>
      PRIMARY_SECTION_ITEMS.map((item) => ({
        ...item,
        label: t(`shell:primarySections.${item.id}`),
      })),
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
  ];

  const channelActionDisabled = channelActionPending !== null;
  const profileAuthorLabel = authorDisplayLabel(
    syncStatus.local_author_pubkey,
    localProfile?.display_name,
    localProfile?.name
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
            onReply={beginReply}
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
            />
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
                      </div>
                      <Button
                        variant='secondary'
                        type='button'
                        onClick={() => void loadTopics(trackedTopics, activeTopic, selectedThread)}
                      >
                        {t('common:actions.refresh')}
                      </Button>
                    </div>
                    <div className='shell-workspace-controls'>
                      <Label>
                        <span>{t('shell:workspace.viewScope')}</span>
                        <Select
                          aria-label={t('shell:workspace.viewScope')}
                          value={timelineScopeValue(activeTimelineScope)}
                          onChange={(event) => {
                            void handleTimelineScopeChange(event.target.value);
                          }}
                        >
                          {timelineViewScopeOptions.map((option) => (
                            <option key={option.value} value={option.value}>
                              {option.label}
                            </option>
                          ))}
                        </Select>
                      </Label>
                      <Label>
                        <span>{t('shell:workspace.composeTarget')}</span>
                        <Select
                          aria-label={t('shell:workspace.composeTarget')}
                          value={channelRefValue(activeComposeChannel)}
                          disabled={Boolean(replyTarget)}
                          onChange={(event) => handleComposeChannelChange(event.target.value)}
                        >
                          {composeTargetOptions.map((option) => (
                            <option key={option.value} value={option.value}>
                              {option.label}
                            </option>
                          ))}
                        </Select>
                      </Label>
                    </div>
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
                      replyTarget={
                        replyTarget
                          ? {
                              content: replyTarget.content,
                              audienceLabel: replyTarget.audience_label,
                            }
                          : null
                      }
                      onClearReply={clearReply}
                    />
                  </Card>
                  <Card className='shell-workspace-card'>
                    <TimelineFeed
                      posts={activeTimelinePostViews}
                      emptyCopy={t('shell:workspace.noPosts')}
                      onOpenAuthor={(authorPubkey) => void openAuthorDetail(authorPubkey)}
                      onOpenThread={(threadId) => void openThread(threadId)}
                      onReply={beginReply}
                    />
                  </Card>
                </>
              ) : null}

              {shellChromeState.activePrimarySection === 'channels' ? (
                <>
                  <Card className='shell-workspace-card'>
                    <div className='shell-main-stack'>
                      <div className='shell-workspace-header'>
                        <div>
                          <h3>{t('channels:title')}</h3>
                          <small>{t('channels:joined', { count: privateChannelListItems.length })}</small>
                        </div>
                      </div>
                      {activeChannelPanelState.status === 'loading' ? (
                        <Notice>{t('channels:loading')}</Notice>
                      ) : null}
                      {activeChannelPanelState.status === 'error' &&
                      (channelError ?? activeChannelPanelState.error) ? (
                        <Notice tone='destructive'>
                          {channelError ?? activeChannelPanelState.error}
                        </Notice>
                      ) : null}
                      <form className='composer composer-compact' onSubmit={handleCreatePrivateChannel}>
                        <Label>
                          <span>{t('channels:editor.createChannel')}</span>
                          <Input
                            value={channelLabelInput}
                            onChange={(event) => setChannelLabelInput(event.target.value)}
                            placeholder={t('channels:editor.placeholders.channelLabel')}
                            disabled={channelActionDisabled}
                          />
                        </Label>
                        <Label>
                          <span>{t('channels:editor.audience')}</span>
                          <Select
                            aria-label={t('channels:editor.audience')}
                            value={channelAudienceInput}
                            disabled={channelActionDisabled}
                            onChange={(event) =>
                              setChannelAudienceInput(
                                event.target.value as ChannelAudienceKind
                              )
                            }
                          >
                            <option value='invite_only'>{t('channels:audienceOptions.invite_only')}</option>
                            <option value='friend_only'>{t('channels:audienceOptions.friend_only')}</option>
                            <option value='friend_plus'>{t('channels:audienceOptions.friend_plus')}</option>
                          </Select>
                        </Label>
                        <Button variant='secondary' type='submit' disabled={channelActionDisabled}>
                          {t('channels:actions.createChannel')}
                        </Button>
                      </form>
                      <form className='composer composer-compact' onSubmit={handleJoinInvite}>
                        <Label>
                          <span>{t('channels:editor.joinViaInvite')}</span>
                          <Textarea
                            value={inviteTokenInput}
                            onChange={(event) => setInviteTokenInput(event.target.value)}
                            placeholder={t('channels:editor.placeholders.inviteToken')}
                            disabled={channelActionDisabled}
                          />
                        </Label>
                        <div className='discovery-actions'>
                          <Button variant='secondary' type='submit' disabled={channelActionDisabled}>
                            {t('channels:actions.joinInvite')}
                          </Button>
                          <Button
                            variant='secondary'
                            type='button'
                            disabled={channelActionDisabled}
                            onClick={() => void handleJoinGrant()}
                          >
                            {t('channels:actions.joinGrant')}
                          </Button>
                          <Button
                            variant='secondary'
                            type='button'
                            disabled={channelActionDisabled}
                            onClick={() => void handleJoinShare()}
                          >
                            {t('channels:actions.joinShare')}
                          </Button>
                        </div>
                      </form>
                      {inviteOutput ? (
                        <Notice tone='accent'>
                          <strong>{inviteOutputSummaryLabel(inviteOutputLabel)}</strong>
                          <code className='extended-inline-code'>{inviteOutput}</code>
                        </Notice>
                      ) : null}
                    </div>
                  </Card>
                  <Card className='shell-workspace-card'>
                    {privateChannelListItems.length === 0 && activeChannelPanelState.status === 'ready' ? (
                      <p className='empty-state'>{t('channels:empty')}</p>
                    ) : (
                      <div className='extended-channel-grid'>
                        <ul className='post-list'>
                          {privateChannelListItems.map(({ channel, active }) => (
                            <li key={channel.channel_id}>
                              <button
                                className={`post-card post-link extended-channel-card${
                                  active ? ' extended-channel-card-active' : ''
                                }`}
                                type='button'
                                aria-pressed={active}
                                onClick={() => handleSelectPrivateChannel(channel.channel_id)}
                              >
                                <div className='post-meta'>
                                  <span>{channel.label}</span>
                                  <span>{translateAudienceKindLabel(channel.audience_kind)}</span>
                                </div>
                                <div className='topic-diagnostic topic-diagnostic-secondary'>
                                  <span>{t('common:labels.epoch')}: {channel.current_epoch_id}</span>
                                  <span>{t('common:labels.sharing')}: {channel.sharing_state}</span>
                                </div>
                              </button>
                            </li>
                          ))}
                        </ul>

                        <Card
                          tone={activePrivateChannel ? 'accent' : 'default'}
                          className='extended-channel-detail'
                        >
                          <div className='panel-header'>
                            <div>
                              <h4>{activePrivateChannel?.label ?? t('channels:selectChannel')}</h4>
                              <small>
                                {activePrivateChannel
                                  ? channelPolicyDescription(activePrivateChannel.audience_kind)
                                  : t('channels:inspectHint')}
                              </small>
                            </div>
                          </div>

                          {activePrivateChannel ? (
                            <>
                              <div className='topic-diagnostic topic-diagnostic-secondary'>
                                <span>
                                  {t('common:labels.policy')}: {channelPolicyDescription(activePrivateChannel.audience_kind)}
                                </span>
                                <span>{t('common:labels.epoch')}: {activePrivateChannel.current_epoch_id}</span>
                                <span>{t('common:labels.sharing')}: {activePrivateChannel.sharing_state}</span>
                                {activePrivateChannel.joined_via_pubkey ? (
                                  <span>
                                    {t('common:labels.joinedVia')} {shortPubkey(activePrivateChannel.joined_via_pubkey)}
                                  </span>
                                ) : null}
                              </div>
                              {(activePrivateChannel.audience_kind === 'friend_only' ||
                                activePrivateChannel.audience_kind === 'friend_plus') ? (
                                <div className='topic-diagnostic topic-diagnostic-secondary'>
                                  <span>{t('common:labels.participants')}: {formatCount(activePrivateChannel.participant_count)}</span>
                                  <span>{t('common:labels.stale')}: {formatCount(activePrivateChannel.stale_participant_count)}</span>
                                  <span>{t('common:labels.owner')}: {translateBooleanLabel(activePrivateChannel.is_owner)}</span>
                                </div>
                              ) : null}
                              {activePrivateChannel.audience_kind === 'friend_only' &&
                              activePrivateChannel.rotation_required ? (
                                <div className='topic-diagnostic topic-diagnostic-error'>
                                  <span>{t('channels:rotationRequired')}</span>
                                </div>
                              ) : null}
                              <div className='discovery-actions'>
                                {activePrivateChannel.audience_kind === 'invite_only' ? (
                                  <Button
                                    variant='secondary'
                                    type='button'
                                    disabled={channelActionDisabled}
                                    onClick={() => void handleCreateInvite()}
                                  >
                                    {t('channels:actions.createInvite')}
                                  </Button>
                                ) : null}
                                {activePrivateChannel.audience_kind === 'friend_only' ? (
                                  <Button
                                    variant='secondary'
                                    type='button'
                                    disabled={channelActionDisabled || !activePrivateChannel.is_owner}
                                    onClick={() => void handleCreateGrant()}
                                  >
                                    {t('channels:actions.createGrant')}
                                  </Button>
                                ) : null}
                                {activePrivateChannel.audience_kind === 'friend_plus' ? (
                                  <Button
                                    variant='secondary'
                                    type='button'
                                    disabled={channelActionDisabled}
                                    onClick={() => void handleCreateShare()}
                                  >
                                    {t('channels:actions.createShare')}
                                  </Button>
                                ) : null}
                                {activePrivateChannel.audience_kind === 'friend_plus' ? (
                                  <Button
                                    variant='secondary'
                                    type='button'
                                    disabled={channelActionDisabled || !activePrivateChannel.is_owner}
                                    onClick={() => void handleFreezePrivateChannel()}
                                  >
                                    {t('common:actions.freeze')}
                                  </Button>
                                ) : null}
                                {activePrivateChannel.audience_kind === 'friend_only' ||
                                activePrivateChannel.audience_kind === 'friend_plus' ? (
                                  <Button
                                    variant='secondary'
                                    type='button'
                                    disabled={channelActionDisabled || !activePrivateChannel.is_owner}
                                    onClick={() => void handleRotatePrivateChannel()}
                                  >
                                    {t('common:actions.rotate')}
                                  </Button>
                                ) : null}
                              </div>
                            </>
                          ) : (
                            <Notice>{t('channels:selectChannelNotice')}</Notice>
                          )}
                        </Card>
                      </div>
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
                      <div className='topic-diagnostic topic-diagnostic-secondary'>
                        <span>{t('common:labels.audience')}: {activeComposeAudienceLabel}</span>
                      </div>
                      <Button type='submit' disabled={liveCreatePending}>
                        {t('live:actions.start')}
                      </Button>
                    </form>
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
                      <div className='topic-diagnostic topic-diagnostic-secondary'>
                        <span>{t('common:labels.audience')}: {activeComposeAudienceLabel}</span>
                      </div>
                      <Button type='submit' disabled={gameCreatePending}>
                        {t('game:actions.createRoom')}
                      </Button>
                    </form>
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
                      onFieldChange={handleProfileFieldChange}
                      onBack={openProfileOverview}
                      onSave={handleSaveProfile}
                      onReset={resetProfileDraft}
                    />
                  ) : (
                    <ProfileOverviewPanel
                      authorLabel={profileAuthorLabel}
                      about={localProfile?.about ?? null}
                      picture={localProfile?.picture ?? null}
                      status={profilePanelState.status}
                      error={profileError ?? profilePanelState.error}
                      postCount={profileTimelinePostViews.length}
                      onEdit={openProfileEditor}
                    />
                  )}
                  <Card className='shell-workspace-card'>
                    <TimelineFeed
                      posts={profileTimelinePostViews}
                      emptyCopy={t('shell:workspace.noPublicPosts')}
                      onOpenAuthor={(authorPubkey) => void openAuthorDetail(authorPubkey)}
                      onOpenThread={(threadId) => void openThread(threadId)}
                      onReply={beginReply}
                    />
                  </Card>
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
