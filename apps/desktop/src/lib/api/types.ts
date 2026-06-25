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

export type DesktopStartupErrorKind = 'database_open' | 'database_migration' | 'unknown';

export type DesktopStartupErrorView = {
  kind: DesktopStartupErrorKind;
  message: string;
  detail: string;
  db_path?: string | null;
};

export type DesktopStartupStatus =
  | { status: 'ready' }
  | {
      status: 'consent_required';
      current_bundle_version: number;
      accepted_bundle_version: number | null;
    }
  | { status: 'failed'; error: DesktopStartupErrorView };

export type AppConsentStatus = {
  currentBundleVersion: number;
  acceptedBundleVersion: number | null;
  acceptedAt: number | null;
  satisfied: boolean;
};

export type ChannelAudienceKind = 'invite_only' | 'friend_only' | 'friend_plus';
export type ChannelSharingState = 'open' | 'frozen';

export type LocalPostDraft = {
  kind: 'post' | 'repost';
  topic: string;
  content: string;
  reply_to?: string | null;
  source_topic?: string | null;
  source_object_id?: string | null;
  channel_ref?: ChannelRef | null;
  attachments?: CreateAttachmentInput[];
};

export type LocalDraftMediaItem = {
  id: string;
  source_name: string;
  preview_url: string;
  attachments: CreateAttachmentInput[];
};

export type PostView = {
  object_id: string;
  envelope_id: string;
  author_pubkey: string;
  author_name?: string | null;
  author_display_name?: string | null;
  author_picture?: string | null;
  author_picture_asset?: ProfileAssetView | null;
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
  reply_preview?: ReplyPreviewView | null;
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
  local_id?: string | null;
  local_state?: 'pending' | 'syncing' | 'failed' | null;
  local_error?: string | null;
  server_object_id?: string | null;
  local_draft?: LocalPostDraft | null;
  local_draft_media_items?: LocalDraftMediaItem[] | null;
};

export type ReplyPreviewAuthorView = {
  pubkey: string;
  name?: string | null;
  display_name?: string | null;
  picture?: string | null;
  picture_asset?: ProfileAssetView | null;
};

export type ReplyPreviewView = {
  object_id: string;
  topic: string;
  author: ReplyPreviewAuthorView;
  content: string;
  attachments: AttachmentView[];
  root_id?: string | null;
  reply_to?: string | null;
};

export type CustomReactionAssetView = {
  asset_id: string;
  owner_pubkey: string;
  blob_hash: string;
  search_key: string;
  mime: string;
  bytes: number;
  width: number;
  height: number;
};

export type BookmarkedCustomReactionView = CustomReactionAssetView;

export type BookmarkedPostView = {
  bookmarked_at: number;
  post: PostView;
};

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

export type RecentReactionView = ReactionKeyView & {
  updated_at: number;
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
  source_author_picture?: string | null;
  source_author_picture_asset?: ProfileAssetView | null;
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
  picture_asset?: ProfileAssetView | null;
  updated_at: number;
};

export type ProfileInput = {
  name?: string | null;
  display_name?: string | null;
  about?: string | null;
  picture?: string | null;
  picture_upload?: CreateAttachmentInput | null;
  clear_picture?: boolean;
};

export type ProfileAssetView = {
  hash: string;
  mime: string;
  bytes: number;
  role: 'profile_avatar';
};

export type AuthorSocialView = {
  author_pubkey: string;
  name?: string | null;
  display_name?: string | null;
  about?: string | null;
  picture?: string | null;
  picture_asset?: ProfileAssetView | null;
  updated_at?: number | null;
  following: boolean;
  followed_by: boolean;
  mutual: boolean;
  friend_of_friend: boolean;
  friend_of_friend_via_pubkeys: string[];
  muted: boolean;
};

export type SocialConnectionKind = 'following' | 'followed' | 'muted';

export type DirectMessageStatusView = {
  peer_pubkey: string;
  dm_id: string;
  mutual: boolean;
  send_enabled: boolean;
  peer_count: number;
  pending_outbox_count: number;
};

export type DirectMessageMessageView = {
  dm_id: string;
  message_id: string;
  sender_pubkey: string;
  recipient_pubkey: string;
  created_at: number;
  text: string;
  reply_to_message_id?: string | null;
  attachments: AttachmentView[];
  outgoing: boolean;
  delivered: boolean;
};

export type DirectMessageConversationView = {
  dm_id: string;
  peer_pubkey: string;
  peer_name?: string | null;
  peer_display_name?: string | null;
  peer_picture?: string | null;
  peer_picture_asset?: ProfileAssetView | null;
  updated_at: number;
  last_message_at?: number | null;
  last_message_id?: string | null;
  last_message_preview?: string | null;
  status: DirectMessageStatusView;
};

export type NotificationKind =
  | 'mention'
  | 'reply'
  | 'repost'
  | 'quote_repost'
  | 'direct_message'
  | 'followed';

export type NotificationView = {
  notification_id: string;
  kind: NotificationKind;
  actor_pubkey: string;
  actor_name?: string | null;
  actor_display_name?: string | null;
  actor_picture?: string | null;
  actor_picture_asset?: ProfileAssetView | null;
  source_envelope_id?: string | null;
  source_replica_id?: string | null;
  topic_id?: string | null;
  channel_id?: string | null;
  object_id?: string | null;
  thread_root_object_id?: string | null;
  dm_id?: string | null;
  message_id?: string | null;
  preview_text?: string | null;
  created_at: number;
  received_at: number;
  read_at?: number | null;
};

export type NotificationStatusView = {
  unread_count: number;
};

export type DirectMessageTimelineView = {
  items: DirectMessageMessageView[];
  next_cursor?: TimelineCursor | null;
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

export type ConnectionPath =
  | 'direct_p2p'
  | 'relay_supported_p2p'
  | 'relay_fallback';

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
  active_path: ConnectionPath;
  fallback_peer_ids: string[];
  env_locked: boolean;
  configured_seed_peer_ids: string[];
  bootstrap_seed_peer_ids: string[];
  manual_ticket_peer_ids: string[];
  connected_peer_ids: string[];
  docs_assist_peer_ids: string[];
  blob_assist_peer_ids: string[];
  local_endpoint_id: string;
  last_discovery_error?: string | null;
};

export type DeliveryState = 'Live' | 'DurableRecovering' | 'DurableReady' | 'Offline';

export type SyncStatus = {
  connected: boolean;
  delivery_state: DeliveryState;
  last_sync_ts?: number | null;
  peer_count: number;
  pending_events: number;
  status_detail: string;
  last_error?: string | null;
  configured_peers: string[];
  subscribed_topics: string[];
  active_path: ConnectionPath;
  fallback_peer_ids: string[];
  topic_diagnostics: TopicSyncStatus[];
  local_author_pubkey: string;
  discovery: DiscoveryStatus;
  gossip_disabled_topics: string[];
  gossip_disabled_channels: string[];
};

export type CommunityNodeResolvedUrls = {
  public_base_url: string;
  connectivity_urls: string[];
  seed_peers?: SeedPeer[];
};

export type CommunityNodeNodeConfig = {
  base_url: string;
  auto_approve?: boolean;
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

export type CommunityNodeSessionPhase =
  | 'idle'
  | 'connecting'
  | 'authenticating'
  | 'accepting'
  | 'refreshing'
  | 'ready'
  | 'retrying';

export type CommunityNodeNodeStatus = {
  base_url: string;
  auto_approve?: boolean;
  auth_state: CommunityNodeAuthState;
  consent_state?: CommunityNodeConsentStatus | null;
  resolved_urls?: CommunityNodeResolvedUrls | null;
  last_error?: string | null;
  session_phase?: CommunityNodeSessionPhase;
  retry_after?: number | null;
  restart_required: boolean;
};

export type CommunityNodeConfigInput = {
  base_url: string;
  auto_approve: boolean;
};

// community node manifest (#355/#356) の client 側表現。public manifest endpoint から取得し、
// dependency 表示 (#357) に使う。snake_case は Rust 由来の JSON 形状に合わせる。
export type CommunityNodeCapabilityScope = {
  available_enabled: string[];
  planned_enabled: string[];
};

export type CommunityNodeAuthorityScope = {
  applies_to: string[];
  does_not_apply_to: string[];
};

export type CommunityNodeP2pBoundary = {
  identity_authority: boolean;
  profile_canonical_store: boolean;
  social_graph_canonical_store: boolean;
  content_truth_source: boolean;
  network_wide_authority: boolean;
};

export type CommunityNodeManifest = {
  node_id: string;
  node_name: string;
  node_role: string;
  server_name: string;
  manifest_version: string;
  capability_scope: CommunityNodeCapabilityScope;
  authority_scope: CommunityNodeAuthorityScope;
  p2p_boundary: CommunityNodeP2pBoundary;
  abuse_contact: string;
  // node が公開する通報受付 endpoint (#310)。未公開なら空文字。
  // client は空なら abuse_contact を mailto / copyable contact として案内する。
  report_endpoint: string;
  terms_url: string;
  privacy_url: string;
  moderation_policy_url: string;
};

// manifest fetch 状態。'ok' は取得成功、'absent' は node が未公開 (404)。
// 'error' は fetch 失敗、'loading' は取得中（client 側で付与）。
export type CommunityNodeManifestFetchStatus = 'ok' | 'absent';

export type CommunityNodeManifestFetch = {
  status: CommunityNodeManifestFetchStatus;
  manifest?: CommunityNodeManifest | null;
};

// 分散通報ルーティング (#310) の送信リクエスト。通報先は client が provenance + manifest
// から解決し、その report_endpoint を載せて渡す。snake_case は Rust 由来の JSON 形状。
export type SubmitCommunityNodeReportRequest = {
  node_base_url: string;
  report_endpoint: string;
  subject_kind: string;
  subject_id: string;
  capability: string;
  reason: string;
  details?: string | null;
  reporter_contact?: string | null;
};

export type SubmitCommunityNodeReportStatus = 'submitted';

export type SubmitCommunityNodeReportResult = {
  status: SubmitCommunityNodeReportStatus;
  reference_id?: string | null;
};

export type TopicSyncStatus = {
  topic: string;
  joined: boolean;
  delivery_state: DeliveryState;
  peer_count: number;
  connected_peers: string[];
  docs_assist_peer_ids: string[];
  configured_peer_ids: string[];
  missing_peer_ids: string[];
  active_path: ConnectionPath;
  rendezvous_peer_ids: string[];
  fallback_peer_ids: string[];
  last_received_at?: number | null;
  last_docs_activity_at?: number | null;
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
export type GameRoomKind = 'score_game' | 'metaverse_room';

export type MetaverseAssetKind = 'vrm' | 'glb' | 'texture' | 'other';

export type MetaverseAssetRef = {
  kind: MetaverseAssetKind;
  blob_hash: string;
  mime_type?: string | null;
  size_bytes?: number | null;
  name?: string | null;
};

export type MetaversePrimitive = 'cube' | 'sphere';

export type SharedRoomObjectV1 = {
  object_id: string;
  asset_ref?: MetaverseAssetRef | null;
  primitive_fallback: MetaversePrimitive;
  position: [number, number, number];
  rotation: [number, number, number];
  scale: [number, number, number];
  updated_by: string;
  updated_at: number;
};

export type MetaverseRoomStateV1 = {
  world_version: number;
  max_peers?: number | null;
  scene: {
    ground: string;
    shared_object: SharedRoomObjectV1;
  };
  default_spawn: {
    position: [number, number, number];
    rotation: [number, number, number];
  };
  asset_refs: MetaverseAssetRef[];
  chat_history?: MetaverseRoomChatMessageV1[];
};

export type MetaverseRoomPresenceV1 = {
  room_id: string;
  peer_id: string;
  display_name?: string | null;
  avatar_asset_ref?: MetaverseAssetRef | null;
  joined_at: number;
  last_seen_at: number;
};

export type MetaverseAvatarTransformV1 = {
  room_id: string;
  peer_id: string;
  seq: number;
  position: [number, number, number];
  rotation: [number, number, number];
  animation?: string | null;
  sent_at: number;
};

export type MetaverseRoomChatMessageV1 = {
  room_id: string;
  message_id: string;
  author_peer_id: string;
  display_name?: string | null;
  body: string;
  created_at: number;
};

export type MetaverseRoomEventV1 =
  | { type: 'presence_join'; presence: MetaverseRoomPresenceV1 }
  | { type: 'presence_leave'; room_id: string; peer_id: string; left_at: number }
  | { type: 'avatar_transform'; transform: MetaverseAvatarTransformV1 }
  | { type: 'chat_message'; message: MetaverseRoomChatMessageV1 }
  | { type: 'object_update'; object: SharedRoomObjectV1 };

export type MetaverseRoomEventView = {
  envelope_id: string;
  content: {
    event_id: string;
    topic_id: string;
    channel_id?: string | null;
    room_id: string;
    peer_id: string;
    seq: number;
    sent_at: number;
    event: MetaverseRoomEventV1;
  };
  envelope: Record<string, unknown>;
  received_at: number;
  source_peer: string;
};

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
  room_kind?: GameRoomKind;
  metaverse?: MetaverseRoomStateV1 | null;
  manifest_blob_hash?: string | null;
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

export type ChannelAccessTokenKind = 'invite' | 'grant' | 'share';

export type ChannelAccessTokenExport = {
  kind: ChannelAccessTokenKind;
  token: string;
};

export type ChannelAccessTokenPreview = {
  kind: ChannelAccessTokenKind;
  topic_id: string;
  channel_id: string;
  channel_label: string;
  owner_pubkey: string;
  inviter_pubkey?: string | null;
  sponsor_pubkey?: string | null;
  epoch_id: string;
};

export type PrivateChannelInvitePreview = {
  channel_id: string;
  topic_id: string;
  channel_label: string;
  inviter_pubkey: string;
  owner_pubkey: string;
  epoch_id: string;
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
  listRecentReactions(limit?: number): Promise<RecentReactionView[]>;
  createCustomReactionAsset(
    upload: CreateAttachmentInput,
    cropRect: CustomReactionCropRect,
    searchKey: string
  ): Promise<CustomReactionAssetView>;
  listBookmarkedCustomReactions(): Promise<BookmarkedCustomReactionView[]>;
  bookmarkCustomReaction(asset: CustomReactionAssetView): Promise<BookmarkedCustomReactionView>;
  removeBookmarkedCustomReaction(assetId: string): Promise<void>;
  listBookmarkedPosts(): Promise<BookmarkedPostView[]>;
  bookmarkPost(topic: string, objectId: string): Promise<BookmarkedPostView>;
  removeBookmarkedPost(objectId: string): Promise<void>;
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
  muteAuthor(pubkey: string): Promise<AuthorSocialView>;
  unmuteAuthor(pubkey: string): Promise<AuthorSocialView>;
  listSocialConnections(kind: SocialConnectionKind): Promise<AuthorSocialView[]>;
  listNotifications(): Promise<NotificationView[]>;
  markNotificationRead(notificationId: string): Promise<NotificationStatusView>;
  markAllNotificationsRead(): Promise<NotificationStatusView>;
  getNotificationStatus(): Promise<NotificationStatusView>;
  openDirectMessage(pubkey: string): Promise<DirectMessageConversationView>;
  listDirectMessages(): Promise<DirectMessageConversationView[]>;
  listDirectMessageMessages(
    pubkey: string,
    cursor?: TimelineCursor | null,
    limit?: number
  ): Promise<DirectMessageTimelineView>;
  sendDirectMessage(
    pubkey: string,
    text?: string | null,
    attachments?: CreateAttachmentInput[],
    replyToMessageId?: string | null
  ): Promise<string>;
  deleteDirectMessageMessage(pubkey: string, messageId: string): Promise<void>;
  clearDirectMessage(pubkey: string): Promise<void>;
  getDirectMessageStatus(pubkey: string): Promise<DirectMessageStatusView>;
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
  createMetaverseRoom(
    topic: string,
    title: string,
    description: string,
    maxPeers?: number | null,
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
  exportChannelAccessToken(
    topic: string,
    channelId: string,
    expiresAt?: number | null
  ): Promise<ChannelAccessTokenExport>;
  previewChannelAccessToken(token: string): Promise<ChannelAccessTokenPreview>;
  importChannelAccessToken(token: string): Promise<ChannelAccessTokenPreview>;
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
  leavePrivateChannel(topic: string, channelId: string): Promise<void>;
  listJoinedPrivateChannels(topic: string): Promise<JoinedPrivateChannelView[]>;
  updateGameRoom(
    topic: string,
    roomId: string,
    status: GameRoomStatus,
    phaseLabel: string | null,
    scores: GameScoreView[]
  ): Promise<void>;
  updateMetaverseRoom(
    topic: string,
    roomId: string,
    status: GameRoomStatus,
    sharedObjectPosition: [number, number, number],
    sharedObjectRotation: [number, number, number],
    sharedObjectScale: [number, number, number]
  ): Promise<void>;
  publishMetaverseRoomEvent(
    topic: string,
    roomId: string,
    peerId: string,
    seq: number,
    event: MetaverseRoomEventV1
  ): Promise<MetaverseRoomEventView>;
  listMetaverseRoomEvents(
    topic: string,
    roomId: string,
    afterEnvelopeId?: string | null,
    limit?: number | null
  ): Promise<MetaverseRoomEventView[]>;
  importMetaverseRoomAsset(
    topic: string,
    roomId: string,
    kind: MetaverseAssetKind,
    mimeType: string,
    name: string | null,
    dataBase64: string
  ): Promise<MetaverseAssetRef>;
  getSyncStatus(): Promise<SyncStatus>;
  getDiscoveryConfig(): Promise<DiscoveryConfig>;
  getCommunityNodeConfig(): Promise<CommunityNodeConfig>;
  getCommunityNodeStatuses(): Promise<CommunityNodeNodeStatus[]>;
  setCommunityNodeConfig(nodes: CommunityNodeConfigInput[]): Promise<CommunityNodeConfig>;
  clearCommunityNodeConfig(): Promise<void>;
  authenticateCommunityNode(baseUrl: string): Promise<CommunityNodeNodeStatus>;
  clearCommunityNodeToken(baseUrl: string): Promise<CommunityNodeNodeStatus>;
  getCommunityNodeConsentStatus(baseUrl: string): Promise<CommunityNodeNodeStatus>;
  acceptCommunityNodeConsents(
    baseUrl: string,
    policySlugs: string[]
  ): Promise<CommunityNodeNodeStatus>;
  refreshCommunityNodeMetadata(baseUrl: string): Promise<CommunityNodeNodeStatus>;
  fetchCommunityNodeManifest(baseUrl: string): Promise<CommunityNodeManifestFetch>;
  submitCommunityNodeReport(
    request: SubmitCommunityNodeReportRequest
  ): Promise<SubmitCommunityNodeReportResult>;
  importPeerTicket(ticket: string): Promise<void>;
  setDiscoverySeeds(seedEntries: string[]): Promise<DiscoveryConfig>;
  unsubscribeTopic(topic: string): Promise<void>;
  setTopicGossipEnabled(topic: string, enabled: boolean): Promise<void>;
  setChannelGossipEnabled(topic: string, channelId: string, enabled: boolean): Promise<void>;
  getLocalPeerTicket(): Promise<string | null>;
  getBlobMediaPayload(hash: string, mime: string): Promise<BlobMediaPayload | null>;
  getBlobPreviewUrl(hash: string, mime: string): Promise<string | null>;
}

declare global {
  interface Window {
    __KUKURI_DESKTOP__?: DesktopApi;
  }
}
