import {
  type AttachmentView,
  type AuthorSocialView,
  type BlobMediaPayload,
  type ChannelAudienceKind,
  type CommunityNodeConfig,
  type CommunityNodeNodeStatus,
  type DesktopApi,
  type DiscoveryConfig,
  type FriendOnlyGrantPreview,
  type FriendPlusSharePreview,
  type GameRoomView,
  type GameScoreView,
  type JoinedPrivateChannelView,
  type LiveSessionView,
  type PostView,
  type PrivateChannelInvitePreview,
  type Profile,
  type SyncStatus,
  type TimelineScope,
  type TimelineView,
} from '@/lib/api';

export type DesktopMockApiOptions = {
  globalLastError?: string | null;
  topicLastError?: string | null;
  assistPeerIds?: string[];
  seedPosts?: Record<string, TimelineView['items']>;
  authorProfileTimelines?: Record<string, TimelineView['items']>;
  seedLiveSessions?: Record<string, LiveSessionView[]>;
  seedGameRooms?: Record<string, GameRoomView[]>;
  myProfile?: Partial<Profile>;
  authorSocialViews?: Record<string, Partial<AuthorSocialView>>;
  myProfileError?: string | null;
  invitePreview?: PrivateChannelInvitePreview;
};

function withSocialPostDefaults(post: PostView): PostView {
  return {
    ...post,
    author_name: post.author_name ?? null,
    author_display_name: post.author_display_name ?? null,
    following: post.following ?? false,
    followed_by: post.followed_by ?? false,
    mutual: post.mutual ?? false,
    friend_of_friend: post.friend_of_friend ?? false,
    published_topic_id: post.published_topic_id ?? post.origin_topic_id ?? null,
    origin_topic_id: post.origin_topic_id ?? null,
    repost_of: post.repost_of ?? null,
    repost_commentary: post.repost_commentary ?? null,
    is_threadable:
      post.is_threadable ?? (post.object_kind !== 'repost' || Boolean(post.repost_commentary)),
    channel_id: post.channel_id ?? null,
    audience_label: post.audience_label ?? (post.channel_id ? 'Private channel' : 'Public'),
    attachments: [...post.attachments],
  };
}

function withLiveSessionDefaults(session: LiveSessionView): LiveSessionView {
  return {
    ...session,
    channel_id: session.channel_id ?? null,
    audience_label: session.audience_label ?? (session.channel_id ? 'Private channel' : 'Public'),
  };
}

function withGameRoomDefaults(room: GameRoomView): GameRoomView {
  return {
    ...room,
    channel_id: room.channel_id ?? null,
    audience_label: room.audience_label ?? (room.channel_id ? 'Private channel' : 'Public'),
    scores: room.scores.map((score) => ({ ...score })),
  };
}

function withJoinedChannelDefaults(channel: JoinedPrivateChannelView): JoinedPrivateChannelView {
  return {
    ...channel,
    owner_pubkey: channel.owner_pubkey ?? channel.creator_pubkey,
    joined_via_pubkey: channel.joined_via_pubkey ?? null,
    audience_kind: channel.audience_kind ?? 'invite_only',
    is_owner: channel.is_owner ?? true,
    current_epoch_id: channel.current_epoch_id ?? 'legacy',
    archived_epoch_ids: [...(channel.archived_epoch_ids ?? [])],
    sharing_state: channel.sharing_state ?? 'open',
    rotation_required: channel.rotation_required ?? false,
    participant_count: channel.participant_count ?? 0,
    stale_participant_count: channel.stale_participant_count ?? 0,
  };
}

function withDefaultAuthorView(
  pubkey: string,
  view?: Partial<AuthorSocialView>
): AuthorSocialView {
  return {
    author_pubkey: pubkey,
    name: null,
    display_name: null,
    about: null,
    picture: null,
    updated_at: null,
    following: false,
    followed_by: false,
    mutual: false,
    friend_of_friend: false,
    friend_of_friend_via_pubkeys: [],
    ...view,
  };
}

function cloneSyncStatus(syncStatus: SyncStatus): SyncStatus {
  return {
    ...syncStatus,
    configured_peers: [...syncStatus.configured_peers],
    subscribed_topics: [...syncStatus.subscribed_topics],
    topic_diagnostics: syncStatus.topic_diagnostics.map((diagnostic) => ({
      ...diagnostic,
      connected_peers: [...diagnostic.connected_peers],
      assist_peer_ids: [...diagnostic.assist_peer_ids],
      configured_peer_ids: [...diagnostic.configured_peer_ids],
      missing_peer_ids: [...diagnostic.missing_peer_ids],
    })),
    discovery: {
      ...syncStatus.discovery,
      configured_seed_peer_ids: [...syncStatus.discovery.configured_seed_peer_ids],
      bootstrap_seed_peer_ids: [...syncStatus.discovery.bootstrap_seed_peer_ids],
      manual_ticket_peer_ids: [...syncStatus.discovery.manual_ticket_peer_ids],
      connected_peer_ids: [...syncStatus.discovery.connected_peer_ids],
      assist_peer_ids: [...syncStatus.discovery.assist_peer_ids],
    },
  };
}

function filterChannelScopedItems<T extends { channel_id?: string | null }>(
  items: T[],
  scope: TimelineScope,
  joinedChannels: JoinedPrivateChannelView[]
) {
  const joinedIds = new Set(joinedChannels.map((channel) => channel.channel_id));
  if (scope.kind === 'channel') {
    return items.filter((item) => item.channel_id === scope.channel_id);
  }
  if (scope.kind === 'all_joined') {
    return items.filter((item) => !item.channel_id || joinedIds.has(item.channel_id));
  }
  return items.filter((item) => !item.channel_id);
}

export function createDesktopMockApi(options?: DesktopMockApiOptions): DesktopApi {
  const assistPeerIds = options?.assistPeerIds ?? [];
  const effectivePeerIds = Array.from(new Set(['peer-a', ...assistPeerIds]));
  const postsByTopic: Record<string, TimelineView['items']> = Object.fromEntries(
    Object.entries(options?.seedPosts ?? {}).map(([topic, posts]) => [
      topic,
      posts.map((post) => withSocialPostDefaults({ ...post, origin_topic_id: post.origin_topic_id ?? topic })),
    ])
  );
  const authorProfileTimelines: Record<string, TimelineView['items']> = Object.fromEntries(
    Object.entries(options?.authorProfileTimelines ?? {}).map(([pubkey, posts]) => [
      pubkey,
      posts.map((post) => withSocialPostDefaults(post)),
    ])
  );
  for (const [topic, posts] of Object.entries(postsByTopic)) {
    for (const post of posts) {
      if (post.channel_id) {
        continue;
      }
      const current = authorProfileTimelines[post.author_pubkey] ?? [];
      if (current.some((item) => item.object_id === post.object_id)) {
        continue;
      }
      authorProfileTimelines[post.author_pubkey] = [
        withSocialPostDefaults({
          ...post,
          origin_topic_id: post.origin_topic_id ?? topic,
          channel_id: null,
          audience_label: 'Public',
        }),
        ...current,
      ].sort((left, right) => right.created_at - left.created_at || right.object_id.localeCompare(left.object_id));
    }
  }
  const liveSessionsByTopic: Record<string, LiveSessionView[]> = Object.fromEntries(
    Object.entries(options?.seedLiveSessions ?? {}).map(([topic, sessions]) => [
      topic,
      sessions.map((session) => withLiveSessionDefaults(session)),
    ])
  );
  const gameRoomsByTopic: Record<string, GameRoomView[]> = Object.fromEntries(
    Object.entries(options?.seedGameRooms ?? {}).map(([topic, rooms]) => [
      topic,
      rooms.map((room) => withGameRoomDefaults(room)),
    ])
  );
  const joinedChannelsByTopic: Record<string, JoinedPrivateChannelView[]> = {};
  let sequence = 0;
  let discoveryConfig: DiscoveryConfig = {
    mode: 'seeded_dht',
    connect_mode: 'direct_only',
    env_locked: false,
    seed_peers: [],
  };
  let communityNodeConfig: CommunityNodeConfig = { nodes: [] };
  let communityNodeStatuses: CommunityNodeNodeStatus[] = [];
  const syncStatus: SyncStatus = {
    connected: true,
    last_sync_ts: 1,
    peer_count: effectivePeerIds.length,
    pending_events: 0,
    status_detail: 'Connected to all configured peers',
    last_error: options?.globalLastError ?? null,
    configured_peers: ['peer-a'],
    subscribed_topics: ['kukuri:topic:demo'],
    topic_diagnostics: [
      {
        topic: 'kukuri:topic:demo',
        joined: true,
        peer_count: effectivePeerIds.length,
        connected_peers: ['peer-a'],
        assist_peer_ids: assistPeerIds,
        configured_peer_ids: ['peer-a'],
        missing_peer_ids: [],
        last_received_at: 1,
        status_detail: 'Connected to all configured peers for this topic',
        last_error: options?.topicLastError ?? null,
      },
    ],
    local_author_pubkey: 'f'.repeat(64),
    discovery: {
      mode: 'seeded_dht',
      connect_mode: 'direct_only',
      env_locked: false,
      configured_seed_peer_ids: [],
      bootstrap_seed_peer_ids: [],
      manual_ticket_peer_ids: [],
      connected_peer_ids: ['peer-a'],
      assist_peer_ids: assistPeerIds,
      local_endpoint_id: 'local-endpoint-a',
      last_discovery_error: null,
    },
  };
  let myProfile: Profile = {
    pubkey: syncStatus.local_author_pubkey,
    name: null,
    display_name: null,
    about: null,
    picture: null,
    updated_at: 0,
    ...options?.myProfile,
  };
  const authorSocialViews: Record<string, AuthorSocialView> = Object.fromEntries(
    Object.entries(options?.authorSocialViews ?? {}).map(([pubkey, view]) => [
      pubkey,
      withDefaultAuthorView(pubkey, view),
    ])
  );

  const api: DesktopApi = {
    async createPost(topic, content, replyTo, attachments, channelRef = { kind: 'public' }) {
      sequence += 1;
      const objectId = `${topic}-${sequence}`;
      const posts = postsByTopic[topic] ?? [];
      const channelId = channelRef.kind === 'private_channel' ? channelRef.channel_id : null;
      const rootId = replyTo
        ? posts.find((post) => post.object_id === replyTo)?.root_id ?? replyTo
        : objectId;
      const postAttachments: AttachmentView[] = (attachments ?? []).map((attachment, index) => ({
        hash: `${objectId}-attachment-${index}`,
        mime: attachment.mime,
        bytes: attachment.byte_size,
        role: attachment.role ?? 'image_original',
        status: 'Available',
      }));
      postsByTopic[topic] = [
        withSocialPostDefaults({
          object_id: objectId,
          envelope_id: `envelope-${sequence}`,
          author_pubkey: syncStatus.local_author_pubkey,
          following: false,
          followed_by: false,
          mutual: false,
          friend_of_friend: false,
          object_kind: replyTo ? 'comment' : 'post',
          content,
          content_status: 'Available',
          attachments: postAttachments,
          created_at: sequence,
          reply_to: replyTo ?? null,
          root_id: rootId,
          origin_topic_id: topic,
          channel_id: channelId,
          audience_label: channelId ? 'Private channel' : 'Public',
        }),
        ...posts,
      ];
      if (!channelId) {
        authorProfileTimelines[syncStatus.local_author_pubkey] = [
          withSocialPostDefaults({
            object_id: objectId,
            envelope_id: objectId,
            author_pubkey: syncStatus.local_author_pubkey,
            following: false,
            followed_by: false,
            mutual: false,
            friend_of_friend: false,
            object_kind: replyTo ? 'comment' : 'post',
            content,
            content_status: 'Available',
            attachments: postAttachments,
            created_at: sequence,
            reply_to: replyTo ?? null,
            root_id: rootId,
            origin_topic_id: topic,
            channel_id: null,
            audience_label: 'Public',
          }),
          ...(authorProfileTimelines[syncStatus.local_author_pubkey] ?? []).filter(
            (post) => post.object_id !== objectId
          ),
        ];
      }
      syncStatus.subscribed_topics = Array.from(new Set([...syncStatus.subscribed_topics, topic]));
      if (!syncStatus.topic_diagnostics.some((entry) => entry.topic === topic)) {
        syncStatus.topic_diagnostics.push({
          topic,
          joined: true,
          peer_count: 1,
          connected_peers: ['peer-a'],
          assist_peer_ids: assistPeerIds,
          configured_peer_ids: ['peer-a'],
          missing_peer_ids: [],
          last_received_at: sequence,
          status_detail: 'Connected to all configured peers for this topic',
          last_error: null,
        });
      }
      return objectId;
    },
    async createRepost(topic, sourceTopic, sourceObjectId, commentary) {
      sequence += 1;
      const objectId = `${topic}-repost-${sequence}`;
      const sourcePost = (postsByTopic[sourceTopic] ?? []).find((post) => post.object_id === sourceObjectId);
      if (!sourcePost || sourcePost.channel_id) {
        throw new Error('only public posts and comments can be reposted');
      }
      const normalizedCommentary = commentary?.trim() ? commentary.trim() : null;
      if (!normalizedCommentary) {
        const existing = (postsByTopic[topic] ?? []).find(
          (post) =>
            post.object_kind === 'repost' &&
            post.author_pubkey === syncStatus.local_author_pubkey &&
            post.repost_of?.source_object_id === sourceObjectId &&
            !post.repost_commentary
        );
        if (existing) {
          return existing.object_id;
        }
      }
      const repost = withSocialPostDefaults({
        object_id: objectId,
        envelope_id: `envelope-${sequence}`,
        author_pubkey: syncStatus.local_author_pubkey,
        following: false,
        followed_by: false,
        mutual: false,
        friend_of_friend: false,
        object_kind: 'repost',
        content: normalizedCommentary ?? '',
        content_status: 'Available',
        attachments: [],
        created_at: sequence,
        reply_to: null,
        root_id: null,
        published_topic_id: topic,
        origin_topic_id: topic,
        repost_of: {
          source_object_id: sourceObjectId,
          source_topic_id: sourceTopic,
          source_author_pubkey: sourcePost.author_pubkey,
          source_author_name: sourcePost.author_name ?? null,
          source_author_display_name: sourcePost.author_display_name ?? null,
          source_object_kind: sourcePost.object_kind,
          content: sourcePost.content,
          attachments: sourcePost.attachments.map((attachment) => ({ ...attachment })),
          reply_to: sourcePost.reply_to ?? null,
          root_id: sourcePost.root_id ?? null,
        },
        repost_commentary: normalizedCommentary,
        is_threadable: Boolean(normalizedCommentary),
        channel_id: null,
        audience_label: 'Public',
      });
      postsByTopic[topic] = [repost, ...(postsByTopic[topic] ?? [])];
      authorProfileTimelines[syncStatus.local_author_pubkey] = [
        repost,
        ...(authorProfileTimelines[syncStatus.local_author_pubkey] ?? []).filter(
          (post) => post.object_id !== objectId
        ),
      ];
      return objectId;
    },
    async listTimeline(topic, _cursor, _limit, scope: TimelineScope = { kind: 'public' }) {
      syncStatus.subscribed_topics = Array.from(new Set([...syncStatus.subscribed_topics, topic]));
      if (!syncStatus.topic_diagnostics.some((entry) => entry.topic === topic)) {
        syncStatus.topic_diagnostics.push({
          topic,
          joined: false,
          peer_count: assistPeerIds.length,
          connected_peers: [],
          assist_peer_ids: assistPeerIds,
          configured_peer_ids: [],
          missing_peer_ids: [],
          last_received_at: null,
          status_detail:
            assistPeerIds.length > 0
              ? `relay-assisted sync available via ${assistPeerIds.length} peer(s)`
              : 'No peers configured for this topic',
          last_error: null,
        });
      }
      return {
        items: filterChannelScopedItems(
          postsByTopic[topic] ?? [],
          scope,
          joinedChannelsByTopic[topic] ?? []
        ),
        next_cursor: null,
      };
    },
    async listThread(topic, threadId) {
      const posts = postsByTopic[topic] ?? [];
      return {
        items: posts.filter((post) => post.root_id === threadId || post.object_id === threadId),
        next_cursor: null,
      };
    },
    async listProfileTimeline(pubkey) {
      return {
        items: [...(authorProfileTimelines[pubkey] ?? [])],
        next_cursor: null,
      };
    },
    async getMyProfile() {
      if (options?.myProfileError) {
        throw new Error(options.myProfileError);
      }
      return myProfile;
    },
    async setMyProfile(input) {
      myProfile = { ...myProfile, ...input, updated_at: myProfile.updated_at + 1 };
      authorSocialViews[myProfile.pubkey] = withDefaultAuthorView(myProfile.pubkey, {
        name: myProfile.name ?? null,
        display_name: myProfile.display_name ?? null,
        about: myProfile.about ?? null,
        picture: myProfile.picture ?? null,
        updated_at: myProfile.updated_at,
      });
      return myProfile;
    },
    async followAuthor(pubkey) {
      const existing = withDefaultAuthorView(pubkey, authorSocialViews[pubkey]);
      const next = { ...existing, following: true, mutual: existing.followed_by };
      authorSocialViews[pubkey] = next;
      for (const topic of Object.keys(postsByTopic)) {
        postsByTopic[topic] = postsByTopic[topic].map((post) =>
          post.author_pubkey === pubkey
            ? { ...post, following: true, mutual: next.mutual, friend_of_friend: false }
            : post
        );
      }
      return next;
    },
    async unfollowAuthor(pubkey) {
      const existing = withDefaultAuthorView(pubkey, authorSocialViews[pubkey]);
      const next = { ...existing, following: false, mutual: false };
      authorSocialViews[pubkey] = next;
      for (const topic of Object.keys(postsByTopic)) {
        postsByTopic[topic] = postsByTopic[topic].map((post) =>
          post.author_pubkey === pubkey
            ? { ...post, following: false, mutual: false }
            : post
        );
      }
      return next;
    },
    async getAuthorSocialView(pubkey) {
      if (pubkey === myProfile.pubkey) {
        return withDefaultAuthorView(myProfile.pubkey, {
          name: myProfile.name ?? null,
          display_name: myProfile.display_name ?? null,
          about: myProfile.about ?? null,
          picture: myProfile.picture ?? null,
          updated_at: myProfile.updated_at,
        });
      }
      return withDefaultAuthorView(pubkey, authorSocialViews[pubkey]);
    },
    async listLiveSessions(topic, scope: TimelineScope = { kind: 'public' }) {
      return filterChannelScopedItems(
        liveSessionsByTopic[topic] ?? [],
        scope,
        joinedChannelsByTopic[topic] ?? []
      );
    },
    async createLiveSession(topic, title, description, channelRef = { kind: 'public' }) {
      sequence += 1;
      const sessionId = `live-${sequence}`;
      const channelId = channelRef.kind === 'private_channel' ? channelRef.channel_id : null;
      liveSessionsByTopic[topic] = [
        withLiveSessionDefaults({
          session_id: sessionId,
          host_pubkey: syncStatus.local_author_pubkey,
          title,
          description,
          status: 'Live',
          started_at: Date.now(),
          ended_at: null,
          viewer_count: 0,
          joined_by_me: false,
          channel_id: channelId,
          audience_label: channelId ? 'Private channel' : 'Public',
        }),
        ...(liveSessionsByTopic[topic] ?? []),
      ];
      return sessionId;
    },
    async endLiveSession(topic, sessionId) {
      liveSessionsByTopic[topic] = (liveSessionsByTopic[topic] ?? []).map((session) =>
        session.session_id === sessionId
          ? { ...session, status: 'Ended', ended_at: Date.now(), joined_by_me: false }
          : session
      );
    },
    async joinLiveSession(topic, sessionId) {
      liveSessionsByTopic[topic] = (liveSessionsByTopic[topic] ?? []).map((session) =>
        session.session_id === sessionId
          ? { ...session, joined_by_me: true, viewer_count: session.viewer_count + 1 }
          : session
      );
    },
    async leaveLiveSession(topic, sessionId) {
      liveSessionsByTopic[topic] = (liveSessionsByTopic[topic] ?? []).map((session) =>
        session.session_id === sessionId
          ? {
              ...session,
              joined_by_me: false,
              viewer_count: Math.max(0, session.viewer_count - 1),
            }
          : session
      );
    },
    async listGameRooms(topic, scope: TimelineScope = { kind: 'public' }) {
      return filterChannelScopedItems(
        gameRoomsByTopic[topic] ?? [],
        scope,
        joinedChannelsByTopic[topic] ?? []
      );
    },
    async createGameRoom(topic, title, description, participants, channelRef = { kind: 'public' }) {
      sequence += 1;
      const roomId = `game-${sequence}`;
      const channelId = channelRef.kind === 'private_channel' ? channelRef.channel_id : null;
      const scores: GameScoreView[] = participants.map((label, index) => ({
        participant_id: `participant-${index + 1}`,
        label,
        score: 0,
      }));
      gameRoomsByTopic[topic] = [
        withGameRoomDefaults({
          room_id: roomId,
          host_pubkey: syncStatus.local_author_pubkey,
          title,
          description,
          status: 'Waiting',
          phase_label: null,
          scores,
          updated_at: Date.now(),
          channel_id: channelId,
          audience_label: channelId ? 'Private channel' : 'Public',
        }),
        ...(gameRoomsByTopic[topic] ?? []),
      ];
      return roomId;
    },
    async createPrivateChannel(
      topic,
      label,
      audienceKind: ChannelAudienceKind = 'invite_only'
    ) {
      sequence += 1;
      const channelId = `channel-${sequence}`;
      const channel = withJoinedChannelDefaults({
        topic_id: topic,
        channel_id: channelId,
        label,
        creator_pubkey: syncStatus.local_author_pubkey,
        owner_pubkey: syncStatus.local_author_pubkey,
        audience_kind: audienceKind,
        is_owner: true,
        current_epoch_id: audienceKind === 'invite_only' ? 'legacy' : `epoch-${sequence}`,
        archived_epoch_ids: [],
        sharing_state: 'open',
        rotation_required: false,
        participant_count: audienceKind === 'invite_only' ? 0 : 1,
        stale_participant_count: 0,
      });
      joinedChannelsByTopic[topic] = [...(joinedChannelsByTopic[topic] ?? []), channel];
      return channel;
    },
    async exportPrivateChannelInvite(topic, channelId) {
      return `invite:${topic}:${channelId}`;
    },
    async importPrivateChannelInvite() {
      const preview: PrivateChannelInvitePreview = options?.invitePreview ?? {
        channel_id: 'channel-imported',
        topic_id: 'kukuri:topic:demo',
        channel_label: 'Imported',
        inviter_pubkey: syncStatus.local_author_pubkey,
        expires_at: null,
        namespace_secret_hex: 'a'.repeat(64),
      };
      joinedChannelsByTopic[preview.topic_id] = [
        ...(joinedChannelsByTopic[preview.topic_id] ?? []),
        withJoinedChannelDefaults({
          topic_id: preview.topic_id,
          channel_id: preview.channel_id,
          label: preview.channel_label,
          creator_pubkey: preview.inviter_pubkey,
          owner_pubkey: preview.inviter_pubkey,
          audience_kind: 'invite_only',
          is_owner: false,
          current_epoch_id: 'legacy',
          archived_epoch_ids: [],
          sharing_state: 'open',
          rotation_required: false,
          participant_count: 0,
          stale_participant_count: 0,
        }),
      ];
      return preview;
    },
    async exportFriendOnlyGrant(topic, channelId) {
      return `grant:${topic}:${channelId}`;
    },
    async importFriendOnlyGrant() {
      const preview: FriendOnlyGrantPreview = {
        channel_id: 'channel-friends',
        topic_id: 'kukuri:topic:demo',
        channel_label: 'Friends',
        owner_pubkey: syncStatus.local_author_pubkey,
        epoch_id: 'epoch-1',
        expires_at: null,
        namespace_secret_hex: 'b'.repeat(64),
      };
      joinedChannelsByTopic[preview.topic_id] = [
        ...(joinedChannelsByTopic[preview.topic_id] ?? []),
        withJoinedChannelDefaults({
          topic_id: preview.topic_id,
          channel_id: preview.channel_id,
          label: preview.channel_label,
          creator_pubkey: preview.owner_pubkey,
          owner_pubkey: preview.owner_pubkey,
          audience_kind: 'friend_only',
          is_owner: false,
          current_epoch_id: preview.epoch_id,
          archived_epoch_ids: [],
          sharing_state: 'open',
          rotation_required: false,
          participant_count: 1,
          stale_participant_count: 0,
        }),
      ];
      return preview;
    },
    async exportFriendPlusShare(topic, channelId) {
      return `share:${topic}:${channelId}`;
    },
    async importFriendPlusShare() {
      const preview: FriendPlusSharePreview = {
        channel_id: 'channel-friends-plus',
        topic_id: 'kukuri:topic:demo',
        channel_label: 'Friends+',
        owner_pubkey: syncStatus.local_author_pubkey,
        sponsor_pubkey: 'sponsor-pubkey-1234',
        epoch_id: 'epoch-plus-1',
        expires_at: null,
        namespace_secret_hex: 'c'.repeat(64),
        share_token_id: 'share-token-1',
      };
      joinedChannelsByTopic[preview.topic_id] = [
        ...(joinedChannelsByTopic[preview.topic_id] ?? []),
        withJoinedChannelDefaults({
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
        }),
      ];
      return preview;
    },
    async freezePrivateChannel(topic, channelId) {
      const channels = joinedChannelsByTopic[topic] ?? [];
      const next = channels.map((channel) =>
        channel.channel_id === channelId
          ? withJoinedChannelDefaults({ ...channel, sharing_state: 'frozen' })
          : channel
      );
      joinedChannelsByTopic[topic] = next;
      return next.find((channel) => channel.channel_id === channelId)!;
    },
    async rotatePrivateChannel(topic, channelId) {
      const channels = joinedChannelsByTopic[topic] ?? [];
      const next = channels.map((channel) =>
        channel.channel_id === channelId
          ? withJoinedChannelDefaults({
              ...channel,
              current_epoch_id: `${channel.current_epoch_id}-rotated`,
              archived_epoch_ids: [...channel.archived_epoch_ids, channel.current_epoch_id],
              rotation_required: false,
              stale_participant_count: 0,
            })
          : channel
      );
      joinedChannelsByTopic[topic] = next;
      return next.find((channel) => channel.channel_id === channelId)!;
    },
    async listJoinedPrivateChannels(topic) {
      return joinedChannelsByTopic[topic] ?? [];
    },
    async updateGameRoom(topic, roomId, status, phaseLabel, scores) {
      gameRoomsByTopic[topic] = (gameRoomsByTopic[topic] ?? []).map((room) =>
        room.room_id === roomId
          ? {
              ...room,
              status,
              phase_label: phaseLabel,
              scores: scores.map((score) => ({ ...score })),
              updated_at: Date.now(),
            }
          : room
      );
    },
    async getSyncStatus() {
      return cloneSyncStatus(syncStatus);
    },
    async getDiscoveryConfig() {
      return discoveryConfig;
    },
    async getCommunityNodeConfig() {
      return communityNodeConfig;
    },
    async getCommunityNodeStatuses() {
      return communityNodeStatuses;
    },
    async setCommunityNodeConfig(baseUrls) {
      communityNodeConfig = {
        nodes: baseUrls.map((baseUrl) => ({
          base_url: baseUrl,
          resolved_urls: null,
        })),
      };
      communityNodeStatuses = baseUrls.map((baseUrl) => ({
        base_url: baseUrl,
        auth_state: { authenticated: false, expires_at: null },
        consent_state: null,
        resolved_urls: null,
        last_error: null,
        restart_required: false,
      }));
      return communityNodeConfig;
    },
    async clearCommunityNodeConfig() {
      communityNodeConfig = { nodes: [] };
      communityNodeStatuses = [];
    },
    async authenticateCommunityNode(baseUrl) {
      communityNodeStatuses = communityNodeStatuses.map((status) =>
        status.base_url === baseUrl
          ? {
              ...status,
              auth_state: { authenticated: true, expires_at: Date.now() },
              consent_state: { all_required_accepted: false, items: [] },
            }
          : status
      );
      return communityNodeStatuses.find((status) => status.base_url === baseUrl)!;
    },
    async clearCommunityNodeToken(baseUrl) {
      communityNodeStatuses = communityNodeStatuses.map((status) =>
        status.base_url === baseUrl
          ? { ...status, auth_state: { authenticated: false, expires_at: null } }
          : status
      );
      return communityNodeStatuses.find((status) => status.base_url === baseUrl)!;
    },
    async getCommunityNodeConsentStatus(baseUrl) {
      return communityNodeStatuses.find((status) => status.base_url === baseUrl)!;
    },
    async acceptCommunityNodeConsents(baseUrl) {
      const resolvedUrls = { public_base_url: baseUrl, connectivity_urls: [baseUrl] };
      syncStatus.discovery.connect_mode = 'direct_or_relay';
      communityNodeStatuses = communityNodeStatuses.map((status) =>
        status.base_url === baseUrl
          ? {
              ...status,
              consent_state: { all_required_accepted: true, items: [] },
              resolved_urls: resolvedUrls,
              restart_required: false,
            }
          : status
      );
      communityNodeConfig = {
        nodes: communityNodeConfig.nodes.map((node) =>
          node.base_url === baseUrl ? { ...node, resolved_urls: resolvedUrls } : node
        ),
      };
      return communityNodeStatuses.find((status) => status.base_url === baseUrl)!;
    },
    async refreshCommunityNodeMetadata(baseUrl) {
      syncStatus.discovery.connect_mode = 'direct_or_relay';
      const resolvedUrls = { public_base_url: baseUrl, connectivity_urls: [baseUrl] };
      communityNodeStatuses = communityNodeStatuses.map((status) =>
        status.base_url === baseUrl
          ? { ...status, resolved_urls: resolvedUrls, restart_required: false }
          : status
      );
      communityNodeConfig = {
        nodes: communityNodeConfig.nodes.map((node) =>
          node.base_url === baseUrl ? { ...node, resolved_urls: resolvedUrls } : node
        ),
      };
      return communityNodeStatuses.find((status) => status.base_url === baseUrl)!;
    },
    async importPeerTicket() {},
    async setDiscoverySeeds(seedEntries) {
      discoveryConfig = {
        ...discoveryConfig,
        seed_peers: seedEntries.map((entry) => {
          const [endpointId, addrHint] = entry.split('@', 2);
          return {
            endpoint_id: endpointId,
            addr_hint: addrHint ?? null,
          };
        }),
      };
      syncStatus.discovery.configured_seed_peer_ids = discoveryConfig.seed_peers.map(
        (peer) => peer.endpoint_id
      );
      return discoveryConfig;
    },
    async unsubscribeTopic(topic) {
      delete postsByTopic[topic];
      delete liveSessionsByTopic[topic];
      delete gameRoomsByTopic[topic];
      delete joinedChannelsByTopic[topic];
      syncStatus.subscribed_topics = syncStatus.subscribed_topics.filter((value) => value !== topic);
      syncStatus.topic_diagnostics = syncStatus.topic_diagnostics.filter(
        (value) => value.topic !== topic
      );
    },
    async getLocalPeerTicket() {
      return 'peer1@127.0.0.1:7777';
    },
    async getBlobMediaPayload(_hash, mime): Promise<BlobMediaPayload | null> {
      return {
        bytes_base64: mime.startsWith('video/') ? 'ZmFrZS12aWRlbw==' : 'ZmFrZS1pbWFnZQ==',
        mime,
      };
    },
    async getBlobPreviewUrl() {
      return null;
    },
  };

  return api;
}
