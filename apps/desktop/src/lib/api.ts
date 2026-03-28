import { invoke } from '@tauri-apps/api/core';

export type TimelineCursor = {
  created_at: number;
  object_id: string;
};

export type ChannelRef =
  | { kind: 'public' }
  | { kind: 'private_channel'; channel_id: string };

export type TimelineScope =
  | { kind: 'public' }
  | { kind: 'all_joined' }
  | { kind: 'channel'; channel_id: string };

export type ChannelAudienceKind = 'invite_only' | 'friend_only' | 'friend_plus';
export type ChannelSharingState = 'open' | 'frozen';

export type PostView = {
  object_id: string;
  envelope_id: string;
  author_pubkey: string;
  author_name?: string | null;
  author_display_name?: string | null;
  following: boolean;
  followed_by: boolean;
  mutual: boolean;
  friend_of_friend: boolean;
  object_kind: string;
  content: string;
  content_status: BlobViewStatus;
  attachments: AttachmentView[];
  created_at: number;
  reply_to?: string | null;
  root_id?: string | null;
  published_topic_id?: string | null;
  origin_topic_id?: string | null;
  repost_of?: RepostSourceView | null;
  repost_commentary?: string | null;
  is_threadable?: boolean;
  channel_id?: string | null;
  audience_label: string;
  reaction_summary?: ReactionSummaryView[];
  my_reactions?: ReactionKeyView[];
};

export type CustomReactionAssetView = {
  asset_id: string;
  owner_pubkey: string;
  blob_hash: string;
  mime: string;
  bytes: number;
  width: number;
  height: number;
};

export type BookmarkedCustomReactionView = CustomReactionAssetView;

export type ReactionKeyView = {
  reaction_key_kind: 'emoji' | 'custom_asset' | string;
  normalized_reaction_key: string;
  emoji?: string | null;
  custom_asset?: CustomReactionAssetView | null;
};

export type ReactionSummaryView = ReactionKeyView & {
  count: number;
};

export type ReactionStateView = {
  target_object_id: string;
  source_replica_id: string;
  reaction_summary: ReactionSummaryView[];
  my_reactions: ReactionKeyView[];
};

export type ReactionKeyInput =
  | { kind: 'emoji'; emoji: string }
  | { kind: 'custom_asset'; asset: CustomReactionAssetView };

export type CustomReactionCropRect = {
  x: number;
  y: number;
  size: number;
};

export type RepostSourceView = {
  source_object_id: string;
  source_topic_id: string;
  source_author_pubkey: string;
  source_author_name?: string | null;
  source_author_display_name?: string | null;
  source_object_kind: string;
  content: string;
  attachments: AttachmentView[];
  reply_to?: string | null;
  root_id?: string | null;
};

export type Profile = {
  pubkey: string;
  name?: string | null;
  display_name?: string | null;
  about?: string | null;
  picture?: string | null;
  updated_at: number;
};

export type ProfileInput = {
  name?: string | null;
  display_name?: string | null;
  about?: string | null;
  picture?: string | null;
};

export type AuthorSocialView = {
  author_pubkey: string;
  name?: string | null;
  display_name?: string | null;
  about?: string | null;
  picture?: string | null;
  updated_at?: number | null;
  following: boolean;
  followed_by: boolean;
  mutual: boolean;
  friend_of_friend: boolean;
  friend_of_friend_via_pubkeys: string[];
};

export type BlobViewStatus = 'Missing' | 'Available' | 'Pinned';

export type AttachmentView = {
  hash: string;
  mime: string;
  bytes: number;
  role: string;
  status: BlobViewStatus;
};

export type CreateAttachmentInput = {
  file_name?: string | null;
  mime: string;
  byte_size: number;
  data_base64: string;
  role?: string | null;
};

export type CreateRepostInput = {
  topic: string;
  source_topic: string;
  source_object_id: string;
  commentary?: string | null;
};

export type BlobMediaPayload = {
  bytes_base64: string;
  mime: string;
};

export type TimelineView = {
  items: PostView[];
  next_cursor?: TimelineCursor | null;
};

export type DiscoveryMode = 'static_peer' | 'seeded_dht';

export type ConnectMode = 'direct_only' | 'direct_or_relay';

export type SeedPeer = {
  endpoint_id: string;
  addr_hint?: string | null;
};

export type DiscoveryConfig = {
  mode: DiscoveryMode;
  connect_mode: ConnectMode;
  env_locked: boolean;
  seed_peers: SeedPeer[];
};

export type DiscoveryStatus = {
  mode: DiscoveryMode;
  connect_mode: ConnectMode;
  env_locked: boolean;
  configured_seed_peer_ids: string[];
  bootstrap_seed_peer_ids: string[];
  manual_ticket_peer_ids: string[];
  connected_peer_ids: string[];
  assist_peer_ids: string[];
  local_endpoint_id: string;
  last_discovery_error?: string | null;
};

export type SyncStatus = {
  connected: boolean;
  last_sync_ts?: number | null;
  peer_count: number;
  pending_events: number;
  status_detail: string;
  last_error?: string | null;
  configured_peers: string[];
  subscribed_topics: string[];
  topic_diagnostics: TopicSyncStatus[];
  local_author_pubkey: string;
  discovery: DiscoveryStatus;
};

export type CommunityNodeResolvedUrls = {
  public_base_url: string;
  connectivity_urls: string[];
  seed_peers?: SeedPeer[];
};

export type CommunityNodeNodeConfig = {
  base_url: string;
  resolved_urls?: CommunityNodeResolvedUrls | null;
};

export type CommunityNodeConfig = {
  nodes: CommunityNodeNodeConfig[];
};

export type CommunityNodeAuthState = {
  authenticated: boolean;
  expires_at?: number | null;
};

export type CommunityNodeConsentItem = {
  policy_slug: string;
  policy_version: number;
  title: string;
  required: boolean;
  accepted_at?: number | null;
};

export type CommunityNodeConsentStatus = {
  all_required_accepted: boolean;
  items: CommunityNodeConsentItem[];
};

export type CommunityNodeNodeStatus = {
  base_url: string;
  auth_state: CommunityNodeAuthState;
  consent_state?: CommunityNodeConsentStatus | null;
  resolved_urls?: CommunityNodeResolvedUrls | null;
  last_error?: string | null;
  restart_required: boolean;
};

export type TopicSyncStatus = {
  topic: string;
  joined: boolean;
  peer_count: number;
  connected_peers: string[];
  assist_peer_ids: string[];
  configured_peer_ids: string[];
  missing_peer_ids: string[];
  last_received_at?: number | null;
  status_detail: string;
  last_error?: string | null;
};

export type LiveSessionStatus = 'Scheduled' | 'Live' | 'Paused' | 'Ended';

export type LiveSessionView = {
  session_id: string;
  host_pubkey: string;
  title: string;
  description: string;
  status: LiveSessionStatus;
  started_at: number;
  ended_at?: number | null;
  viewer_count: number;
  joined_by_me: boolean;
  channel_id?: string | null;
  audience_label: string;
};

export type GameRoomStatus = 'Waiting' | 'Running' | 'Paused' | 'Ended';

export type GameScoreView = {
  participant_id: string;
  label: string;
  score: number;
};

export type GameRoomView = {
  room_id: string;
  host_pubkey: string;
  title: string;
  description: string;
  status: GameRoomStatus;
  phase_label?: string | null;
  scores: GameScoreView[];
  updated_at: number;
  channel_id?: string | null;
  audience_label: string;
};

export type JoinedPrivateChannelView = {
  topic_id: string;
  channel_id: string;
  label: string;
  creator_pubkey: string;
  owner_pubkey: string;
  joined_via_pubkey?: string | null;
  audience_kind: ChannelAudienceKind;
  is_owner: boolean;
  current_epoch_id: string;
  archived_epoch_ids: string[];
  sharing_state: ChannelSharingState;
  rotation_required: boolean;
  participant_count: number;
  stale_participant_count: number;
};

export type PrivateChannelInvitePreview = {
  channel_id: string;
  topic_id: string;
  channel_label: string;
  inviter_pubkey: string;
  expires_at?: number | null;
  namespace_secret_hex: string;
};

export type FriendOnlyGrantPreview = {
  channel_id: string;
  topic_id: string;
  channel_label: string;
  owner_pubkey: string;
  epoch_id: string;
  expires_at?: number | null;
  namespace_secret_hex: string;
};

export type FriendPlusSharePreview = {
  channel_id: string;
  topic_id: string;
  channel_label: string;
  owner_pubkey: string;
  sponsor_pubkey: string;
  epoch_id: string;
  expires_at?: number | null;
  namespace_secret_hex: string;
  share_token_id: string;
};

export interface DesktopApi {
  createPost(
    topic: string,
    content: string,
    replyTo?: string | null,
    attachments?: CreateAttachmentInput[],
    channelRef?: ChannelRef
  ): Promise<string>;
  createRepost(
    topic: string,
    sourceTopic: string,
    sourceObjectId: string,
    commentary?: string | null
  ): Promise<string>;
  toggleReaction(
    targetTopicId: string,
    targetObjectId: string,
    reactionKey: ReactionKeyInput,
    channelRef?: ChannelRef | null
  ): Promise<ReactionStateView>;
  listMyCustomReactionAssets(): Promise<CustomReactionAssetView[]>;
  createCustomReactionAsset(
    upload: CreateAttachmentInput,
    cropRect: CustomReactionCropRect
  ): Promise<CustomReactionAssetView>;
  listBookmarkedCustomReactions(): Promise<BookmarkedCustomReactionView[]>;
  bookmarkCustomReaction(asset: CustomReactionAssetView): Promise<BookmarkedCustomReactionView>;
  removeBookmarkedCustomReaction(assetId: string): Promise<void>;
  listTimeline(
    topic: string,
    cursor?: TimelineCursor | null,
    limit?: number,
    scope?: TimelineScope
  ): Promise<TimelineView>;
  listThread(
    topic: string,
    threadId: string,
    cursor?: TimelineCursor | null,
    limit?: number
  ): Promise<TimelineView>;
  listProfileTimeline(
    pubkey: string,
    cursor?: TimelineCursor | null,
    limit?: number
  ): Promise<TimelineView>;
  getMyProfile(): Promise<Profile>;
  setMyProfile(input: ProfileInput): Promise<Profile>;
  followAuthor(pubkey: string): Promise<AuthorSocialView>;
  unfollowAuthor(pubkey: string): Promise<AuthorSocialView>;
  getAuthorSocialView(pubkey: string): Promise<AuthorSocialView>;
  listLiveSessions(topic: string, scope?: TimelineScope): Promise<LiveSessionView[]>;
  createLiveSession(
    topic: string,
    title: string,
    description: string,
    channelRef?: ChannelRef
  ): Promise<string>;
  endLiveSession(topic: string, sessionId: string): Promise<void>;
  joinLiveSession(topic: string, sessionId: string): Promise<void>;
  leaveLiveSession(topic: string, sessionId: string): Promise<void>;
  listGameRooms(topic: string, scope?: TimelineScope): Promise<GameRoomView[]>;
  createGameRoom(
    topic: string,
    title: string,
    description: string,
    participants: string[],
    channelRef?: ChannelRef
  ): Promise<string>;
  createPrivateChannel(
    topic: string,
    label: string,
    audienceKind?: ChannelAudienceKind
  ): Promise<JoinedPrivateChannelView>;
  exportPrivateChannelInvite(
    topic: string,
    channelId: string,
    expiresAt?: number | null
  ): Promise<string>;
  importPrivateChannelInvite(token: string): Promise<PrivateChannelInvitePreview>;
  exportFriendOnlyGrant(
    topic: string,
    channelId: string,
    expiresAt?: number | null
  ): Promise<string>;
  importFriendOnlyGrant(token: string): Promise<FriendOnlyGrantPreview>;
  exportFriendPlusShare(
    topic: string,
    channelId: string,
    expiresAt?: number | null
  ): Promise<string>;
  importFriendPlusShare(token: string): Promise<FriendPlusSharePreview>;
  freezePrivateChannel(topic: string, channelId: string): Promise<JoinedPrivateChannelView>;
  rotatePrivateChannel(topic: string, channelId: string): Promise<JoinedPrivateChannelView>;
  listJoinedPrivateChannels(topic: string): Promise<JoinedPrivateChannelView[]>;
  updateGameRoom(
    topic: string,
    roomId: string,
    status: GameRoomStatus,
    phaseLabel: string | null,
    scores: GameScoreView[]
  ): Promise<void>;
  getSyncStatus(): Promise<SyncStatus>;
  getDiscoveryConfig(): Promise<DiscoveryConfig>;
  getCommunityNodeConfig(): Promise<CommunityNodeConfig>;
  getCommunityNodeStatuses(): Promise<CommunityNodeNodeStatus[]>;
  setCommunityNodeConfig(baseUrls: string[]): Promise<CommunityNodeConfig>;
  clearCommunityNodeConfig(): Promise<void>;
  authenticateCommunityNode(baseUrl: string): Promise<CommunityNodeNodeStatus>;
  clearCommunityNodeToken(baseUrl: string): Promise<CommunityNodeNodeStatus>;
  getCommunityNodeConsentStatus(baseUrl: string): Promise<CommunityNodeNodeStatus>;
  acceptCommunityNodeConsents(
    baseUrl: string,
    policySlugs: string[]
  ): Promise<CommunityNodeNodeStatus>;
  refreshCommunityNodeMetadata(baseUrl: string): Promise<CommunityNodeNodeStatus>;
  importPeerTicket(ticket: string): Promise<void>;
  setDiscoverySeeds(seedEntries: string[]): Promise<DiscoveryConfig>;
  unsubscribeTopic(topic: string): Promise<void>;
  getLocalPeerTicket(): Promise<string | null>;
  getBlobMediaPayload(hash: string, mime: string): Promise<BlobMediaPayload | null>;
  getBlobPreviewUrl(hash: string, mime: string): Promise<string | null>;
}

declare global {
  interface Window {
    __KUKURI_DESKTOP__?: DesktopApi;
  }
}

const BACKEND_UNAVAILABLE_MESSAGE = 'Desktop backend is not attached.';

function normalizeInvokeError(error: unknown): Error {
  const normalized =
    error instanceof Error
      ? error
      : typeof error === 'string'
        ? new Error(error)
        : typeof error === 'object' &&
            error !== null &&
            'message' in error &&
            typeof error.message === 'string'
          ? new Error(error.message)
          : new Error(BACKEND_UNAVAILABLE_MESSAGE);
  const message = normalized.message.toLowerCase();
  if (
    message.includes('__tauri') ||
    message.includes('__tauri_ipc__') ||
    (message.includes('ipc') && message.includes('not available')) ||
    (message.includes('invoke') && message.includes('undefined'))
  ) {
    return new Error(BACKEND_UNAVAILABLE_MESSAGE);
  }
  return normalized;
}

async function invokeDesktop<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw normalizeInvokeError(error);
  }
}

export const runtimeApi: DesktopApi = {
  createPost: async (topic, content, replyTo, attachments = [], channelRef = { kind: 'public' }) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.createPost(topic, content, replyTo, attachments, channelRef);
    }
    return invokeDesktop<string>('create_post', {
      request: {
        topic,
        content,
        reply_to: replyTo,
        channel_ref: channelRef,
        attachments,
      },
    });
  },
  createRepost: async (topic, sourceTopic, sourceObjectId, commentary) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.createRepost(
        topic,
        sourceTopic,
        sourceObjectId,
        commentary
      );
    }
    return invokeDesktop<string>('create_repost', {
      request: {
        topic,
        source_topic: sourceTopic,
        source_object_id: sourceObjectId,
        commentary,
      },
    });
  },
  toggleReaction: async (targetTopicId, targetObjectId, reactionKey, channelRef = null) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.toggleReaction(
        targetTopicId,
        targetObjectId,
        reactionKey,
        channelRef
      );
    }
    return invokeDesktop<ReactionStateView>('toggle_reaction', {
      request: {
        target_topic_id: targetTopicId,
        target_object_id: targetObjectId,
        reaction_key:
          reactionKey.kind === 'emoji'
            ? { kind: 'emoji', emoji: reactionKey.emoji }
            : {
                kind: 'custom_asset',
                asset_id: reactionKey.asset.asset_id,
                owner_pubkey: reactionKey.asset.owner_pubkey,
                blob_hash: reactionKey.asset.blob_hash,
                mime: reactionKey.asset.mime,
                bytes: reactionKey.asset.bytes,
                width: reactionKey.asset.width,
                height: reactionKey.asset.height,
              },
        channel_ref: channelRef,
      },
    });
  },
  listMyCustomReactionAssets: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listMyCustomReactionAssets();
    }
    return invokeDesktop<CustomReactionAssetView[]>('list_my_custom_reaction_assets');
  },
  createCustomReactionAsset: async (upload, cropRect) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.createCustomReactionAsset(upload, cropRect);
    }
    return invokeDesktop<CustomReactionAssetView>('create_custom_reaction_asset', {
      request: {
        upload,
        crop_rect: cropRect,
      },
    });
  },
  listBookmarkedCustomReactions: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listBookmarkedCustomReactions();
    }
    return invokeDesktop<BookmarkedCustomReactionView[]>('list_bookmarked_custom_reactions');
  },
  bookmarkCustomReaction: async (asset) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.bookmarkCustomReaction(asset);
    }
    return invokeDesktop<BookmarkedCustomReactionView>('bookmark_custom_reaction', {
      request: {
        asset_id: asset.asset_id,
        owner_pubkey: asset.owner_pubkey,
        blob_hash: asset.blob_hash,
        mime: asset.mime,
        bytes: asset.bytes,
        width: asset.width,
        height: asset.height,
      },
    });
  },
  removeBookmarkedCustomReaction: async (assetId) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.removeBookmarkedCustomReaction(assetId);
    }
    return invokeDesktop<void>('remove_bookmarked_custom_reaction', {
      request: {
        asset_id: assetId,
      },
    });
  },
  listTimeline: async (topic, cursor, limit, scope = { kind: 'public' }) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listTimeline(topic, cursor, limit, scope);
    }
    return invokeDesktop<TimelineView>('list_timeline', {
      request: {
        topic,
        scope,
        cursor,
        limit,
      },
    });
  },
  listThread: async (topic, threadId, cursor, limit) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listThread(topic, threadId, cursor, limit);
    }
    return invokeDesktop<TimelineView>('list_thread', {
      request: {
        topic,
        thread_id: threadId,
        cursor,
        limit,
      },
    });
  },
  listProfileTimeline: async (pubkey, cursor, limit) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listProfileTimeline(pubkey, cursor, limit);
    }
    return invokeDesktop<TimelineView>('list_profile_timeline', {
      request: {
        pubkey,
        cursor,
        limit,
      },
    });
  },
  getMyProfile: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getMyProfile();
    }
    return invokeDesktop<Profile>('get_my_profile');
  },
  setMyProfile: async (input) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.setMyProfile(input);
    }
    return invokeDesktop<Profile>('set_my_profile', {
      request: input,
    });
  },
  followAuthor: async (pubkey) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.followAuthor(pubkey);
    }
    return invokeDesktop<AuthorSocialView>('follow_author', {
      request: { pubkey },
    });
  },
  unfollowAuthor: async (pubkey) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.unfollowAuthor(pubkey);
    }
    return invokeDesktop<AuthorSocialView>('unfollow_author', {
      request: { pubkey },
    });
  },
  getAuthorSocialView: async (pubkey) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getAuthorSocialView(pubkey);
    }
    return invokeDesktop<AuthorSocialView>('get_author_social_view', {
      request: { pubkey },
    });
  },
  listLiveSessions: async (topic, scope = { kind: 'public' }) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listLiveSessions(topic, scope);
    }
    return invokeDesktop<LiveSessionView[]>('list_live_sessions', {
      request: {
        topic,
        scope,
      },
    });
  },
  createLiveSession: async (topic, title, description, channelRef = { kind: 'public' }) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.createLiveSession(topic, title, description, channelRef);
    }
    return invokeDesktop<string>('create_live_session', {
      request: {
        topic,
        channel_ref: channelRef,
        title,
        description,
      },
    });
  },
  endLiveSession: async (topic, sessionId) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.endLiveSession(topic, sessionId);
    }
    return invokeDesktop<void>('end_live_session', {
      request: {
        topic,
        session_id: sessionId,
      },
    });
  },
  joinLiveSession: async (topic, sessionId) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.joinLiveSession(topic, sessionId);
    }
    return invokeDesktop<void>('join_live_session', {
      request: {
        topic,
        session_id: sessionId,
      },
    });
  },
  leaveLiveSession: async (topic, sessionId) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.leaveLiveSession(topic, sessionId);
    }
    return invokeDesktop<void>('leave_live_session', {
      request: {
        topic,
        session_id: sessionId,
      },
    });
  },
  listGameRooms: async (topic, scope = { kind: 'public' }) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listGameRooms(topic, scope);
    }
    return invokeDesktop<GameRoomView[]>('list_game_rooms', {
      request: {
        topic,
        scope,
      },
    });
  },
  createGameRoom: async (
    topic,
    title,
    description,
    participants,
    channelRef = { kind: 'public' }
  ) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.createGameRoom(
        topic,
        title,
        description,
        participants,
        channelRef
      );
    }
    return invokeDesktop<string>('create_game_room', {
      request: {
        topic,
        channel_ref: channelRef,
        title,
        description,
        participants,
      },
    });
  },
  createPrivateChannel: async (topic, label, audienceKind = 'invite_only') => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.createPrivateChannel(topic, label, audienceKind);
    }
    return invokeDesktop<JoinedPrivateChannelView>('create_private_channel', {
      request: { topic, label, audience_kind: audienceKind },
    });
  },
  exportPrivateChannelInvite: async (topic, channelId, expiresAt = null) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.exportPrivateChannelInvite(topic, channelId, expiresAt);
    }
    return invokeDesktop<string>('export_private_channel_invite', {
      request: {
        topic,
        channel_id: channelId,
        expires_at: expiresAt,
      },
    });
  },
  importPrivateChannelInvite: async (token) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.importPrivateChannelInvite(token);
    }
    return invokeDesktop<PrivateChannelInvitePreview>('import_private_channel_invite', {
      request: { token },
    });
  },
  exportFriendOnlyGrant: async (topic, channelId, expiresAt = null) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.exportFriendOnlyGrant(topic, channelId, expiresAt);
    }
    return invokeDesktop<string>('export_friend_only_grant', {
      request: {
        topic,
        channel_id: channelId,
        expires_at: expiresAt,
      },
    });
  },
  importFriendOnlyGrant: async (token) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.importFriendOnlyGrant(token);
    }
    return invokeDesktop<FriendOnlyGrantPreview>('import_friend_only_grant', {
      request: { token },
    });
  },
  exportFriendPlusShare: async (topic, channelId, expiresAt = null) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.exportFriendPlusShare(topic, channelId, expiresAt);
    }
    return invokeDesktop<string>('export_friend_plus_share', {
      request: {
        topic,
        channel_id: channelId,
        expires_at: expiresAt,
      },
    });
  },
  importFriendPlusShare: async (token) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.importFriendPlusShare(token);
    }
    return invokeDesktop<FriendPlusSharePreview>('import_friend_plus_share', {
      request: { token },
    });
  },
  freezePrivateChannel: async (topic, channelId) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.freezePrivateChannel(topic, channelId);
    }
    return invokeDesktop<JoinedPrivateChannelView>('freeze_private_channel', {
      request: {
        topic,
        channel_id: channelId,
      },
    });
  },
  rotatePrivateChannel: async (topic, channelId) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.rotatePrivateChannel(topic, channelId);
    }
    return invokeDesktop<JoinedPrivateChannelView>('rotate_private_channel', {
      request: {
        topic,
        channel_id: channelId,
      },
    });
  },
  listJoinedPrivateChannels: async (topic) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listJoinedPrivateChannels(topic);
    }
    return invokeDesktop<JoinedPrivateChannelView[]>('list_joined_private_channels', {
      request: { topic },
    });
  },
  updateGameRoom: async (topic, roomId, status, phaseLabel, scores) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.updateGameRoom(topic, roomId, status, phaseLabel, scores);
    }
    return invokeDesktop<void>('update_game_room', {
      request: {
        topic,
        room_id: roomId,
        status,
        phase_label: phaseLabel,
        scores,
      },
    });
  },
  getSyncStatus: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getSyncStatus();
    }
    return invokeDesktop<SyncStatus>('get_sync_status');
  },
  getDiscoveryConfig: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getDiscoveryConfig();
    }
    return invokeDesktop<DiscoveryConfig>('get_discovery_config');
  },
  getCommunityNodeConfig: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getCommunityNodeConfig();
    }
    return invokeDesktop<CommunityNodeConfig>('get_community_node_config');
  },
  getCommunityNodeStatuses: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getCommunityNodeStatuses();
    }
    return invokeDesktop<CommunityNodeNodeStatus[]>('get_community_node_statuses');
  },
  setCommunityNodeConfig: async (baseUrls) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.setCommunityNodeConfig(baseUrls);
    }
    return invokeDesktop<CommunityNodeConfig>('set_community_node_config', {
      request: {
        base_urls: baseUrls,
      },
    });
  },
  clearCommunityNodeConfig: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.clearCommunityNodeConfig();
    }
    return invokeDesktop<void>('clear_community_node_config');
  },
  authenticateCommunityNode: async (baseUrl) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.authenticateCommunityNode(baseUrl);
    }
    return invokeDesktop<CommunityNodeNodeStatus>('authenticate_community_node', {
      request: {
        base_url: baseUrl,
      },
    });
  },
  clearCommunityNodeToken: async (baseUrl) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.clearCommunityNodeToken(baseUrl);
    }
    return invokeDesktop<CommunityNodeNodeStatus>('clear_community_node_token', {
      request: {
        base_url: baseUrl,
      },
    });
  },
  getCommunityNodeConsentStatus: async (baseUrl) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getCommunityNodeConsentStatus(baseUrl);
    }
    return invokeDesktop<CommunityNodeNodeStatus>('get_community_node_consent_status', {
      request: {
        base_url: baseUrl,
      },
    });
  },
  acceptCommunityNodeConsents: async (baseUrl, policySlugs) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.acceptCommunityNodeConsents(baseUrl, policySlugs);
    }
    return invokeDesktop<CommunityNodeNodeStatus>('accept_community_node_consents', {
      request: {
        base_url: baseUrl,
        policy_slugs: policySlugs,
      },
    });
  },
  refreshCommunityNodeMetadata: async (baseUrl) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.refreshCommunityNodeMetadata(baseUrl);
    }
    return invokeDesktop<CommunityNodeNodeStatus>('refresh_community_node_metadata', {
      request: {
        base_url: baseUrl,
      },
    });
  },
  importPeerTicket: async (ticket) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.importPeerTicket(ticket);
    }
    return invokeDesktop<void>('import_peer_ticket', {
      request: {
        ticket,
      },
    });
  },
  setDiscoverySeeds: async (seedEntries) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.setDiscoverySeeds(seedEntries);
    }
    return invokeDesktop<DiscoveryConfig>('set_discovery_seeds', {
      request: {
        seed_entries: seedEntries,
      },
    });
  },
  unsubscribeTopic: async (topic) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.unsubscribeTopic(topic);
    }
    return invokeDesktop<void>('unsubscribe_topic', {
      request: {
        topic,
      },
    });
  },
  getLocalPeerTicket: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getLocalPeerTicket();
    }
    return invokeDesktop<string | null>('get_local_peer_ticket');
  },
  getBlobMediaPayload: async (hash, mime) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getBlobMediaPayload(hash, mime);
    }
    return invokeDesktop<BlobMediaPayload | null>('get_blob_media_payload', {
      request: {
        hash,
        mime,
      },
    });
  },
  getBlobPreviewUrl: async (hash, mime) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getBlobPreviewUrl(hash, mime);
    }
    return invokeDesktop<string | null>('get_blob_preview_url', {
      request: {
        hash,
        mime,
      },
    });
  },
};
