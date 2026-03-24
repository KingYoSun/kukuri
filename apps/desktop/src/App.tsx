import {
  ChangeEvent,
  FormEvent,
  SyntheticEvent,
  startTransition,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';

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
import { ContextPane } from '@/components/shell/ContextPane';
import { ShellFrame } from '@/components/shell/ShellFrame';
import { ShellNavRail } from '@/components/shell/ShellNavRail';
import { SettingsDrawer } from '@/components/shell/SettingsDrawer';
import { ShellTopBar } from '@/components/shell/ShellTopBar';
import {
  type ContextPaneMode,
  type PrimarySection,
  type SettingsSection,
  type ShellChromeState,
} from '@/components/shell/types';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
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
  GameRoomStatus,
  GameRoomView,
  GameScoreView,
  JoinedPrivateChannelView,
  LiveSessionView,
  PostView,
  Profile,
  ProfileInput,
  SyncStatus,
  TimelineScope,
  TopicSyncStatus,
  runtimeApi,
} from './lib/api';
import { blobToCreateAttachment, fileToCreateAttachment } from './lib/attachments';

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
  status_detail: 'No peers configured',
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

const PRIMARY_SECTION_ITEMS: Array<{
  id: PrimarySection;
  label: string;
  description: string;
}> = [
  {
    id: 'timeline',
    label: 'Timeline',
    description: 'Jump to the main feed, scope, and refresh controls.',
  },
  {
    id: 'channels',
    label: 'Channels',
    description: 'Private channel controls, invite import, and audience policy.',
  },
  {
    id: 'live',
    label: 'Live',
    description: 'Live session creation and active session status.',
  },
  {
    id: 'game',
    label: 'Game',
    description: 'Game room creation, score editing, and room status.',
  },
];

const SETTINGS_SECTION_COPY: Array<{
  id: SettingsSection;
  label: string;
  description: string;
}> = [
  {
    id: 'profile',
    label: 'Profile',
    description: 'Edit your local profile without leaving the workspace.',
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

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
  if (bytes >= 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${bytes} B`;
}

function shortPubkey(pubkey: string): string {
  return pubkey.slice(0, 12);
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
    return 'Public';
  }
  return (
    joinedChannels.find((channel) => channel.channel_id === channelRef.channel_id)?.label ??
    'Private channel'
  );
}

function audienceLabelForTimelineScope(
  scope: TimelineScope,
  joinedChannels: JoinedPrivateChannelView[]
): string {
  if (scope.kind === 'all_joined') {
    return 'All joined';
  }
  if (scope.kind === 'channel') {
    return (
      joinedChannels.find((channel) => channel.channel_id === scope.channel_id)?.label ??
      'Private channel'
    );
  }
  return 'Public';
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
    return 'error';
  }
  return syncStatus.connected ? 'connected' : 'waiting';
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
    return 'pending consent acceptance';
  }
  if (status?.auth_state.authenticated) {
    return 'not resolved yet';
  }
  return 'not resolved';
}

function communityNodeNextStepLabel(status?: CommunityNodeNodeStatus): string {
  if (!status) {
    return 'save nodes to begin authentication';
  }
  if (!status.auth_state.authenticated) {
    return 'authenticate this node';
  }
  if (status.consent_state && !status.consent_state.all_required_accepted) {
    return 'accept required policies to resolve connectivity urls';
  }
  if (status.restart_required) {
    return 'unexpected restart requirement; refresh metadata or restart only as fallback';
  }
  if (!status.resolved_urls) {
    return 'refresh metadata if connectivity urls stay unresolved';
  }
  return 'connectivity urls active on current session';
}

function communityNodeSessionActivationLabel(status?: CommunityNodeNodeStatus): string {
  if (!status) {
    return 'unknown';
  }
  if (status.restart_required) {
    return 'restart required (unexpected)';
  }
  if (status.resolved_urls?.connectivity_urls?.length) {
    return 'active on current session';
  }
  if (status.consent_state && !status.consent_state.all_required_accepted) {
    return 'waiting for consent acceptance';
  }
  if (status.auth_state.authenticated) {
    return 'awaiting connectivity metadata';
  }
  return 'not authenticated';
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
      reject(new Error('failed to generate video poster'));
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
        reject(new Error('failed to generate video poster'));
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

export function App({ api = runtimeApi }: AppProps) {
  const [trackedTopics, setTrackedTopics] = useState<string[]>([DEFAULT_TOPIC]);
  const [activeTopic, setActiveTopic] = useState(DEFAULT_TOPIC);
  const [topicInput, setTopicInput] = useState('');
  const [composer, setComposer] = useState('');
  const [draftMediaItems, setDraftMediaItems] = useState<DraftMediaItem[]>([]);
  const [attachmentInputKey, setAttachmentInputKey] = useState(0);
  const [timelinesByTopic, setTimelinesByTopic] = useState<Record<string, PostView[]>>({
    [DEFAULT_TOPIC]: [],
  });
  const [liveSessionsByTopic, setLiveSessionsByTopic] = useState<Record<string, LiveSessionView[]>>({
    [DEFAULT_TOPIC]: [],
  });
  const [gameRoomsByTopic, setGameRoomsByTopic] = useState<Record<string, GameRoomView[]>>({
    [DEFAULT_TOPIC]: [],
  });
  const [joinedChannelsByTopic, setJoinedChannelsByTopic] = useState<
    Record<string, JoinedPrivateChannelView[]>
  >({
    [DEFAULT_TOPIC]: [],
  });
  const [timelineScopeByTopic, setTimelineScopeByTopic] = useState<Record<string, TimelineScope>>({
    [DEFAULT_TOPIC]: PUBLIC_TIMELINE_SCOPE,
  });
  const [composeChannelByTopic, setComposeChannelByTopic] = useState<Record<string, ChannelRef>>({
    [DEFAULT_TOPIC]: PUBLIC_CHANNEL_REF,
  });
  const [thread, setThread] = useState<PostView[]>([]);
  const [selectedThread, setSelectedThread] = useState<string | null>(null);
  const [replyTarget, setReplyTarget] = useState<PostView | null>(null);
  const [peerTicket, setPeerTicket] = useState('');
  const [localPeerTicket, setLocalPeerTicket] = useState<string | null>(null);
  const [discoveryConfig, setDiscoveryConfig] = useState<DiscoveryConfig>(DEFAULT_DISCOVERY_CONFIG);
  const [discoverySeedInput, setDiscoverySeedInput] = useState('');
  const [discoveryEditorDirty, setDiscoveryEditorDirty] = useState(false);
  const [discoveryError, setDiscoveryError] = useState<string | null>(null);
  const [communityNodeConfig, setCommunityNodeConfig] = useState<CommunityNodeConfig>(
    DEFAULT_COMMUNITY_NODE_CONFIG
  );
  const [communityNodeStatuses, setCommunityNodeStatuses] = useState<CommunityNodeNodeStatus[]>([]);
  const [communityNodeInput, setCommunityNodeInput] = useState('');
  const [communityNodeEditorDirty, setCommunityNodeEditorDirty] = useState(false);
  const [communityNodeError, setCommunityNodeError] = useState<string | null>(null);
  const [mediaObjectUrls, setMediaObjectUrls] = useState<Record<string, string | null>>({});
  const [unsupportedVideoManifests, setUnsupportedVideoManifests] = useState<
    Record<string, true>
  >({});
  const [syncStatus, setSyncStatus] = useState<SyncStatus>(DEFAULT_SYNC_STATUS);
  const [localProfile, setLocalProfile] = useState<Profile | null>(null);
  const [profileDraft, setProfileDraft] = useState<ProfileInput>({});
  const [profileDirty, setProfileDirty] = useState(false);
  const [profileError, setProfileError] = useState<string | null>(null);
  const [selectedAuthorPubkey, setSelectedAuthorPubkey] = useState<string | null>(null);
  const [selectedAuthor, setSelectedAuthor] = useState<AuthorSocialView | null>(null);
  const [authorError, setAuthorError] = useState<string | null>(null);
  const [composerError, setComposerError] = useState<string | null>(null);
  const [liveTitle, setLiveTitle] = useState('');
  const [liveDescription, setLiveDescription] = useState('');
  const [liveError, setLiveError] = useState<string | null>(null);
  const [channelLabelInput, setChannelLabelInput] = useState('');
  const [channelAudienceInput, setChannelAudienceInput] =
    useState<ChannelAudienceKind>('invite_only');
  const [inviteTokenInput, setInviteTokenInput] = useState('');
  const [inviteOutput, setInviteOutput] = useState<string | null>(null);
  const [inviteOutputLabel, setInviteOutputLabel] = useState<'invite' | 'grant' | 'share'>(
    'invite'
  );
  const [channelError, setChannelError] = useState<string | null>(null);
  const [gameTitle, setGameTitle] = useState('');
  const [gameDescription, setGameDescription] = useState('');
  const [gameParticipantsInput, setGameParticipantsInput] = useState('');
  const [gameError, setGameError] = useState<string | null>(null);
  const [gameDrafts, setGameDrafts] = useState<Record<string, GameEditorDraft>>({});
  const [error, setError] = useState<string | null>(null);
  const [shellChromeState, setShellChromeState] = useState<ShellChromeState>({
    activePrimarySection: 'timeline',
    activeContextPaneMode: 'thread',
    activeSettingsSection: 'profile',
    navOpen: false,
    contextOpen: false,
    settingsOpen: false,
  });
  const draftSequenceRef = useRef(0);
  const mediaFetchAttemptRef = useRef(new Map<string, number>());
  const remoteObjectUrlRef = useRef(new Map<string, string>());
  const draftPreviewUrlRef = useRef(new Map<string, string>());
  const navTriggerRef = useRef<HTMLButtonElement | null>(null);
  const settingsTriggerRef = useRef<HTMLButtonElement | null>(null);
  const contextTriggerRef = useRef<HTMLButtonElement | null>(null);
  const primarySectionRefs = useRef<Record<PrimarySection, HTMLElement | null>>({
    timeline: null,
    channels: null,
    live: null,
    game: null,
  });

  const headline = useMemo(
    () => {
      if (syncStatus.discovery.mode === 'seeded_dht') {
        return syncStatus.connected ? 'Seeded DHT + direct peers' : 'Seeded DHT shell';
      }
      return syncStatus.connected ? 'Live over static peers' : 'Local-first shell';
    },
    [syncStatus.connected, syncStatus.discovery.mode]
  );

  const activeTimeline = useMemo(
    () => timelinesByTopic[activeTopic] ?? [],
    [activeTopic, timelinesByTopic]
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
  const activePrivateChannel = useMemo(
    () =>
      activeComposeChannel.kind === 'private_channel'
        ? activeJoinedChannels.find((channel) => channel.channel_id === activeComposeChannel.channel_id) ??
          null
        : null,
    [activeComposeChannel, activeJoinedChannels]
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
    for (const post of [...activeTimeline, ...thread]) {
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
  }, [activeTimeline, thread]);

  const loadTopics = useCallback(
    async (currentTopics: string[], currentActiveTopic: string, currentThread: string | null) => {
      try {
        const [timelineViews, liveViews, gameViews, joinedChannelViews, threadView, status] =
          await Promise.all([
          Promise.all(
            currentTopics.map(async (topic) => ({
              topic,
              timeline: await api.listTimeline(
                topic,
                null,
                50,
                timelineScopeByTopic[topic] ?? PUBLIC_TIMELINE_SCOPE
              ),
            }))
          ),
          Promise.all(
            currentTopics.map(async (topic) => ({
              topic,
              sessions: await api.listLiveSessions(
                topic,
                timelineScopeByTopic[topic] ?? PUBLIC_TIMELINE_SCOPE
              ),
            }))
          ),
          Promise.all(
            currentTopics.map(async (topic) => ({
              topic,
              rooms: await api.listGameRooms(
                topic,
                timelineScopeByTopic[topic] ?? PUBLIC_TIMELINE_SCOPE
              ),
            }))
          ),
          Promise.all(
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
          selectedAuthorPubkey
            ? api.getAuthorSocialView(selectedAuthorPubkey)
            : Promise.resolve(null),
        ]);
        startTransition(() => {
          setTimelinesByTopic(
            Object.fromEntries(timelineViews.map(({ topic, timeline }) => [topic, timeline.items]))
          );
          setLiveSessionsByTopic(
            Object.fromEntries(liveViews.map(({ topic, sessions }) => [topic, sessions]))
          );
          setGameRoomsByTopic(
            Object.fromEntries(gameViews.map(({ topic, rooms }) => [topic, rooms]))
          );
          setJoinedChannelsByTopic(
            Object.fromEntries(joinedChannelViews.map(({ topic, channels }) => [topic, channels]))
          );
          setSyncStatus(status);
          if (discoveryResult.status === 'fulfilled') {
            setDiscoveryConfig(discoveryResult.value);
            if (!discoveryEditorDirty) {
              setDiscoverySeedInput(seedPeersToEditorValue(discoveryResult.value));
            }
          }
          if (communityConfigResult.status === 'fulfilled') {
            setCommunityNodeConfig(communityConfigResult.value);
            if (!communityNodeEditorDirty) {
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
            if (!profileDirty) {
              setProfileDraft(profileInputFromProfile(profileResult.value));
            }
            setProfileError(null);
          } else {
            setProfileError(
              profileResult.reason instanceof Error
                ? profileResult.reason.message
                : 'failed to load profile'
            );
          }
          if (!selectedAuthorPubkey) {
            setSelectedAuthor(null);
            setAuthorError(null);
          } else if (authorViewResult.status === 'fulfilled') {
            setSelectedAuthor(authorViewResult.value);
            setAuthorError(null);
          } else {
            setAuthorError(
              authorViewResult.reason instanceof Error
                ? authorViewResult.reason.message
                : 'failed to load author'
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
        setError(loadError instanceof Error ? loadError.message : 'failed to load topic');
      }
    },
    [
      api,
      communityNodeEditorDirty,
      discoveryEditorDirty,
      profileDirty,
      selectedAuthorPubkey,
      timelineScopeByTopic,
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
  }, [activeGameRooms]);

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
  }, [api, mediaObjectUrls, previewableMediaAttachments]);

  useEffect(() => {
    if (!selectedThread) {
      return;
    }
    setShellChromeState((current) => ({
      ...current,
      activeContextPaneMode: 'thread',
      contextOpen: true,
    }));
  }, [selectedThread]);

  useEffect(() => {
    if (!selectedAuthorPubkey) {
      return;
    }
    setShellChromeState((current) => ({
      ...current,
      activeContextPaneMode: 'author',
      contextOpen: true,
    }));
  }, [selectedAuthorPubkey]);

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
  }, []);

  const setContextOpen = useCallback((open: boolean, restoreToTrigger = false) => {
    setShellChromeState((current) => ({
      ...current,
      contextOpen: open,
    }));
    if (!open && restoreToTrigger) {
      window.requestAnimationFrame(() => {
        contextTriggerRef.current?.focus();
      });
    }
  }, []);

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
  }, []);

  function setPrimarySectionRef(section: PrimarySection) {
    return (element: HTMLElement | null) => {
      primarySectionRefs.current[section] = element;
    };
  }

  function focusPrimarySection(section: PrimarySection) {
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: section,
      navOpen: false,
    }));
    window.requestAnimationFrame(() => {
      primarySectionRefs.current[section]?.focus();
    });
  }

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
      if (shellChromeState.contextOpen) {
        event.preventDefault();
        setContextOpen(false, true);
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
    setContextOpen,
    setNavOpen,
    setSettingsOpen,
    shellChromeState.contextOpen,
    shellChromeState.navOpen,
    shellChromeState.settingsOpen,
  ]);

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
  }

  async function handleSaveProfile(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    try {
      const profile = await api.setMyProfile(profileDraft);
      setLocalProfile(profile);
      setProfileDraft(profileInputFromProfile(profile));
      setProfileDirty(false);
      setProfileError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (saveError) {
      setProfileError(saveError instanceof Error ? saveError.message : 'failed to save profile');
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
    await loadTopics(nextTopics, nextTopic, null);
  }

  async function handleSelectTopic(topic: string) {
    setActiveTopic(topic);
    setShellChromeState((current) => ({
      ...current,
      navOpen: false,
    }));
    clearThreadContext();
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
    await loadTopics(trackedTopics, activeTopic, selectedThread);
  }

  function handleComposeChannelChange(value: string) {
    setComposeChannelByTopic((current) => ({
      ...current,
      [activeTopic]: channelRefFromValue(value),
    }));
  }

  async function handleCreatePrivateChannel(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!channelLabelInput.trim()) {
      setChannelError('channel label is required');
      return;
    }
    try {
      const channel = await api.createPrivateChannel(
        activeTopic,
        channelLabelInput.trim(),
        channelAudienceInput
      );
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
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (channelCreateError) {
      setChannelError(
        channelCreateError instanceof Error
          ? channelCreateError.message
          : 'failed to create channel'
      );
    }
  }

  async function handleCreateInvite() {
    if (activeComposeChannel.kind !== 'private_channel') {
      setChannelError('select a private channel before creating an invite');
      return;
    }
    try {
      const token = await api.exportPrivateChannelInvite(
        activeTopic,
        activeComposeChannel.channel_id,
        null
      );
      setInviteOutput(token);
      setInviteOutputLabel('invite');
      setChannelError(null);
    } catch (inviteError) {
      setChannelError(
        inviteError instanceof Error ? inviteError.message : 'failed to create invite'
      );
    }
  }

  async function handleCreateGrant() {
    if (activeComposeChannel.kind !== 'private_channel') {
      setChannelError('select a private channel before creating a grant');
      return;
    }
    try {
      const token = await api.exportFriendOnlyGrant(
        activeTopic,
        activeComposeChannel.channel_id,
        null
      );
      setInviteOutput(token);
      setInviteOutputLabel('grant');
      setChannelError(null);
    } catch (grantError) {
      setChannelError(
        grantError instanceof Error ? grantError.message : 'failed to create friend grant'
      );
    }
  }

  async function handleCreateShare() {
    if (activeComposeChannel.kind !== 'private_channel') {
      setChannelError('select a private channel before creating a share');
      return;
    }
    try {
      const token = await api.exportFriendPlusShare(
        activeTopic,
        activeComposeChannel.channel_id,
        null
      );
      setInviteOutput(token);
      setInviteOutputLabel('share');
      setChannelError(null);
    } catch (shareError) {
      setChannelError(
        shareError instanceof Error ? shareError.message : 'failed to create friends+ share'
      );
    }
  }

  async function activateImportedPrivateChannel(topicId: string, channelId: string) {
    const nextTopics = trackedTopics.includes(topicId) ? trackedTopics : [...trackedTopics, topicId];
    setTrackedTopics(nextTopics);
    setActiveTopic(topicId);
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
    await loadTopics(nextTopics, topicId, null);
  }

  async function handleJoinInvite(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!inviteTokenInput.trim()) {
      setChannelError('invite token is required');
      return;
    }
    try {
      const preview = await api.importPrivateChannelInvite(inviteTokenInput.trim());
      await activateImportedPrivateChannel(preview.topic_id, preview.channel_id);
    } catch (inviteError) {
      setChannelError(
        inviteError instanceof Error ? inviteError.message : 'failed to join private channel'
      );
    }
  }

  async function handleJoinGrant() {
    if (!inviteTokenInput.trim()) {
      setChannelError('grant token is required');
      return;
    }
    try {
      const preview = await api.importFriendOnlyGrant(inviteTokenInput.trim());
      await activateImportedPrivateChannel(preview.topic_id, preview.channel_id);
    } catch (grantError) {
      setChannelError(
        grantError instanceof Error ? grantError.message : 'failed to join friends channel'
      );
    }
  }

  async function handleJoinShare() {
    if (!inviteTokenInput.trim()) {
      setChannelError('share token is required');
      return;
    }
    try {
      const preview = await api.importFriendPlusShare(inviteTokenInput.trim());
      await activateImportedPrivateChannel(preview.topic_id, preview.channel_id);
    } catch (shareError) {
      setChannelError(
        shareError instanceof Error ? shareError.message : 'failed to join friends+ channel'
      );
    }
  }

  async function handleFreezePrivateChannel() {
    if (activeComposeChannel.kind !== 'private_channel') {
      setChannelError('select a private channel before freezing it');
      return;
    }
    try {
      await api.freezePrivateChannel(activeTopic, activeComposeChannel.channel_id);
      setInviteOutput(null);
      setChannelError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (freezeError) {
      setChannelError(
        freezeError instanceof Error ? freezeError.message : 'failed to freeze private channel'
      );
    }
  }

  async function handleRotatePrivateChannel() {
    if (activeComposeChannel.kind !== 'private_channel') {
      setChannelError('select a private channel before rotating it');
      return;
    }
    try {
      await api.rotatePrivateChannel(activeTopic, activeComposeChannel.channel_id);
      setInviteOutput(null);
      setChannelError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (rotateError) {
      setChannelError(
        rotateError instanceof Error ? rotateError.message : 'failed to rotate private channel'
      );
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
        activePrimarySection: 'channels',
      }));
      await loadTopics(trackedTopics, activeTopic, selectedThread);
      setReplyTarget(null);
    } catch (publishError) {
      setComposerError(
        publishError instanceof Error ? publishError.message : 'failed to publish'
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
        failures.push(`unsupported attachment type: ${file.name}`);
      } catch (attachmentError) {
        failures.push(
          attachmentError instanceof Error
            ? attachmentError.message
            : 'failed to generate video poster'
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

  async function openThread(threadId: string) {
    try {
      const threadView = await api.listThread(activeTopic, threadId, null, 50);
      startTransition(() => {
        setSelectedThread(threadId);
        setThread(threadView.items);
        setShellChromeState((current) => ({
          ...current,
          activeContextPaneMode: 'thread',
          contextOpen: true,
        }));
      });
    } catch (threadError) {
      setError(threadError instanceof Error ? threadError.message : 'failed to load thread');
    }
  }

  function beginReply(post: PostView) {
    setReplyTarget(post);
    if (post.root_id) {
      setSelectedThread(post.root_id);
      void openThread(post.root_id);
      return;
    }
    setSelectedThread(post.object_id);
    void openThread(post.object_id);
  }

  function clearReply() {
    setReplyTarget(null);
  }

  async function openAuthorDetail(authorPubkey: string) {
    try {
      const socialView = await api.getAuthorSocialView(authorPubkey);
      setSelectedAuthorPubkey(authorPubkey);
      setSelectedAuthor(socialView);
      setAuthorError(null);
      setShellChromeState((current) => ({
        ...current,
        activeContextPaneMode: 'author',
        contextOpen: true,
      }));
    } catch (detailError) {
      setAuthorError(detailError instanceof Error ? detailError.message : 'failed to load author');
    }
  }

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
          : 'failed to update follow state'
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
    } catch (saveError) {
      setDiscoveryError(
        saveError instanceof Error ? saveError.message : 'failed to update discovery seeds'
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
    } catch (saveError) {
      setCommunityNodeError(
        saveError instanceof Error ? saveError.message : 'failed to update community nodes'
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
    } catch (clearError) {
      setCommunityNodeError(
        clearError instanceof Error ? clearError.message : 'failed to clear community nodes'
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
        authError instanceof Error ? authError.message : 'failed to authenticate community node'
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
        clearError instanceof Error ? clearError.message : 'failed to clear community node token'
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
        refreshError instanceof Error ? refreshError.message : 'failed to refresh community node'
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
        consentError instanceof Error ? consentError.message : 'failed to fetch consent status'
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
        consentError instanceof Error ? consentError.message : 'failed to accept consents'
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
      setError(importError instanceof Error ? importError.message : 'failed to import peer');
    }
  }

  async function handleCreateLiveSession(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!liveTitle.trim()) {
      setLiveError('live session title is required');
      return;
    }
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
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (liveCreateError) {
      setLiveError(
        liveCreateError instanceof Error ? liveCreateError.message : 'failed to create live session'
      );
    }
  }

  async function handleJoinLiveSession(sessionId: string) {
    try {
      await api.joinLiveSession(activeTopic, sessionId);
      setLiveError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (joinError) {
      setLiveError(joinError instanceof Error ? joinError.message : 'failed to join live session');
    }
  }

  async function handleLeaveLiveSession(sessionId: string) {
    try {
      await api.leaveLiveSession(activeTopic, sessionId);
      setLiveError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (leaveError) {
      setLiveError(leaveError instanceof Error ? leaveError.message : 'failed to leave live session');
    }
  }

  async function handleEndLiveSession(sessionId: string) {
    try {
      await api.endLiveSession(activeTopic, sessionId);
      setLiveError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (endError) {
      setLiveError(endError instanceof Error ? endError.message : 'failed to end live session');
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
      setGameError('game room title is required');
      return;
    }
    if (participants.length < 2) {
      setGameError('game room requires at least two unique participants');
      return;
    }
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
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (createError) {
      setGameError(createError instanceof Error ? createError.message : 'failed to create game room');
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

  async function handleUpdateGameRoom(room: GameRoomView) {
    const draft = gameDrafts[room.room_id] ?? createGameEditorDraft(room);
    const scores: GameScoreView[] = [];
    for (const score of room.scores) {
      const rawScore = draft.scores[score.participant_id] ?? String(score.score);
      const parsed = Number.parseInt(rawScore, 10);
      if (Number.isNaN(parsed)) {
        setGameError(`invalid score for ${score.label}`);
        return;
      }
      scores.push({
        participant_id: score.participant_id,
        label: score.label,
        score: parsed,
      });
    }
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
      setGameError(updateError instanceof Error ? updateError.message : 'failed to update game room');
    }
  }

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
            ? 'unsupported on this client'
            : videoPlaybackSrc
              ? 'playable video'
              : videoPosterPreviewSrc
                ? 'poster ready'
                : 'syncing poster'
          : mediaKind === 'image'
            ? imagePreviewSrc
              ? 'image ready'
              : 'syncing image'
            : null;

      return {
        post,
        context,
        authorLabel: authorDisplayLabel(
          post.author_pubkey,
          post.author_display_name,
          post.author_name
        ),
        relationshipLabel: strongestRelationshipLabel(post),
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
          metaBytesLabel: mediaMetaAttachment ? formatBytes(mediaMetaAttachment.bytes) : null,
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
    [mediaObjectUrls, unsupportedVideoManifests]
  );

  const activeTimelinePostViews = useMemo(
    () => activeTimeline.map((post) => buildPostCardView(post, 'timeline')),
    [activeTimeline, buildPostCardView]
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
        lastReceivedLabel: topicDiagnostics[topic]?.last_received_at
          ? new Date(topicDiagnostics[topic].last_received_at!).toLocaleTimeString('ja-JP')
          : 'no events',
        expectedPeerCount: topicDiagnostics[topic]?.configured_peer_ids.length ?? 0,
        missingPeerCount: topicDiagnostics[topic]?.missing_peer_ids.length ?? 0,
        statusDetail: topicDiagnostics[topic]?.status_detail ?? 'No topic diagnostics yet',
        lastError: topicDiagnostics[topic]?.last_error ?? null,
      })),
    [activeTopic, topicDiagnostics, trackedTopics]
  );
  const composerDraftViews = useMemo<ComposerDraftMediaView[]>(
    () =>
      draftMediaItems.map((item) => ({
        id: item.id,
        sourceName: item.source_name,
        previewUrl: item.preview_url,
        attachments: item.attachments.map((attachment) => ({
          key: `${attachment.role ?? attachment.mime}-${attachment.file_name ?? item.source_name}`,
          label: attachment.role ?? 'attachment',
          mime: attachment.mime,
          byteSizeLabel: formatBytes(attachment.byte_size),
        })),
      })),
    [draftMediaItems]
  );
  const threadPanelState = useMemo<ThreadPanelState>(
    () => ({
      selectedThreadId: selectedThread,
      summary: selectedThread
        ? `${thread.length} posts in thread`
        : 'Select a post to inspect the thread.',
      emptyCopy: 'Select a post to inspect the thread.',
    }),
    [selectedThread, thread.length]
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
        : 'Author Detail',
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
    [authorError, selectedAuthor, syncStatus.local_author_pubkey]
  );
  const timelineViewScopeOptions = useMemo(
    () => [
      { value: 'public', label: 'Public' },
      { value: 'all_joined', label: 'All joined' },
      ...activeJoinedChannels.map((channel) => ({
        value: `channel:${channel.channel_id}`,
        label: channel.label,
      })),
    ],
    [activeJoinedChannels]
  );
  const composeTargetOptions = useMemo(
    () => [
      { value: 'public', label: 'Public' },
      ...activeJoinedChannels.map((channel) => ({
        value: `channel:${channel.channel_id}`,
        label: channel.label,
      })),
    ],
    [activeJoinedChannels]
  );

  const statusBadges = (
    <div className='shell-status-badges'>
      <StatusBadge
        label={syncStatusBadgeLabel(syncStatus)}
        tone={syncStatusBadgeTone(syncStatus)}
      />
      <StatusBadge label={`${syncStatus.peer_count} peers`} />
      <StatusBadge
        label={syncStatus.discovery.mode === 'seeded_dht' ? 'seeded dht' : 'static peers'}
      />
      {syncStatus.pending_events > 0 ? (
        <StatusBadge label={`${syncStatus.pending_events} pending`} tone='warning' />
      ) : null}
    </div>
  );

  const topicList = (
    <TopicNavList
      items={topicNavItems}
      onSelectTopic={(topic) => void handleSelectTopic(topic)}
      onRemoveTopic={(topic) => void handleRemoveTopic(topic)}
    />
  );

  const settingsSections = [
    {
      ...SETTINGS_SECTION_COPY[0],
      content: (
        <Card>
          <CardHeader>
            <h3>My Profile</h3>
            <small>
              {authorDisplayLabel(
                syncStatus.local_author_pubkey,
                localProfile?.display_name,
                localProfile?.name
              )}
            </small>
          </CardHeader>
          <form className='composer composer-compact' onSubmit={handleSaveProfile}>
            <Label>
              <span>Display Name</span>
              <Input
                value={profileDraft.display_name ?? ''}
                onChange={(event) => {
                  setProfileDraft((current) => ({
                    ...current,
                    display_name: event.target.value,
                  }));
                  setProfileDirty(true);
                }}
                placeholder='Visible label'
              />
            </Label>
            <Label>
              <span>Name</span>
              <Input
                value={profileDraft.name ?? ''}
                onChange={(event) => {
                  setProfileDraft((current) => ({
                    ...current,
                    name: event.target.value,
                  }));
                  setProfileDirty(true);
                }}
                placeholder='Canonical name'
              />
            </Label>
            <Label>
              <span>About</span>
              <Textarea
                value={profileDraft.about ?? ''}
                onChange={(event) => {
                  setProfileDraft((current) => ({
                    ...current,
                    about: event.target.value,
                  }));
                  setProfileDirty(true);
                }}
                className='ticket-output'
                placeholder='Short bio'
              />
            </Label>
            <Label>
              <span>Picture URL</span>
              <Input
                value={profileDraft.picture ?? ''}
                onChange={(event) => {
                  setProfileDraft((current) => ({
                    ...current,
                    picture: event.target.value,
                  }));
                  setProfileDirty(true);
                }}
                placeholder='https://...'
              />
            </Label>
            {profileError ? <p className='error error-inline'>{profileError}</p> : null}
            <div className='discovery-actions'>
              <Button variant='secondary' type='submit' disabled={!profileDirty}>
                Save Profile
              </Button>
              <Button
                variant='secondary'
                type='button'
                disabled={!profileDirty}
                onClick={() => {
                  if (!localProfile) {
                    return;
                  }
                  setProfileDraft(profileInputFromProfile(localProfile));
                  setProfileDirty(false);
                  setProfileError(null);
                }}
              >
                Reset
              </Button>
            </div>
          </form>
        </Card>
      ),
    },
    {
      ...SETTINGS_SECTION_COPY[1],
      content: (
        <div className='shell-main-stack'>
          <Card>
            <CardHeader>
              <h3>Sync Status</h3>
              <small>{syncStatus.connected ? 'connected' : 'waiting'}</small>
            </CardHeader>
            <dl className='status-grid'>
              <div>
                <dt>Connected</dt>
                <dd>{syncStatus.connected ? 'yes' : 'no'}</dd>
              </div>
              <div>
                <dt>Peers</dt>
                <dd>{syncStatus.peer_count}</dd>
              </div>
              <div>
                <dt>Pending</dt>
                <dd>{syncStatus.pending_events}</dd>
              </div>
            </dl>
            <div className='diagnostic-block'>
              <strong>Configured Peers</strong>
              <p>
                {syncStatus.configured_peers.length > 0
                  ? syncStatus.configured_peers.join(', ')
                  : 'none'}
              </p>
            </div>
            <div className='diagnostic-block'>
              <strong>Connection Detail</strong>
              <p>{syncStatus.status_detail}</p>
            </div>
            <div className='diagnostic-block'>
              <strong>Effective Peers</strong>
              <p>{effectivePeerIds.join(', ') || 'none'}</p>
            </div>
            <div className='diagnostic-block'>
              <strong>Last Error</strong>
              <p className={syncStatus.last_error ? 'diagnostic-error' : undefined}>
                {syncStatus.last_error ?? 'none'}
              </p>
            </div>
            {error ? (
              <Notice tone='destructive' className='mt-4'>
                {error}
              </Notice>
            ) : null}
          </Card>

          <Card>
            <CardHeader>
              <h3>Peer Tickets</h3>
              <small>manual connectivity</small>
            </CardHeader>
            <Label>
              <span>Your Ticket</span>
              <Textarea readOnly value={localPeerTicket ?? ''} className='ticket-output' />
            </Label>
            <Label>
              <span>Peer Ticket</span>
              <Input
                value={peerTicket}
                onChange={(event) => setPeerTicket(event.target.value)}
                placeholder='nodeid@127.0.0.1:7777'
              />
            </Label>
            <Button variant='secondary' onClick={() => void handleImportPeer()}>
              Import Peer
            </Button>
          </Card>
        </div>
      ),
    },
    {
      ...SETTINGS_SECTION_COPY[2],
      content: (
        <Card className='discovery-panel'>
          <CardHeader>
            <h3>Discovery</h3>
            <small>{syncStatus.discovery.mode}</small>
          </CardHeader>
          <dl className='status-grid status-grid-compact'>
            <div>
              <dt>Mode</dt>
              <dd>{syncStatus.discovery.mode}</dd>
            </div>
            <div>
              <dt>Connect</dt>
              <dd>{syncStatus.discovery.connect_mode}</dd>
            </div>
            <div>
              <dt>Env Lock</dt>
              <dd>{discoveryConfig.env_locked ? 'yes' : 'no'}</dd>
            </div>
          </dl>
          <div className='diagnostic-block'>
            <strong>Local Endpoint ID</strong>
            <p>{syncStatus.discovery.local_endpoint_id || 'unknown'}</p>
          </div>
          <div className='diagnostic-block'>
            <strong>Connected Peers</strong>
            <p>{syncStatus.discovery.connected_peer_ids.join(', ') || 'none'}</p>
          </div>
          <div className='diagnostic-block'>
            <strong>Relay-assisted Peers</strong>
            <p>{syncStatus.discovery.assist_peer_ids.join(', ') || 'none'}</p>
          </div>
          <div className='diagnostic-block'>
            <strong>Manual Ticket Peers</strong>
            <p>{syncStatus.discovery.manual_ticket_peer_ids.join(', ') || 'none'}</p>
          </div>
          <div className='diagnostic-block'>
            <strong>Community Bootstrap Peers</strong>
            <p>{syncStatus.discovery.bootstrap_seed_peer_ids.join(', ') || 'none'}</p>
          </div>
          <div className='diagnostic-block'>
            <strong>Configured Seed IDs</strong>
            <p>{syncStatus.discovery.configured_seed_peer_ids.join(', ') || 'none'}</p>
          </div>
          <Label>
            <span>Seed Peers</span>
            <Textarea
              value={discoverySeedInput}
              onChange={(event) => {
                setDiscoverySeedInput(event.target.value);
                setDiscoveryEditorDirty(true);
              }}
              readOnly={discoveryConfig.env_locked}
              className='ticket-output discovery-editor'
              placeholder='node_id or node_id@host:port'
            />
          </Label>
          <div className='diagnostic-block'>
            <strong>Discovery Error</strong>
            <p
              className={
                discoveryError || syncStatus.discovery.last_discovery_error
                  ? 'diagnostic-error'
                  : undefined
              }
            >
              {discoveryError ?? syncStatus.discovery.last_discovery_error ?? 'none'}
            </p>
          </div>
          <div className='discovery-actions'>
            <Button
              variant='secondary'
              type='button'
              disabled={discoveryConfig.env_locked || !discoveryEditorDirty}
              onClick={() => void handleSaveDiscoverySeeds()}
            >
              Save Seeds
            </Button>
            <Button
              variant='secondary'
              type='button'
              disabled={!discoveryEditorDirty}
              onClick={() => {
                setDiscoverySeedInput(seedPeersToEditorValue(discoveryConfig));
                setDiscoveryEditorDirty(false);
                setDiscoveryError(null);
              }}
            >
              Reset
            </Button>
          </div>
        </Card>
      ),
    },
    {
      ...SETTINGS_SECTION_COPY[3],
      content: (
        <Card className='discovery-panel'>
          <CardHeader>
            <h3>Community Node</h3>
            <small>{communityNodeStatuses.length} configured</small>
          </CardHeader>
          <Label>
            <span>Base URLs</span>
            <Textarea
              value={communityNodeInput}
              onChange={(event) => {
                setCommunityNodeInput(event.target.value);
                setCommunityNodeEditorDirty(true);
              }}
              className='ticket-output discovery-editor'
              placeholder='https://community.example.com'
            />
          </Label>
          <div className='diagnostic-block'>
            <strong>Community Node Error</strong>
            <p className={communityNodeError ? 'diagnostic-error' : undefined}>
              {communityNodeError ?? 'none'}
            </p>
          </div>
          <div className='discovery-actions'>
            <Button
              variant='secondary'
              type='button'
              disabled={!communityNodeEditorDirty}
              onClick={() => void handleSaveCommunityNodes()}
            >
              Save Nodes
            </Button>
            <Button
              variant='secondary'
              type='button'
              disabled={!communityNodeEditorDirty}
              onClick={() => {
                setCommunityNodeInput(communityNodesToEditorValue(communityNodeConfig));
                setCommunityNodeEditorDirty(false);
                setCommunityNodeError(null);
              }}
            >
              Reset
            </Button>
            <Button
              variant='secondary'
              type='button'
              disabled={communityNodeConfig.nodes.length === 0}
              onClick={() => void handleClearCommunityNodes()}
            >
              Clear
            </Button>
          </div>
          {communityNodeConfig.nodes.map((node) => {
            const status = communityNodeStatusByBaseUrl[node.base_url];
            return (
              <div key={node.base_url} className='diagnostic-block'>
                <strong>{node.base_url}</strong>
                <p>
                  auth:{' '}
                  {status?.auth_state.authenticated
                    ? `yes (${status.auth_state.expires_at ?? 'unknown'})`
                    : 'no'}
                </p>
                <p>
                  consent:{' '}
                  {status?.consent_state
                    ? status.consent_state.all_required_accepted
                      ? 'accepted'
                      : 'required'
                    : 'unknown'}
                </p>
                <p>connectivity urls: {communityNodeConnectivityUrlsLabel(status)}</p>
                <p>session activation: {communityNodeSessionActivationLabel(status)}</p>
                <p>next step: {communityNodeNextStepLabel(status)}</p>
                <div className='discovery-actions'>
                  <Button
                    variant='secondary'
                    type='button'
                    onClick={() => void handleAuthenticateCommunityNode(node.base_url)}
                  >
                    Authenticate
                  </Button>
                  <Button
                    variant='secondary'
                    type='button'
                    onClick={() => void handleFetchCommunityNodeConsents(node.base_url)}
                  >
                    Consents
                  </Button>
                  <Button
                    variant='secondary'
                    type='button'
                    onClick={() => void handleAcceptCommunityNodeConsents(node.base_url)}
                  >
                    Accept
                  </Button>
                  <Button
                    variant='secondary'
                    type='button'
                    onClick={() => void handleRefreshCommunityNode(node.base_url)}
                  >
                    Refresh
                  </Button>
                  <Button
                    variant='secondary'
                    type='button'
                    onClick={() => void handleClearCommunityNodeToken(node.base_url)}
                  >
                    Clear Token
                  </Button>
                </div>
              </div>
            );
          })}
        </Card>
      ),
    },
  ];

  const contextTabs = [
    {
      id: 'thread' as ContextPaneMode,
      label: 'Thread',
      summary: threadPanelState.summary,
      content: (
        <ThreadPanel
          state={threadPanelState}
          posts={threadPostViews}
          onClearThread={() => {
            setSelectedThread(null);
            setThread([]);
            clearReply();
          }}
          onOpenAuthor={(authorPubkey) => void openAuthorDetail(authorPubkey)}
          onOpenThread={(threadId) => void openThread(threadId)}
          onReply={beginReply}
        />
      ),
    },
    {
      id: 'author' as ContextPaneMode,
      label: 'Author',
      summary: selectedAuthor
        ? authorDetailView.displayLabel
        : 'Select an author to inspect profile and relationship.',
      content: (
        <div className='shell-main-stack'>
          <AuthorDetailCard
            view={authorDetailView}
            localAuthorPubkey={syncStatus.local_author_pubkey}
            onClearAuthor={() => {
              setSelectedAuthorPubkey(null);
              setSelectedAuthor(null);
              setAuthorError(null);
            }}
            onToggleRelationship={(authorPubkey, following) =>
              void handleRelationshipAction(authorPubkey, following)
            }
          />
        </div>
      ),
    },
  ];

  return (
    <>
      <ShellFrame
        skipTargetId={SHELL_WORKSPACE_ID}
        topBar={
          <ShellTopBar
            headline={headline}
            activeTopic={activeTopic}
            statusBadges={statusBadges}
            navOpen={shellChromeState.navOpen}
            settingsOpen={shellChromeState.settingsOpen}
            navControlsId={SHELL_NAV_ID}
            settingsControlsId={SHELL_SETTINGS_ID}
            navButtonRef={navTriggerRef}
            settingsButtonRef={settingsTriggerRef}
            onToggleNav={() => setNavOpen(!shellChromeState.navOpen)}
            onToggleSettings={() => setSettingsOpen(!shellChromeState.settingsOpen)}
          />
        }
        navRail={
          <ShellNavRail
            railId={SHELL_NAV_ID}
            open={shellChromeState.navOpen}
            onOpenChange={(open) => setNavOpen(open, !open)}
            primaryItems={PRIMARY_SECTION_ITEMS}
            activePrimarySection={shellChromeState.activePrimarySection}
            onSelectPrimarySection={focusPrimarySection}
            addTopicControl={
              <Label>
                <span>Add Topic</span>
                <div className='topic-input-row'>
                  <Input
                    value={topicInput}
                    onChange={(event) => setTopicInput(event.target.value)}
                    placeholder='kukuri:topic:demo'
                  />
                  <Button variant='secondary' onClick={() => void handleAddTopic()}>
                    Add
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
            <Card className='shell-workspace-card'>
              <div
                className='shell-section'
                ref={setPrimarySectionRef('timeline')}
                tabIndex={-1}
                onFocusCapture={() =>
                  setShellChromeState((current) => ({
                    ...current,
                    activePrimarySection: 'timeline',
                  }))
                }
              >
                <TimelineWorkspaceHeader
                  activeTopic={activeTopic}
                  viewingLabel={audienceLabelForTimelineScope(
                    activeTimelineScope,
                    activeJoinedChannels
                  )}
                  postingLabel={activeComposeAudienceLabel}
                  viewScopeValue={timelineScopeValue(activeTimelineScope)}
                  composeTargetValue={channelRefValue(activeComposeChannel)}
                  viewScopeOptions={timelineViewScopeOptions}
                  composeTargetOptions={composeTargetOptions}
                  contextButtonRef={contextTriggerRef}
                  contextOpen={shellChromeState.contextOpen}
                  contextControlsId={SHELL_CONTEXT_ID}
                  onOpenContext={() => setContextOpen(true)}
                  onRefresh={() => void loadTopics(trackedTopics, activeTopic, selectedThread)}
                  onViewScopeChange={(value) => {
                    void handleTimelineScopeChange(value);
                  }}
                  onComposeTargetChange={handleComposeChannelChange}
                  composeTargetDisabled={Boolean(replyTarget)}
                />
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
                <TimelineFeed
                  posts={activeTimelinePostViews}
                  emptyCopy='No posts yet for this topic.'
                  onOpenAuthor={(authorPubkey) => void openAuthorDetail(authorPubkey)}
                  onOpenThread={(threadId) => void openThread(threadId)}
                  onReply={beginReply}
                />
              </div>

              <section
                className='shell-section'
                ref={setPrimarySectionRef('channels')}
                tabIndex={-1}
                onFocusCapture={() =>
                  setShellChromeState((current) => ({
                    ...current,
                    activePrimarySection: 'channels',
                  }))
                }
              >
                <form className='composer composer-compact' onSubmit={handleCreatePrivateChannel}>
                  <Label>
                    <span>Create Channel</span>
                    <Input
                      value={channelLabelInput}
                      onChange={(event) => setChannelLabelInput(event.target.value)}
                      placeholder='core contributors'
                    />
                  </Label>
                  <Label>
                    <span>Audience</span>
                    <Select
                      aria-label='Channel Audience'
                      value={channelAudienceInput}
                      onChange={(event) =>
                        setChannelAudienceInput(event.target.value as ChannelAudienceKind)
                      }
                    >
                      <option value='invite_only'>Invite only</option>
                      <option value='friend_only'>Friends</option>
                      <option value='friend_plus'>Friends+</option>
                    </Select>
                  </Label>
                  <Button variant='secondary' type='submit'>
                    Create Channel
                  </Button>
                  {activePrivateChannel?.audience_kind === 'invite_only' ? (
                    <Button
                      variant='secondary'
                      type='button'
                      disabled={activeComposeChannel.kind !== 'private_channel'}
                      onClick={() => void handleCreateInvite()}
                    >
                      Create Invite
                    </Button>
                  ) : null}
                  {activePrivateChannel?.audience_kind === 'friend_only' ? (
                    <Button
                      variant='secondary'
                      type='button'
                      disabled={!activePrivateChannel.is_owner}
                      onClick={() => void handleCreateGrant()}
                    >
                      Create Grant
                    </Button>
                  ) : null}
                  {activePrivateChannel?.audience_kind === 'friend_plus' ? (
                    <Button
                      variant='secondary'
                      type='button'
                      disabled={activeComposeChannel.kind !== 'private_channel'}
                      onClick={() => void handleCreateShare()}
                    >
                      Create Share
                    </Button>
                  ) : null}
                  {activePrivateChannel?.audience_kind === 'friend_plus' ? (
                    <Button
                      variant='secondary'
                      type='button'
                      disabled={!activePrivateChannel.is_owner}
                      onClick={() => void handleFreezePrivateChannel()}
                    >
                      Freeze
                    </Button>
                  ) : null}
                  {activePrivateChannel?.audience_kind === 'friend_only' ? (
                    <Button
                      variant='secondary'
                      type='button'
                      disabled={!activePrivateChannel.is_owner}
                      onClick={() => void handleRotatePrivateChannel()}
                    >
                      Rotate
                    </Button>
                  ) : null}
                  {activePrivateChannel?.audience_kind === 'friend_plus' ? (
                    <Button
                      variant='secondary'
                      type='button'
                      disabled={!activePrivateChannel.is_owner}
                      onClick={() => void handleRotatePrivateChannel()}
                    >
                      Rotate
                    </Button>
                  ) : null}
                </form>
                <form className='composer composer-compact' onSubmit={handleJoinInvite}>
                  <Label>
                    <span>Join via Invite</span>
                    <Textarea
                      value={inviteTokenInput}
                      onChange={(event) => setInviteTokenInput(event.target.value)}
                      placeholder='paste private channel invite, friend grant, or friends+ share'
                    />
                  </Label>
                  <Button variant='secondary' type='submit'>
                    Join Invite
                  </Button>
                  <Button variant='secondary' type='button' onClick={() => void handleJoinGrant()}>
                    Join Grant
                  </Button>
                  <Button variant='secondary' type='button' onClick={() => void handleJoinShare()}>
                    Join Share
                  </Button>
                </form>
                {inviteOutput ? (
                  <div className='topic-diagnostic topic-diagnostic-secondary'>
                    <span>
                      {inviteOutputLabel === 'grant'
                        ? 'Latest grant'
                        : inviteOutputLabel === 'share'
                          ? 'Latest share'
                          : 'Latest invite'}
                    </span>
                    <code>{inviteOutput}</code>
                  </div>
                ) : null}
                {activePrivateChannel ? (
                  <div className='topic-diagnostic topic-diagnostic-secondary'>
                    <span>
                      Policy:{' '}
                      {activePrivateChannel.audience_kind === 'friend_only'
                        ? 'Friends: only mutual followers can join'
                        : activePrivateChannel.audience_kind === 'friend_plus'
                          ? 'Friends+: participants can share to their mutuals'
                          : 'Invite only'}
                    </span>
                    <span>epoch: {activePrivateChannel.current_epoch_id}</span>
                    <span>sharing: {activePrivateChannel.sharing_state}</span>
                    {activePrivateChannel.joined_via_pubkey ? (
                      <span>joined via {shortPubkey(activePrivateChannel.joined_via_pubkey)}</span>
                    ) : null}
                  </div>
                ) : null}
                {activePrivateChannel &&
                (activePrivateChannel.audience_kind === 'friend_only' ||
                  activePrivateChannel.audience_kind === 'friend_plus') ? (
                  <div className='topic-diagnostic topic-diagnostic-secondary'>
                    <span>participants: {activePrivateChannel.participant_count}</span>
                    <span>stale: {activePrivateChannel.stale_participant_count}</span>
                    <span>owner: {activePrivateChannel.is_owner ? 'yes' : 'no'}</span>
                  </div>
                ) : null}
                {activePrivateChannel?.audience_kind === 'friend_only' &&
                activePrivateChannel.rotation_required ? (
                  <div className='topic-diagnostic topic-diagnostic-error'>
                    <span>rotation required: current participants include non-mutual followers</span>
                  </div>
                ) : null}
                {channelError ? <p className='error error-inline'>{channelError}</p> : null}
              </section>

              <section
                className='shell-section'
                ref={setPrimarySectionRef('live')}
                tabIndex={-1}
                onFocusCapture={() =>
                  setShellChromeState((current) => ({
                    ...current,
                    activePrimarySection: 'live',
                  }))
                }
              >
                <Card className='panel-subsection'>
                  <CardHeader>
                    <h3>Live Sessions</h3>
                    <small>{activeLiveSessions.length} active</small>
                  </CardHeader>
                  <form className='composer composer-compact' onSubmit={handleCreateLiveSession}>
                    <Label>
                      <span>Live Title</span>
                      <Input
                        value={liveTitle}
                        onChange={(event) => setLiveTitle(event.target.value)}
                        placeholder='Friday stream'
                      />
                    </Label>
                    <Label>
                      <span>Live Description</span>
                      <Textarea
                        value={liveDescription}
                        onChange={(event) => setLiveDescription(event.target.value)}
                        placeholder='short session summary'
                      />
                    </Label>
                    {liveError ? <p className='error error-inline'>{liveError}</p> : null}
                    <div className='topic-diagnostic topic-diagnostic-secondary'>
                      <span>Audience: {activeComposeAudienceLabel}</span>
                    </div>
                    <Button type='submit'>Start Live</Button>
                  </form>
                  {activeLiveSessions.length === 0 ? (
                    <p className='empty-state'>No live sessions</p>
                  ) : null}
                  <ul className='post-list'>
                    {activeLiveSessions.map((session) => {
                      const isOwner = session.host_pubkey === syncStatus.local_author_pubkey;
                      return (
                        <li key={session.session_id}>
                          <article className='post-card'>
                            <div className='post-meta'>
                              <span>{session.title}</span>
                              <span>{session.status}</span>
                              <span className='reply-chip'>{session.audience_label}</span>
                            </div>
                            <div className='post-body'>
                              <strong className='post-title'>
                                {session.description || 'no description'}
                              </strong>
                            </div>
                            <small>{session.session_id}</small>
                            <div className='topic-diagnostic topic-diagnostic-secondary'>
                              <span>viewers: {session.viewer_count}</span>
                              <span>
                                started:{' '}
                                {new Date(session.started_at).toLocaleTimeString('ja-JP')}
                              </span>
                            </div>
                            {session.ended_at ? (
                              <div className='topic-diagnostic topic-diagnostic-secondary'>
                                <span>
                                  ended: {new Date(session.ended_at).toLocaleTimeString('ja-JP')}
                                </span>
                              </div>
                            ) : null}
                            <div className='post-actions'>
                              {session.joined_by_me ? (
                                <Button
                                  variant='secondary'
                                  type='button'
                                  onClick={() => void handleLeaveLiveSession(session.session_id)}
                                >
                                  Leave
                                </Button>
                              ) : (
                                <Button
                                  variant='secondary'
                                  type='button'
                                  disabled={session.status === 'Ended'}
                                  onClick={() => void handleJoinLiveSession(session.session_id)}
                                >
                                  Join
                                </Button>
                              )}
                              {isOwner ? (
                                <Button
                                  variant='secondary'
                                  type='button'
                                  disabled={session.status === 'Ended'}
                                  onClick={() => void handleEndLiveSession(session.session_id)}
                                >
                                  End
                                </Button>
                              ) : null}
                            </div>
                          </article>
                        </li>
                      );
                    })}
                  </ul>
                </Card>
              </section>

              <section
                className='shell-section'
                ref={setPrimarySectionRef('game')}
                tabIndex={-1}
                onFocusCapture={() =>
                  setShellChromeState((current) => ({
                    ...current,
                    activePrimarySection: 'game',
                  }))
                }
              >
                <Card className='panel-subsection'>
                  <CardHeader>
                    <h3>Game Rooms</h3>
                    <small>{activeGameRooms.length} tracked</small>
                  </CardHeader>
                  <form className='composer composer-compact' onSubmit={handleCreateGameRoom}>
                    <Label>
                      <span>Game Title</span>
                      <Input
                        value={gameTitle}
                        onChange={(event) => setGameTitle(event.target.value)}
                        placeholder='Top 8 Finals'
                      />
                    </Label>
                    <Label>
                      <span>Game Description</span>
                      <Textarea
                        value={gameDescription}
                        onChange={(event) => setGameDescription(event.target.value)}
                        placeholder='match summary'
                      />
                    </Label>
                    <Label>
                      <span>Participants</span>
                      <Input
                        value={gameParticipantsInput}
                        onChange={(event) => setGameParticipantsInput(event.target.value)}
                        placeholder='Alice, Bob'
                      />
                    </Label>
                    {gameError ? <p className='error error-inline'>{gameError}</p> : null}
                    <div className='topic-diagnostic topic-diagnostic-secondary'>
                      <span>Audience: {activeComposeAudienceLabel}</span>
                    </div>
                    <Button type='submit'>Create Room</Button>
                  </form>
                  {activeGameRooms.length === 0 ? <p className='empty-state'>No game rooms</p> : null}
                  <ul className='post-list'>
                    {activeGameRooms.map((room) => {
                      const draft = gameDrafts[room.room_id] ?? createGameEditorDraft(room);
                      const isOwner = room.host_pubkey === syncStatus.local_author_pubkey;
                      return (
                        <li key={room.room_id}>
                          <article className='post-card'>
                            <div className='post-meta'>
                              <span>{room.title}</span>
                              <span>{room.status}</span>
                              <span className='reply-chip'>{room.audience_label}</span>
                            </div>
                            <div className='post-body'>
                              <strong className='post-title'>
                                {room.description || 'no description'}
                              </strong>
                            </div>
                            <small>{room.room_id}</small>
                            <div className='topic-diagnostic topic-diagnostic-secondary'>
                              <span>phase: {room.phase_label ?? 'none'}</span>
                              <span>
                                updated: {new Date(room.updated_at).toLocaleTimeString('ja-JP')}
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
                                        draft.scores[score.participant_id] ?? String(score.score)
                                      }
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
                            {isOwner ? (
                              <div className='composer composer-compact'>
                                <Label>
                                  <span>Status</span>
                                  <Select
                                    aria-label={`${room.room_id}-status`}
                                    value={draft.status}
                                    onChange={(event) =>
                                      updateGameDraft(room.room_id, (current) => ({
                                        ...current,
                                        status: event.target.value as GameRoomStatus,
                                      }))
                                    }
                                  >
                                    <option value='Waiting'>Waiting</option>
                                    <option value='Running'>Running</option>
                                    <option value='Paused'>Paused</option>
                                    <option value='Ended'>Ended</option>
                                  </Select>
                                </Label>
                                <Label>
                                  <span>Phase</span>
                                  <Input
                                    aria-label={`${room.room_id}-phase`}
                                    value={draft.phase_label}
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
                                  onClick={() => void handleUpdateGameRoom(room)}
                                >
                                  Save Room
                                </Button>
                              </div>
                            ) : null}
                          </article>
                        </li>
                      );
                    })}
                  </ul>
                </Card>
              </section>

            </Card>
          </div>
        }
        contextPane={
          <ContextPane
            paneId={SHELL_CONTEXT_ID}
            open={shellChromeState.contextOpen}
            onOpenChange={(open) => setContextOpen(open, !open)}
            activeMode={shellChromeState.activeContextPaneMode}
            onModeChange={(mode) =>
              setShellChromeState((current) => ({
                ...current,
                activeContextPaneMode: mode,
                contextOpen: true,
              }))
            }
            tabs={contextTabs}
          />
        }
      />

      <SettingsDrawer
        drawerId={SHELL_SETTINGS_ID}
        open={shellChromeState.settingsOpen}
        onOpenChange={(open) => setSettingsOpen(open, !open)}
        activeSection={shellChromeState.activeSettingsSection}
        onSectionChange={(section) =>
          setShellChromeState((current) => ({
            ...current,
            activeSettingsSection: section,
          }))
        }
        sections={settingsSections}
      />
    </>
  );
}
