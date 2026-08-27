import type {
  AuthorSocialView,
  BlobMediaPayload,
  BookmarkedCustomReactionView,
  BookmarkedPostView,
  ChannelAccessTokenExport,
  ChannelAccessTokenPreview,
  CommunityNodeConfig,
  CommunityNodeIndexingRequest,
  CommunityNodeIndexQueryRequest,
  CommunityNodeManifestFetch,
  CommunityNodeNodeStatus,
  CommunityNodeRelationNeighborsRequest,
  CommunityNodeUserAdvisoryRequest,
  CustomReactionAssetView,
  DesktopApi,
  DirectMessageConversationView,
  DirectMessageStatusView,
  DirectMessageTimelineView,
  DiscoveryConfig,
  FriendOnlyGrantPreview,
  FriendPlusSharePreview,
  GameRoomView,
  JoinedPrivateChannelView,
  IndexQueryResponse,
  RelationNeighborsResponse,
  RelationOptoutResponse,
  RelationReadResponse,
  LiveSessionView,
  MetaverseAssetRef,
  MetaverseRoomEventView,
  NotificationStatusView,
  NotificationView,
  PrivateChannelInvitePreview,
  Profile,
  ReactionStateView,
  RecentReactionView,
  SubmitCommunityNodeReportResult,
  SubmitIndexingRequestResponse,
  SyncStatus,
  TimelineView,
  TrustUserReadResponse,
} from '../types';

// request DTO の生成型(WP-B6)。組み立てた literal を satisfies で拘束し、
// Rust 側 DTO の変更を tsc で検出する。手書き types.ts の同名 shadow を避けるため
// types.generated から直接 import する。
import type {
  AcceptCommunityNodeConsentsRequest,
  AuthorRequest,
  BookmarkCustomReactionRequest,
  BookmarkPostRequest,
  CommunityNodeTargetRequest,
  CreateCustomReactionAssetRequest,
  CreateGameRoomRequest,
  CreateLiveSessionRequest,
  CreateMetaverseRoomRequest,
  CreatePostRequest,
  CreatePrivateChannelRequest,
  CreateRepostRequest,
  DeleteDirectMessageMessageRequest,
  DirectMessageRequest,
  ExportChannelAccessTokenRequest,
  ExportFriendOnlyGrantRequest,
  ExportFriendPlusShareRequest,
  ExportPrivateChannelInviteRequest,
  FreezePrivateChannelRequest,
  GetBlobMediaRequest,
  GetBlobPreviewRequest,
  ImportChannelAccessTokenRequest,
  ImportFriendOnlyGrantRequest,
  ImportFriendPlusShareRequest,
  ImportMetaverseRoomAssetRequest,
  ImportPeerTicketRequest,
  ImportPrivateChannelInviteRequest,
  LeavePrivateChannelRequest,
  ListDirectMessageMessagesRequest,
  ListGameRoomsRequest,
  ListJoinedPrivateChannelsRequest,
  ListLiveSessionsRequest,
  ListMetaverseRoomEventsRequest,
  ListProfileTimelineRequest,
  ListRecentReactionsRequest,
  ListSocialConnectionsRequest,
  ListThreadRequest,
  ListTimelineRequest,
  LiveSessionCommandRequest,
  NotificationIdRequest,
  PreviewChannelAccessTokenRequest,
  PublishMetaverseRoomEventRequest,
  RemoveBookmarkedCustomReactionRequest,
  RemoveBookmarkedPostRequest,
  RotatePrivateChannelRequest,
  SendDirectMessageRequest,
  SetChannelGossipEnabledRequest,
  SetCommunityNodeConfigRequest,
  SetCommunityNodeInviteCodeRequest,
  SetDiscoverySeedsRequest,
  SetTopicGossipEnabledRequest,
  ToggleReactionRequest,
  UnsubscribeTopicRequest,
  UpdateGameRoomRequest,
  UpdateMetaverseRoomRequest,
  WithdrawPostRequest,
} from '../types.generated';

import { invokeDesktop } from '../invoke/desktop';
import { command } from '../invoke/dispatch';

export const runtimeApi: DesktopApi = {
  createPost: command('createPost', async (topic, content, replyTo, attachments = [], channelRef = { kind: 'public' }) => {
    return invokeDesktop<string>('create_post', {
      request: {
        topic,
        content,
        reply_to: replyTo,
        channel_ref: channelRef,
        attachments,
      } satisfies CreatePostRequest,
    });
  }),
  createRepost: command('createRepost', async (topic, sourceTopic, sourceObjectId, commentary) => {
    return invokeDesktop<string>('create_repost', {
      request: {
        topic,
        source_topic: sourceTopic,
        source_object_id: sourceObjectId,
        commentary,
      } satisfies CreateRepostRequest,
    });
  }),
  withdrawPost: command(
    'withdrawPost',
    async (
      topic,
      objectId,
      channelRef = { kind: 'public' },
      replacementObjectId = null,
      reasonVisibility = 'public',
      reason = 'author_request'
    ) => {
      return invokeDesktop<string>('withdraw_post', {
        request: {
          topic,
          object_id: objectId,
          channel_ref: channelRef,
          replacement_object_id: replacementObjectId,
          reason_visibility: reasonVisibility,
          reason,
        } satisfies WithdrawPostRequest,
      });
    }
  ),
  toggleReaction: command('toggleReaction', async (targetTopicId, targetObjectId, reactionKey, channelRef = null) => {
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
                search_key: reactionKey.asset.search_key,
                mime: reactionKey.asset.mime,
                bytes: reactionKey.asset.bytes,
                width: reactionKey.asset.width,
                height: reactionKey.asset.height,
              },
        channel_ref: channelRef,
      } satisfies ToggleReactionRequest,
    });
  }),
  listMyCustomReactionAssets: command('listMyCustomReactionAssets', async () => {
    return invokeDesktop<CustomReactionAssetView[]>('list_my_custom_reaction_assets');
  }),
  listRecentReactions: command('listRecentReactions', async (limit = 8) => {
    return invokeDesktop<RecentReactionView[]>('list_recent_reactions', {
      request: {
        limit,
      } satisfies ListRecentReactionsRequest,
    });
  }),
  createCustomReactionAsset: command('createCustomReactionAsset', async (upload, cropRect, searchKey) => {
    return invokeDesktop<CustomReactionAssetView>('create_custom_reaction_asset', {
      request: {
        upload,
        crop_rect: cropRect,
        search_key: searchKey,
      } satisfies CreateCustomReactionAssetRequest,
    });
  }),
  listBookmarkedCustomReactions: command('listBookmarkedCustomReactions', async () => {
    return invokeDesktop<BookmarkedCustomReactionView[]>('list_bookmarked_custom_reactions');
  }),
  bookmarkCustomReaction: command('bookmarkCustomReaction', async (asset) => {
    return invokeDesktop<BookmarkedCustomReactionView>('bookmark_custom_reaction', {
      request: {
        asset_id: asset.asset_id,
        owner_pubkey: asset.owner_pubkey,
        blob_hash: asset.blob_hash,
        search_key: asset.search_key,
        mime: asset.mime,
        bytes: asset.bytes,
        width: asset.width,
        height: asset.height,
      } satisfies BookmarkCustomReactionRequest,
    });
  }),
  removeBookmarkedCustomReaction: command('removeBookmarkedCustomReaction', async (assetId) => {
    return invokeDesktop<void>('remove_bookmarked_custom_reaction', {
      request: {
        asset_id: assetId,
      } satisfies RemoveBookmarkedCustomReactionRequest,
    });
  }),
  listBookmarkedPosts: command('listBookmarkedPosts', async () => {
    return invokeDesktop<BookmarkedPostView[]>('list_bookmarked_posts');
  }),
  bookmarkPost: command('bookmarkPost', async (topic, objectId) => {
    return invokeDesktop<BookmarkedPostView>('bookmark_post', {
      request: {
        topic,
        object_id: objectId,
      } satisfies BookmarkPostRequest,
    });
  }),
  removeBookmarkedPost: command('removeBookmarkedPost', async (objectId) => {
    return invokeDesktop<void>('remove_bookmarked_post', {
      request: {
        object_id: objectId,
      } satisfies RemoveBookmarkedPostRequest,
    });
  }),
  listTimeline: command('listTimeline', async (topic, cursor, limit, scope = { kind: 'public' }) => {
    return invokeDesktop<TimelineView>('list_timeline', {
      request: {
        topic,
        scope,
        cursor,
        limit,
      } satisfies ListTimelineRequest,
    });
  }),
  listThread: command('listThread', async (topic, threadId, cursor, limit) => {
    return invokeDesktop<TimelineView>('list_thread', {
      request: {
        topic,
        thread_id: threadId,
        cursor,
        limit,
      } satisfies ListThreadRequest,
    });
  }),
  listProfileTimeline: command('listProfileTimeline', async (pubkey, cursor, limit) => {
    return invokeDesktop<TimelineView>('list_profile_timeline', {
      request: {
        pubkey,
        cursor,
        limit,
      } satisfies ListProfileTimelineRequest,
    });
  }),
  getMyProfile: command('getMyProfile', async () => {
    return invokeDesktop<Profile>('get_my_profile');
  }),
  setMyProfile: command('setMyProfile', async (input) => {
    return invokeDesktop<Profile>('set_my_profile', {
      request: input,
    });
  }),
  followAuthor: command('followAuthor', async (pubkey) => {
    return invokeDesktop<AuthorSocialView>('follow_author', {
      request: { pubkey } satisfies AuthorRequest,
    });
  }),
  unfollowAuthor: command('unfollowAuthor', async (pubkey) => {
    return invokeDesktop<AuthorSocialView>('unfollow_author', {
      request: { pubkey } satisfies AuthorRequest,
    });
  }),
  getAuthorSocialView: command('getAuthorSocialView', async (pubkey) => {
    return invokeDesktop<AuthorSocialView>('get_author_social_view', {
      request: { pubkey } satisfies AuthorRequest,
    });
  }),
  muteAuthor: command('muteAuthor', async (pubkey) => {
    return invokeDesktop<AuthorSocialView>('mute_author', {
      request: { pubkey } satisfies AuthorRequest,
    });
  }),
  unmuteAuthor: command('unmuteAuthor', async (pubkey) => {
    return invokeDesktop<AuthorSocialView>('unmute_author', {
      request: { pubkey } satisfies AuthorRequest,
    });
  }),
  listSocialConnections: command('listSocialConnections', async (kind) => {
    return invokeDesktop<AuthorSocialView[]>('list_social_connections', {
      request: { kind } satisfies ListSocialConnectionsRequest,
    });
  }),
  listNotifications: command('listNotifications', async () => {
    return invokeDesktop<NotificationView[]>('list_notifications');
  }),
  markNotificationRead: command('markNotificationRead', async (notificationId) => {
    return invokeDesktop<NotificationStatusView>('mark_notification_read', {
      request: { notification_id: notificationId } satisfies NotificationIdRequest,
    });
  }),
  markAllNotificationsRead: command('markAllNotificationsRead', async () => {
    return invokeDesktop<NotificationStatusView>('mark_all_notifications_read');
  }),
  getNotificationStatus: command('getNotificationStatus', async () => {
    return invokeDesktop<NotificationStatusView>('get_notification_status');
  }),
  openDirectMessage: command('openDirectMessage', async (pubkey) => {
    return invokeDesktop<DirectMessageConversationView>('open_direct_message', {
      request: { pubkey } satisfies DirectMessageRequest,
    });
  }),
  listDirectMessages: command('listDirectMessages', async () => {
    return invokeDesktop<DirectMessageConversationView[]>('list_direct_messages');
  }),
  listDirectMessageMessages: command('listDirectMessageMessages', async (pubkey, cursor, limit) => {
    return invokeDesktop<DirectMessageTimelineView>('list_direct_message_messages', {
      request: {
        pubkey,
        cursor,
        limit,
      } satisfies ListDirectMessageMessagesRequest,
    });
  }),
  sendDirectMessage: command('sendDirectMessage', async (pubkey, text, attachments = [], replyToMessageId) => {
    return invokeDesktop<string>('send_direct_message', {
      request: {
        pubkey,
        text,
        reply_to_message_id: replyToMessageId,
        attachments,
      } satisfies SendDirectMessageRequest,
    });
  }),
  deleteDirectMessageMessage: command('deleteDirectMessageMessage', async (pubkey, messageId) => {
    return invokeDesktop<void>('delete_direct_message_message', {
      request: {
        pubkey,
        message_id: messageId,
      } satisfies DeleteDirectMessageMessageRequest,
    });
  }),
  clearDirectMessage: command('clearDirectMessage', async (pubkey) => {
    return invokeDesktop<void>('clear_direct_message', {
      request: { pubkey } satisfies DirectMessageRequest,
    });
  }),
  getDirectMessageStatus: command('getDirectMessageStatus', async (pubkey) => {
    return invokeDesktop<DirectMessageStatusView>('get_direct_message_status', {
      request: { pubkey } satisfies DirectMessageRequest,
    });
  }),
  listLiveSessions: command('listLiveSessions', async (topic, scope = { kind: 'public' }) => {
    return invokeDesktop<LiveSessionView[]>('list_live_sessions', {
      request: {
        topic,
        scope,
      } satisfies ListLiveSessionsRequest,
    });
  }),
  createLiveSession: command('createLiveSession', async (topic, title, description, channelRef = { kind: 'public' }) => {
    return invokeDesktop<string>('create_live_session', {
      request: {
        topic,
        channel_ref: channelRef,
        title,
        description,
      } satisfies CreateLiveSessionRequest,
    });
  }),
  endLiveSession: command('endLiveSession', async (topic, sessionId) => {
    return invokeDesktop<void>('end_live_session', {
      request: {
        topic,
        session_id: sessionId,
      } satisfies LiveSessionCommandRequest,
    });
  }),
  joinLiveSession: command('joinLiveSession', async (topic, sessionId) => {
    return invokeDesktop<void>('join_live_session', {
      request: {
        topic,
        session_id: sessionId,
      } satisfies LiveSessionCommandRequest,
    });
  }),
  leaveLiveSession: command('leaveLiveSession', async (topic, sessionId) => {
    return invokeDesktop<void>('leave_live_session', {
      request: {
        topic,
        session_id: sessionId,
      } satisfies LiveSessionCommandRequest,
    });
  }),
  listGameRooms: command('listGameRooms', async (topic, scope = { kind: 'public' }) => {
    return invokeDesktop<GameRoomView[]>('list_game_rooms', {
      request: {
        topic,
        scope,
      } satisfies ListGameRoomsRequest,
    });
  }),
  createGameRoom: command('createGameRoom', async (
    topic,
    title,
    description,
    participants,
    channelRef = { kind: 'public' }
  ) => {
    return invokeDesktop<string>('create_game_room', {
      request: {
        topic,
        channel_ref: channelRef,
        title,
        description,
        participants,
      } satisfies CreateGameRoomRequest,
    });
  }),
  createMetaverseRoom: command('createMetaverseRoom', async (
    topic,
    title,
    description,
    maxPeers = null,
    channelRef = { kind: 'public' }
  ) => {
    return invokeDesktop<string>('create_metaverse_room', {
      request: {
        topic,
        channel_ref: channelRef,
        title,
        description,
        max_peers: maxPeers,
      } satisfies CreateMetaverseRoomRequest,
    });
  }),
  createPrivateChannel: command('createPrivateChannel', async (topic, label, audienceKind = 'invite_only') => {
    return invokeDesktop<JoinedPrivateChannelView>('create_private_channel', {
      request: { topic, label, audience_kind: audienceKind } satisfies CreatePrivateChannelRequest,
    });
  }),
  exportPrivateChannelInvite: command('exportPrivateChannelInvite', async (topic, channelId, expiresAt = null) => {
    return invokeDesktop<string>('export_private_channel_invite', {
      request: {
        topic,
        channel_id: channelId,
        expires_at: expiresAt,
      } satisfies ExportPrivateChannelInviteRequest,
    });
  }),
  importPrivateChannelInvite: command('importPrivateChannelInvite', async (token) => {
    return invokeDesktop<PrivateChannelInvitePreview>('import_private_channel_invite', {
      request: { token } satisfies ImportPrivateChannelInviteRequest,
    });
  }),
  exportChannelAccessToken: command('exportChannelAccessToken', async (topic, channelId, expiresAt = null) => {
    return invokeDesktop<ChannelAccessTokenExport>('export_channel_access_token', {
      request: {
        topic,
        channel_id: channelId,
        expires_at: expiresAt,
      } satisfies ExportChannelAccessTokenRequest,
    });
  }),
  previewChannelAccessToken: command('previewChannelAccessToken', async (token) => {
    return invokeDesktop<ChannelAccessTokenPreview>('preview_channel_access_token', {
      request: {
        token,
      } satisfies PreviewChannelAccessTokenRequest,
    });
  }),
  importChannelAccessToken: command('importChannelAccessToken', async (token) => {
    return invokeDesktop<ChannelAccessTokenPreview>('import_channel_access_token', {
      request: { token } satisfies ImportChannelAccessTokenRequest,
    });
  }),
  exportFriendOnlyGrant: command('exportFriendOnlyGrant', async (topic, channelId, expiresAt = null) => {
    return invokeDesktop<string>('export_friend_only_grant', {
      request: {
        topic,
        channel_id: channelId,
        expires_at: expiresAt,
      } satisfies ExportFriendOnlyGrantRequest,
    });
  }),
  importFriendOnlyGrant: command('importFriendOnlyGrant', async (token) => {
    return invokeDesktop<FriendOnlyGrantPreview>('import_friend_only_grant', {
      request: { token } satisfies ImportFriendOnlyGrantRequest,
    });
  }),
  exportFriendPlusShare: command('exportFriendPlusShare', async (topic, channelId, expiresAt = null) => {
    return invokeDesktop<string>('export_friend_plus_share', {
      request: {
        topic,
        channel_id: channelId,
        expires_at: expiresAt,
      } satisfies ExportFriendPlusShareRequest,
    });
  }),
  importFriendPlusShare: command('importFriendPlusShare', async (token) => {
    return invokeDesktop<FriendPlusSharePreview>('import_friend_plus_share', {
      request: { token } satisfies ImportFriendPlusShareRequest,
    });
  }),
  freezePrivateChannel: command('freezePrivateChannel', async (topic, channelId) => {
    return invokeDesktop<JoinedPrivateChannelView>('freeze_private_channel', {
      request: {
        topic,
        channel_id: channelId,
      } satisfies FreezePrivateChannelRequest,
    });
  }),
  rotatePrivateChannel: command('rotatePrivateChannel', async (topic, channelId) => {
    return invokeDesktop<JoinedPrivateChannelView>('rotate_private_channel', {
      request: {
        topic,
        channel_id: channelId,
      } satisfies RotatePrivateChannelRequest,
    });
  }),
  leavePrivateChannel: command('leavePrivateChannel', async (topic, channelId) => {
    return invokeDesktop<void>('leave_private_channel', {
      request: {
        topic,
        channel_id: channelId,
      } satisfies LeavePrivateChannelRequest,
    });
  }),
  listJoinedPrivateChannels: command('listJoinedPrivateChannels', async (topic) => {
    return invokeDesktop<JoinedPrivateChannelView[]>('list_joined_private_channels', {
      request: { topic } satisfies ListJoinedPrivateChannelsRequest,
    });
  }),
  updateGameRoom: command('updateGameRoom', async (topic, roomId, status, phaseLabel, scores) => {
    return invokeDesktop<void>('update_game_room', {
      request: {
        topic,
        room_id: roomId,
        status,
        phase_label: phaseLabel,
        scores,
      } satisfies UpdateGameRoomRequest,
    });
  }),
  updateMetaverseRoom: command('updateMetaverseRoom', async (
    topic,
    roomId,
    status,
    customization
  ) => {
    return invokeDesktop<void>('update_metaverse_room', {
      request: {
        topic,
        room_id: roomId,
        status,
        customization,
      } satisfies UpdateMetaverseRoomRequest,
    });
  }),
  publishMetaverseRoomEvent: command('publishMetaverseRoomEvent', async (topic, roomId, peerId, seq, event) => {
    return invokeDesktop<MetaverseRoomEventView>('publish_metaverse_room_event', {
      request: {
        topic,
        room_id: roomId,
        peer_id: peerId,
        seq,
        event,
      } satisfies PublishMetaverseRoomEventRequest,
    });
  }),
  listMetaverseRoomEvents: command('listMetaverseRoomEvents', async (topic, roomId, afterEnvelopeId = null, limit = null) => {
    return invokeDesktop<MetaverseRoomEventView[]>('list_metaverse_room_events', {
      request: {
        topic,
        room_id: roomId,
        after_envelope_id: afterEnvelopeId,
        limit,
      } satisfies ListMetaverseRoomEventsRequest,
    });
  }),
  importMetaverseRoomAsset: command('importMetaverseRoomAsset', async (topic, roomId, kind, mimeType, name, dataBase64) => {
    return invokeDesktop<MetaverseAssetRef>('import_metaverse_room_asset', {
      request: {
        topic,
        room_id: roomId,
        kind,
        mime_type: mimeType,
        name,
        data_base64: dataBase64,
      } satisfies ImportMetaverseRoomAssetRequest,
    });
  }),
  getSyncStatus: command('getSyncStatus', async () => {
    return invokeDesktop<SyncStatus>('get_sync_status');
  }),
  getDiscoveryConfig: command('getDiscoveryConfig', async () => {
    return invokeDesktop<DiscoveryConfig>('get_discovery_config');
  }),
  getCommunityNodeConfig: command('getCommunityNodeConfig', async () => {
    return invokeDesktop<CommunityNodeConfig>('get_community_node_config');
  }),
  getCommunityNodeStatuses: command('getCommunityNodeStatuses', async () => {
    return invokeDesktop<CommunityNodeNodeStatus[]>('get_community_node_statuses');
  }),
  setCommunityNodeConfig: command('setCommunityNodeConfig', async (nodes) => {
    return invokeDesktop<CommunityNodeConfig>('set_community_node_config', {
      request: {
        nodes,
      } satisfies SetCommunityNodeConfigRequest,
    });
  }),
  clearCommunityNodeConfig: command('clearCommunityNodeConfig', async () => {
    return invokeDesktop<void>('clear_community_node_config');
  }),
  authenticateCommunityNode: command('authenticateCommunityNode', async (baseUrl) => {
    return invokeDesktop<CommunityNodeNodeStatus>('authenticate_community_node', {
      request: {
        base_url: baseUrl,
      } satisfies CommunityNodeTargetRequest,
    });
  }),
  setCommunityNodeInviteCode: command(
    'setCommunityNodeInviteCode',
    async (baseUrl, inviteCode) => {
      return invokeDesktop<CommunityNodeNodeStatus>('set_community_node_invite_code', {
        request: {
          base_url: baseUrl,
          invite_code: inviteCode,
        } satisfies SetCommunityNodeInviteCodeRequest,
      });
    }
  ),
  clearCommunityNodeToken: command('clearCommunityNodeToken', async (baseUrl) => {
    return invokeDesktop<CommunityNodeNodeStatus>('clear_community_node_token', {
      request: {
        base_url: baseUrl,
      } satisfies CommunityNodeTargetRequest,
    });
  }),
  getCommunityNodeConsentStatus: command('getCommunityNodeConsentStatus', async (baseUrl) => {
    return invokeDesktop<CommunityNodeNodeStatus>('get_community_node_consent_status', {
      request: {
        base_url: baseUrl,
      } satisfies CommunityNodeTargetRequest,
    });
  }),
  acceptCommunityNodeConsents: command('acceptCommunityNodeConsents', async (baseUrl, policySlugs) => {
    return invokeDesktop<CommunityNodeNodeStatus>('accept_community_node_consents', {
      request: {
        base_url: baseUrl,
        policy_slugs: policySlugs,
      } satisfies AcceptCommunityNodeConsentsRequest,
    });
  }),
  refreshCommunityNodeMetadata: command('refreshCommunityNodeMetadata', async (baseUrl) => {
    return invokeDesktop<CommunityNodeNodeStatus>('refresh_community_node_metadata', {
      request: {
        base_url: baseUrl,
      } satisfies CommunityNodeTargetRequest,
    });
  }),
  fetchCommunityNodeManifest: command('fetchCommunityNodeManifest', async (baseUrl) => {
    return invokeDesktop<CommunityNodeManifestFetch>('fetch_community_node_manifest', {
      request: {
        base_url: baseUrl,
      } satisfies CommunityNodeTargetRequest,
    });
  }),
  readCommunityNodeTrustUser: command('readCommunityNodeTrustUser', async (request) => {
    return invokeDesktop<TrustUserReadResponse>('read_community_node_trust_user', {
      request: request satisfies CommunityNodeUserAdvisoryRequest,
    });
  }),
  readCommunityNodeRelationUser: command('readCommunityNodeRelationUser', async (request) => {
    return invokeDesktop<RelationReadResponse>('read_community_node_relation_user', {
      request: request satisfies CommunityNodeUserAdvisoryRequest,
    });
  }),
  listCommunityNodeRelationNeighbors: command(
    'listCommunityNodeRelationNeighbors',
    async (request) => {
      return invokeDesktop<RelationNeighborsResponse>('list_community_node_relation_neighbors', {
        request: request satisfies CommunityNodeRelationNeighborsRequest,
      });
    }
  ),
  getCommunityNodeRelationOptout: command('getCommunityNodeRelationOptout', async (baseUrl) => {
    return invokeDesktop<RelationOptoutResponse>('get_community_node_relation_optout', {
      request: { base_url: baseUrl } satisfies CommunityNodeTargetRequest,
    });
  }),
  setCommunityNodeRelationOptout: command('setCommunityNodeRelationOptout', async (baseUrl) => {
    return invokeDesktop<RelationOptoutResponse>('set_community_node_relation_optout', {
      request: { base_url: baseUrl } satisfies CommunityNodeTargetRequest,
    });
  }),
  clearCommunityNodeRelationOptout: command('clearCommunityNodeRelationOptout', async (baseUrl) => {
    return invokeDesktop<RelationOptoutResponse>('clear_community_node_relation_optout', {
      request: { base_url: baseUrl } satisfies CommunityNodeTargetRequest,
    });
  }),
  searchCommunityNodeIndex: command('searchCommunityNodeIndex', async (request) => {
    return invokeDesktop<IndexQueryResponse>('search_community_node_index', {
      request: request satisfies CommunityNodeIndexQueryRequest,
    });
  }),
  discoverCommunityNodeIndex: command('discoverCommunityNodeIndex', async (request) => {
    return invokeDesktop<IndexQueryResponse>('discover_community_node_index', {
      request: request satisfies CommunityNodeIndexQueryRequest,
    });
  }),
  recommendCommunityNodeIndex: command('recommendCommunityNodeIndex', async (request) => {
    return invokeDesktop<IndexQueryResponse>('recommend_community_node_index', {
      request: request satisfies CommunityNodeIndexQueryRequest,
    });
  }),
  submitCommunityNodeIndexingRequest: command(
    'submitCommunityNodeIndexingRequest',
    async (request) => {
      return invokeDesktop<SubmitIndexingRequestResponse>(
        'submit_community_node_indexing_request',
        {
          request: request satisfies CommunityNodeIndexingRequest,
        }
      );
    }
  ),
  submitCommunityNodeReport: command('submitCommunityNodeReport', async (request) => {
    return invokeDesktop<SubmitCommunityNodeReportResult>('submit_community_node_report', {
      request,
    });
  }),
  importPeerTicket: command('importPeerTicket', async (ticket) => {
    return invokeDesktop<void>('import_peer_ticket', {
      request: {
        ticket,
      } satisfies ImportPeerTicketRequest,
    });
  }),
  setDiscoverySeeds: command('setDiscoverySeeds', async (seedEntries) => {
    return invokeDesktop<DiscoveryConfig>('set_discovery_seeds', {
      request: {
        seed_entries: seedEntries,
      } satisfies SetDiscoverySeedsRequest,
    });
  }),
  unsubscribeTopic: command('unsubscribeTopic', async (topic) => {
    return invokeDesktop<void>('unsubscribe_topic', {
      request: {
        topic,
      } satisfies UnsubscribeTopicRequest,
    });
  }),
  setTopicGossipEnabled: command('setTopicGossipEnabled', async (topic, enabled) => {
    return invokeDesktop<void>('set_topic_gossip_enabled', {
      request: {
        topic,
        enabled,
      } satisfies SetTopicGossipEnabledRequest,
    });
  }),
  setChannelGossipEnabled: command('setChannelGossipEnabled', async (topic, channelId, enabled) => {
    return invokeDesktop<void>('set_channel_gossip_enabled', {
      request: {
        topic,
        channel: channelId,
        enabled,
      } satisfies SetChannelGossipEnabledRequest,
    });
  }),
  getLocalPeerTicket: command('getLocalPeerTicket', async () => {
    return invokeDesktop<string | null>('get_local_peer_ticket');
  }),
  getBlobMediaPayload: command('getBlobMediaPayload', async (hash, mime) => {
    return invokeDesktop<BlobMediaPayload | null>('get_blob_media_payload', {
      request: {
        hash,
        mime,
      } satisfies GetBlobMediaRequest,
    });
  }),
  getBlobPreviewUrl: command('getBlobPreviewUrl', async (hash, mime) => {
    return invokeDesktop<string | null>('get_blob_preview_url', {
      request: {
        hash,
        mime,
      } satisfies GetBlobPreviewRequest,
    });
  }),
};
