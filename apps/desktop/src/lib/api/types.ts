// IPC の view / DTO 型は Rust(crates/app-api/src/views.rs ほか)から生成する
// (WP-H7 PR3)。定義元は types.generated.ts。ここでは front 専用型・入力型・
// DesktopApi interface と、生成型に front 専用フィールドを交差させる PostView を扱う。
export * from './types.generated';
import type {
  AuthorSocialView,
  BlobMediaPayload,
  BookmarkedPostView,
  ChannelAccessTokenExport,
  ChannelAccessTokenPreview,
  ChannelAudienceKind,
  ConnectMode,
  CustomReactionAssetView,
  DirectMessageConversationView,
  DirectMessageStatusView,
  DirectMessageTimelineView,
  DiscoveryMode,
  JoinedPrivateChannelView,
  NotificationStatusView,
  NotificationView,
  PostView as WirePostView,
  ProfileAssetView,
  ReactionStateView,
  RecentReactionView,
  SocialConnectionKind,
  SyncStatus,
  TimelineCursor,
  TimelineView,
} from './types.generated';

// PostView は wire 型に front 専用のローカル下書き状態を交差させる。
export type PostView = WirePostView & {
  local_id?: string | null;
  local_state?: 'pending' | 'syncing' | 'failed' | null;
  local_error?: string | null;
  server_object_id?: string | null;
  local_draft?: LocalPostDraft | null;
  local_draft_media_items?: LocalDraftMediaItem[] | null;
};

export type ChannelRef =
  | { kind: 'public' }
  | { kind: 'private_channel'; channel_id: string };

export type TimelineScope =
  | { kind: 'public' }
  | { kind: 'all_joined' }
  | { kind: 'channel'; channel_id: string };

// Tauri コマンドの構造化エラー封筒(src-tauri の CommandError と対、WP-C3)。
// invoke reject の payload 形状であり views fixture(S4 codegen)の対象外。
// message は従来の平文エラー文言と同一、code は機械判定用(既知値:
// 'command_failed'。'bridge_unavailable' は TS 側合成 — invoke/error.ts 参照)。
export type CommandError = {
  code: string;
  message: string;
};

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

export type BookmarkedCustomReactionView = CustomReactionAssetView;

export type ReactionKeyInput =
  | { kind: 'emoji'; emoji: string }
  | { kind: 'custom_asset'; asset: CustomReactionAssetView };

export type CustomReactionCropRect = {
  x: number;
  y: number;
  size: number;
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
  body?: string;
  required: boolean;
  accepted_at?: number | null;
  previously_accepted_version?: number | null;
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
  room_kind: GameRoomKind;
  metaverse?: MetaverseRoomStateV1 | null;
  manifest_blob_hash: string;
  updated_at: number;
  channel_id?: string | null;
  audience_label: string;
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
