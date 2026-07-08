import type {
  AuthorSocialView,
  BlobMediaPayload,
  BookmarkedCustomReactionView,
  BookmarkedPostView,
  ChannelAccessTokenExport,
  ChannelAccessTokenPreview,
  CommunityNodeConfig,
  CommunityNodeManifestFetch,
  CommunityNodeNodeStatus,
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
  SyncStatus,
  TimelineView,
} from '../types';

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
      },
    });
  }),
  createRepost: command('createRepost', async (topic, sourceTopic, sourceObjectId, commentary) => {
    return invokeDesktop<string>('create_repost', {
      request: {
        topic,
        source_topic: sourceTopic,
        source_object_id: sourceObjectId,
        commentary,
      },
    });
  }),
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
      },
    });
  }),
  listMyCustomReactionAssets: command('listMyCustomReactionAssets', async () => {
    return invokeDesktop<CustomReactionAssetView[]>('list_my_custom_reaction_assets');
  }),
  listRecentReactions: command('listRecentReactions', async (limit = 8) => {
    return invokeDesktop<RecentReactionView[]>('list_recent_reactions', {
      request: {
        limit,
      },
    });
  }),
  createCustomReactionAsset: command('createCustomReactionAsset', async (upload, cropRect, searchKey) => {
    return invokeDesktop<CustomReactionAssetView>('create_custom_reaction_asset', {
      request: {
        upload,
        crop_rect: cropRect,
        search_key: searchKey,
      },
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
      },
    });
  }),
  removeBookmarkedCustomReaction: command('removeBookmarkedCustomReaction', async (assetId) => {
    return invokeDesktop<void>('remove_bookmarked_custom_reaction', {
      request: {
        asset_id: assetId,
      },
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
      },
    });
  }),
  removeBookmarkedPost: command('removeBookmarkedPost', async (objectId) => {
    return invokeDesktop<void>('remove_bookmarked_post', {
      request: {
        object_id: objectId,
      },
    });
  }),
  listTimeline: command('listTimeline', async (topic, cursor, limit, scope = { kind: 'public' }) => {
    return invokeDesktop<TimelineView>('list_timeline', {
      request: {
        topic,
        scope,
        cursor,
        limit,
      },
    });
  }),
  listThread: command('listThread', async (topic, threadId, cursor, limit) => {
    return invokeDesktop<TimelineView>('list_thread', {
      request: {
        topic,
        thread_id: threadId,
        cursor,
        limit,
      },
    });
  }),
  listProfileTimeline: command('listProfileTimeline', async (pubkey, cursor, limit) => {
    return invokeDesktop<TimelineView>('list_profile_timeline', {
      request: {
        pubkey,
        cursor,
        limit,
      },
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
      request: { pubkey },
    });
  }),
  unfollowAuthor: command('unfollowAuthor', async (pubkey) => {
    return invokeDesktop<AuthorSocialView>('unfollow_author', {
      request: { pubkey },
    });
  }),
  getAuthorSocialView: command('getAuthorSocialView', async (pubkey) => {
    return invokeDesktop<AuthorSocialView>('get_author_social_view', {
      request: { pubkey },
    });
  }),
  muteAuthor: command('muteAuthor', async (pubkey) => {
    return invokeDesktop<AuthorSocialView>('mute_author', {
      request: { pubkey },
    });
  }),
  unmuteAuthor: command('unmuteAuthor', async (pubkey) => {
    return invokeDesktop<AuthorSocialView>('unmute_author', {
      request: { pubkey },
    });
  }),
  listSocialConnections: command('listSocialConnections', async (kind) => {
    return invokeDesktop<AuthorSocialView[]>('list_social_connections', {
      request: { kind },
    });
  }),
  listNotifications: command('listNotifications', async () => {
    return invokeDesktop<NotificationView[]>('list_notifications');
  }),
  markNotificationRead: command('markNotificationRead', async (notificationId) => {
    return invokeDesktop<NotificationStatusView>('mark_notification_read', {
      request: { notification_id: notificationId },
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
      request: { pubkey },
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
      },
    });
  }),
  sendDirectMessage: command('sendDirectMessage', async (pubkey, text, attachments = [], replyToMessageId) => {
    return invokeDesktop<string>('send_direct_message', {
      request: {
        pubkey,
        text,
        reply_to_message_id: replyToMessageId,
        attachments,
      },
    });
  }),
  deleteDirectMessageMessage: command('deleteDirectMessageMessage', async (pubkey, messageId) => {
    return invokeDesktop<void>('delete_direct_message_message', {
      request: {
        pubkey,
        message_id: messageId,
      },
    });
  }),
  clearDirectMessage: command('clearDirectMessage', async (pubkey) => {
    return invokeDesktop<void>('clear_direct_message', {
      request: { pubkey },
    });
  }),
  getDirectMessageStatus: command('getDirectMessageStatus', async (pubkey) => {
    return invokeDesktop<DirectMessageStatusView>('get_direct_message_status', {
      request: { pubkey },
    });
  }),
  listLiveSessions: command('listLiveSessions', async (topic, scope = { kind: 'public' }) => {
    return invokeDesktop<LiveSessionView[]>('list_live_sessions', {
      request: {
        topic,
        scope,
      },
    });
  }),
  createLiveSession: command('createLiveSession', async (topic, title, description, channelRef = { kind: 'public' }) => {
    return invokeDesktop<string>('create_live_session', {
      request: {
        topic,
        channel_ref: channelRef,
        title,
        description,
      },
    });
  }),
  endLiveSession: command('endLiveSession', async (topic, sessionId) => {
    return invokeDesktop<void>('end_live_session', {
      request: {
        topic,
        session_id: sessionId,
      },
    });
  }),
  joinLiveSession: command('joinLiveSession', async (topic, sessionId) => {
    return invokeDesktop<void>('join_live_session', {
      request: {
        topic,
        session_id: sessionId,
      },
    });
  }),
  leaveLiveSession: command('leaveLiveSession', async (topic, sessionId) => {
    return invokeDesktop<void>('leave_live_session', {
      request: {
        topic,
        session_id: sessionId,
      },
    });
  }),
  listGameRooms: command('listGameRooms', async (topic, scope = { kind: 'public' }) => {
    return invokeDesktop<GameRoomView[]>('list_game_rooms', {
      request: {
        topic,
        scope,
      },
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
      },
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
      },
    });
  }),
  createPrivateChannel: command('createPrivateChannel', async (topic, label, audienceKind = 'invite_only') => {
    return invokeDesktop<JoinedPrivateChannelView>('create_private_channel', {
      request: { topic, label, audience_kind: audienceKind },
    });
  }),
  exportPrivateChannelInvite: command('exportPrivateChannelInvite', async (topic, channelId, expiresAt = null) => {
    return invokeDesktop<string>('export_private_channel_invite', {
      request: {
        topic,
        channel_id: channelId,
        expires_at: expiresAt,
      },
    });
  }),
  importPrivateChannelInvite: command('importPrivateChannelInvite', async (token) => {
    return invokeDesktop<PrivateChannelInvitePreview>('import_private_channel_invite', {
      request: { token },
    });
  }),
  exportChannelAccessToken: command('exportChannelAccessToken', async (topic, channelId, expiresAt = null) => {
    return invokeDesktop<ChannelAccessTokenExport>('export_channel_access_token', {
      request: {
        topic,
        channel_id: channelId,
        expires_at: expiresAt,
      },
    });
  }),
  previewChannelAccessToken: command('previewChannelAccessToken', async (token) => {
    return invokeDesktop<ChannelAccessTokenPreview>('preview_channel_access_token', {
      request: {
        token,
      },
    });
  }),
  importChannelAccessToken: command('importChannelAccessToken', async (token) => {
    return invokeDesktop<ChannelAccessTokenPreview>('import_channel_access_token', {
      request: { token },
    });
  }),
  exportFriendOnlyGrant: command('exportFriendOnlyGrant', async (topic, channelId, expiresAt = null) => {
    return invokeDesktop<string>('export_friend_only_grant', {
      request: {
        topic,
        channel_id: channelId,
        expires_at: expiresAt,
      },
    });
  }),
  importFriendOnlyGrant: command('importFriendOnlyGrant', async (token) => {
    return invokeDesktop<FriendOnlyGrantPreview>('import_friend_only_grant', {
      request: { token },
    });
  }),
  exportFriendPlusShare: command('exportFriendPlusShare', async (topic, channelId, expiresAt = null) => {
    return invokeDesktop<string>('export_friend_plus_share', {
      request: {
        topic,
        channel_id: channelId,
        expires_at: expiresAt,
      },
    });
  }),
  importFriendPlusShare: command('importFriendPlusShare', async (token) => {
    return invokeDesktop<FriendPlusSharePreview>('import_friend_plus_share', {
      request: { token },
    });
  }),
  freezePrivateChannel: command('freezePrivateChannel', async (topic, channelId) => {
    return invokeDesktop<JoinedPrivateChannelView>('freeze_private_channel', {
      request: {
        topic,
        channel_id: channelId,
      },
    });
  }),
  rotatePrivateChannel: command('rotatePrivateChannel', async (topic, channelId) => {
    return invokeDesktop<JoinedPrivateChannelView>('rotate_private_channel', {
      request: {
        topic,
        channel_id: channelId,
      },
    });
  }),
  leavePrivateChannel: command('leavePrivateChannel', async (topic, channelId) => {
    return invokeDesktop<void>('leave_private_channel', {
      request: {
        topic,
        channel_id: channelId,
      },
    });
  }),
  listJoinedPrivateChannels: command('listJoinedPrivateChannels', async (topic) => {
    return invokeDesktop<JoinedPrivateChannelView[]>('list_joined_private_channels', {
      request: { topic },
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
      },
    });
  }),
  updateMetaverseRoom: command('updateMetaverseRoom', async (
    topic,
    roomId,
    status,
    sharedObjectPosition,
    sharedObjectRotation,
    sharedObjectScale
  ) => {
    return invokeDesktop<void>('update_metaverse_room', {
      request: {
        topic,
        room_id: roomId,
        status,
        shared_object_position: sharedObjectPosition,
        shared_object_rotation: sharedObjectRotation,
        shared_object_scale: sharedObjectScale,
      },
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
      },
    });
  }),
  listMetaverseRoomEvents: command('listMetaverseRoomEvents', async (topic, roomId, afterEnvelopeId = null, limit = null) => {
    return invokeDesktop<MetaverseRoomEventView[]>('list_metaverse_room_events', {
      request: {
        topic,
        room_id: roomId,
        after_envelope_id: afterEnvelopeId,
        limit,
      },
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
      },
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
      },
    });
  }),
  clearCommunityNodeConfig: command('clearCommunityNodeConfig', async () => {
    return invokeDesktop<void>('clear_community_node_config');
  }),
  authenticateCommunityNode: command('authenticateCommunityNode', async (baseUrl) => {
    return invokeDesktop<CommunityNodeNodeStatus>('authenticate_community_node', {
      request: {
        base_url: baseUrl,
      },
    });
  }),
  clearCommunityNodeToken: command('clearCommunityNodeToken', async (baseUrl) => {
    return invokeDesktop<CommunityNodeNodeStatus>('clear_community_node_token', {
      request: {
        base_url: baseUrl,
      },
    });
  }),
  getCommunityNodeConsentStatus: command('getCommunityNodeConsentStatus', async (baseUrl) => {
    return invokeDesktop<CommunityNodeNodeStatus>('get_community_node_consent_status', {
      request: {
        base_url: baseUrl,
      },
    });
  }),
  acceptCommunityNodeConsents: command('acceptCommunityNodeConsents', async (baseUrl, policySlugs) => {
    return invokeDesktop<CommunityNodeNodeStatus>('accept_community_node_consents', {
      request: {
        base_url: baseUrl,
        policy_slugs: policySlugs,
      },
    });
  }),
  refreshCommunityNodeMetadata: command('refreshCommunityNodeMetadata', async (baseUrl) => {
    return invokeDesktop<CommunityNodeNodeStatus>('refresh_community_node_metadata', {
      request: {
        base_url: baseUrl,
      },
    });
  }),
  fetchCommunityNodeManifest: command('fetchCommunityNodeManifest', async (baseUrl) => {
    return invokeDesktop<CommunityNodeManifestFetch>('fetch_community_node_manifest', {
      request: {
        base_url: baseUrl,
      },
    });
  }),
  submitCommunityNodeReport: command('submitCommunityNodeReport', async (request) => {
    return invokeDesktop<SubmitCommunityNodeReportResult>('submit_community_node_report', {
      request,
    });
  }),
  importPeerTicket: command('importPeerTicket', async (ticket) => {
    return invokeDesktop<void>('import_peer_ticket', {
      request: {
        ticket,
      },
    });
  }),
  setDiscoverySeeds: command('setDiscoverySeeds', async (seedEntries) => {
    return invokeDesktop<DiscoveryConfig>('set_discovery_seeds', {
      request: {
        seed_entries: seedEntries,
      },
    });
  }),
  unsubscribeTopic: command('unsubscribeTopic', async (topic) => {
    return invokeDesktop<void>('unsubscribe_topic', {
      request: {
        topic,
      },
    });
  }),
  setTopicGossipEnabled: command('setTopicGossipEnabled', async (topic, enabled) => {
    return invokeDesktop<void>('set_topic_gossip_enabled', {
      request: {
        topic,
        enabled,
      },
    });
  }),
  setChannelGossipEnabled: command('setChannelGossipEnabled', async (topic, channelId, enabled) => {
    return invokeDesktop<void>('set_channel_gossip_enabled', {
      request: {
        topic,
        channel: channelId,
        enabled,
      },
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
      },
    });
  }),
  getBlobPreviewUrl: command('getBlobPreviewUrl', async (hash, mime) => {
    return invokeDesktop<string | null>('get_blob_preview_url', {
      request: {
        hash,
        mime,
      },
    });
  }),
};
