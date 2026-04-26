import type {
  AuthorSocialView,
  BlobMediaPayload,
  BookmarkedCustomReactionView,
  BookmarkedPostView,
  ChannelAccessTokenExport,
  ChannelAccessTokenPreview,
  CommunityNodeConfig,
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
  NotificationStatusView,
  NotificationView,
  PrivateChannelInvitePreview,
  Profile,
  ReactionStateView,
  RecentReactionView,
  SyncStatus,
  TimelineView,
} from '../types';

import { invokeDesktop } from '../invoke/desktop';

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
                search_key: reactionKey.asset.search_key,
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
  listRecentReactions: async (limit = 8) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listRecentReactions(limit);
    }
    return invokeDesktop<RecentReactionView[]>('list_recent_reactions', {
      request: {
        limit,
      },
    });
  },
  createCustomReactionAsset: async (upload, cropRect, searchKey) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.createCustomReactionAsset(upload, cropRect, searchKey);
    }
    return invokeDesktop<CustomReactionAssetView>('create_custom_reaction_asset', {
      request: {
        upload,
        crop_rect: cropRect,
        search_key: searchKey,
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
        search_key: asset.search_key,
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
  listBookmarkedPosts: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listBookmarkedPosts();
    }
    return invokeDesktop<BookmarkedPostView[]>('list_bookmarked_posts');
  },
  bookmarkPost: async (topic, objectId) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.bookmarkPost(topic, objectId);
    }
    return invokeDesktop<BookmarkedPostView>('bookmark_post', {
      request: {
        topic,
        object_id: objectId,
      },
    });
  },
  removeBookmarkedPost: async (objectId) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.removeBookmarkedPost(objectId);
    }
    return invokeDesktop<void>('remove_bookmarked_post', {
      request: {
        object_id: objectId,
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
  muteAuthor: async (pubkey) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.muteAuthor(pubkey);
    }
    return invokeDesktop<AuthorSocialView>('mute_author', {
      request: { pubkey },
    });
  },
  unmuteAuthor: async (pubkey) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.unmuteAuthor(pubkey);
    }
    return invokeDesktop<AuthorSocialView>('unmute_author', {
      request: { pubkey },
    });
  },
  listSocialConnections: async (kind) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listSocialConnections(kind);
    }
    return invokeDesktop<AuthorSocialView[]>('list_social_connections', {
      request: { kind },
    });
  },
  listNotifications: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listNotifications();
    }
    return invokeDesktop<NotificationView[]>('list_notifications');
  },
  markNotificationRead: async (notificationId) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.markNotificationRead(notificationId);
    }
    return invokeDesktop<NotificationStatusView>('mark_notification_read', {
      request: { notification_id: notificationId },
    });
  },
  markAllNotificationsRead: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.markAllNotificationsRead();
    }
    return invokeDesktop<NotificationStatusView>('mark_all_notifications_read');
  },
  getNotificationStatus: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getNotificationStatus();
    }
    return invokeDesktop<NotificationStatusView>('get_notification_status');
  },
  openDirectMessage: async (pubkey) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.openDirectMessage(pubkey);
    }
    return invokeDesktop<DirectMessageConversationView>('open_direct_message', {
      request: { pubkey },
    });
  },
  listDirectMessages: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listDirectMessages();
    }
    return invokeDesktop<DirectMessageConversationView[]>('list_direct_messages');
  },
  listDirectMessageMessages: async (pubkey, cursor, limit) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listDirectMessageMessages(pubkey, cursor, limit);
    }
    return invokeDesktop<DirectMessageTimelineView>('list_direct_message_messages', {
      request: {
        pubkey,
        cursor,
        limit,
      },
    });
  },
  sendDirectMessage: async (pubkey, text, attachments = [], replyToMessageId) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.sendDirectMessage(
        pubkey,
        text,
        attachments,
        replyToMessageId
      );
    }
    return invokeDesktop<string>('send_direct_message', {
      request: {
        pubkey,
        text,
        reply_to_message_id: replyToMessageId,
        attachments,
      },
    });
  },
  deleteDirectMessageMessage: async (pubkey, messageId) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.deleteDirectMessageMessage(pubkey, messageId);
    }
    return invokeDesktop<void>('delete_direct_message_message', {
      request: {
        pubkey,
        message_id: messageId,
      },
    });
  },
  clearDirectMessage: async (pubkey) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.clearDirectMessage(pubkey);
    }
    return invokeDesktop<void>('clear_direct_message', {
      request: { pubkey },
    });
  },
  getDirectMessageStatus: async (pubkey) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getDirectMessageStatus(pubkey);
    }
    return invokeDesktop<DirectMessageStatusView>('get_direct_message_status', {
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
  exportChannelAccessToken: async (topic, channelId, expiresAt = null) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.exportChannelAccessToken(topic, channelId, expiresAt);
    }
    return invokeDesktop<ChannelAccessTokenExport>('export_channel_access_token', {
      request: {
        topic,
        channel_id: channelId,
        expires_at: expiresAt,
      },
    });
  },
  previewChannelAccessToken: async (token) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.previewChannelAccessToken(token);
    }
    return invokeDesktop<ChannelAccessTokenPreview>('preview_channel_access_token', {
      request: {
        token,
      },
    });
  },
  importChannelAccessToken: async (token) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.importChannelAccessToken(token);
    }
    return invokeDesktop<ChannelAccessTokenPreview>('import_channel_access_token', {
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
  leavePrivateChannel: async (topic, channelId) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.leavePrivateChannel(topic, channelId);
    }
    return invokeDesktop<void>('leave_private_channel', {
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
  setCommunityNodeConfig: async (nodes) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.setCommunityNodeConfig(nodes);
    }
    return invokeDesktop<CommunityNodeConfig>('set_community_node_config', {
      request: {
        nodes,
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
