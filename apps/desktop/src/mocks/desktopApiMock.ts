import {
  type AttachmentView,
  type AuthorSocialView,
  type BlobMediaPayload,
  type BookmarkedCustomReactionView,
  type BookmarkedPostView,
  type ChannelAccessTokenExport,
  type ChannelAccessTokenPreview,
  type ChannelAudienceKind,
  type CommunityNodeConfig,
  type CommunityNodeNodeStatus,
  type CustomReactionAssetView,
  type CustomReactionCropRect,
  type DesktopApi,
  type DirectMessageConversationView,
  type DirectMessageMessageView,
  type DirectMessageStatusView,
  type DirectMessageTimelineView,
  type DiscoveryConfig,
  type FriendOnlyGrantPreview,
  type FriendPlusSharePreview,
  type GameRoomView,
  type GameScoreView,
  type JoinedPrivateChannelView,
  type LiveSessionView,
  type NotificationStatusView,
  type NotificationView,
  type PostView,
  type PrivateChannelInvitePreview,
  type Profile,
  type ReactionKeyInput,
  type ReactionStateView,
  type RecentReactionView,
  type SyncStatus,
  type SocialConnectionKind,
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
  notifications?: NotificationView[];
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
    reaction_summary: [...(post.reaction_summary ?? [])],
    my_reactions: [...(post.my_reactions ?? [])],
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

function cloneBookmarkedPost(view: BookmarkedPostView): BookmarkedPostView {
  return {
    bookmarked_at: view.bookmarked_at,
    post: withSocialPostDefaults({
      ...view.post,
      attachments: view.post.attachments.map((attachment) => ({ ...attachment })),
      repost_of: view.post.repost_of
        ? {
            ...view.post.repost_of,
            attachments: view.post.repost_of.attachments.map((attachment) => ({ ...attachment })),
          }
        : null,
      reaction_summary: [...(view.post.reaction_summary ?? [])],
      my_reactions: [...(view.post.my_reactions ?? [])],
    }),
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
    picture_asset: null,
    updated_at: null,
    following: false,
    followed_by: false,
    mutual: false,
    friend_of_friend: false,
    friend_of_friend_via_pubkeys: [],
    muted: false,
    ...view,
  };
}

function cloneAuthorView(view: AuthorSocialView): AuthorSocialView {
  return {
    ...view,
    picture_asset: view.picture_asset ? { ...view.picture_asset } : null,
    friend_of_friend_via_pubkeys: [...view.friend_of_friend_via_pubkeys],
  };
}

function cloneNotification(view: NotificationView): NotificationView {
  return {
    ...view,
    actor_picture_asset: view.actor_picture_asset ? { ...view.actor_picture_asset } : null,
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

function compareAuthorViews(left: AuthorSocialView, right: AuthorSocialView) {
  const normalize = (value?: string | null) => value?.trim().toLocaleLowerCase() ?? '';
  const leftDisplay = normalize(left.display_name);
  const rightDisplay = normalize(right.display_name);
  if (leftDisplay !== rightDisplay) {
    if (!leftDisplay) {
      return 1;
    }
    if (!rightDisplay) {
      return -1;
    }
    return leftDisplay.localeCompare(rightDisplay);
  }
  const leftName = normalize(left.name);
  const rightName = normalize(right.name);
  if (leftName !== rightName) {
    if (!leftName) {
      return 1;
    }
    if (!rightName) {
      return -1;
    }
    return leftName.localeCompare(rightName);
  }
  return left.author_pubkey.localeCompare(right.author_pubkey);
}

function normalizedReactionKey(reactionKey: ReactionKeyInput): string {
  return reactionKey.kind === 'emoji'
    ? `emoji:${reactionKey.emoji.trim()}`
    : `custom_asset:${reactionKey.asset.asset_id}`;
}

function reactionStateForPost(post: PostView): ReactionStateView {
  return {
    target_object_id: post.object_id,
    source_replica_id: post.channel_id ?? 'public',
    reaction_summary: [...(post.reaction_summary ?? [])],
    my_reactions: [...(post.my_reactions ?? [])],
  };
}

function recentReactionFromInput(
  reactionKey: ReactionKeyInput,
  updatedAt: number
): RecentReactionView {
  return reactionKey.kind === 'emoji'
    ? {
        reaction_key_kind: 'emoji',
        normalized_reaction_key: `emoji:${reactionKey.emoji.trim()}`,
        emoji: reactionKey.emoji.trim(),
        custom_asset: null,
        updated_at: updatedAt,
      }
    : {
        reaction_key_kind: 'custom_asset',
        normalized_reaction_key: `custom_asset:${reactionKey.asset.asset_id}`,
        emoji: null,
        custom_asset: { ...reactionKey.asset },
        updated_at: updatedAt,
      };
}

function pushRecentReaction(
  current: RecentReactionView[],
  reactionKey: ReactionKeyInput,
  updatedAt: number
): RecentReactionView[] {
  const next = recentReactionFromInput(reactionKey, updatedAt);
  return [
    next,
    ...current.filter((item) => item.normalized_reaction_key !== next.normalized_reaction_key),
  ].slice(0, 8);
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
    picture_asset: null,
    updated_at: 0,
    ...options?.myProfile,
  };
  const authorSocialViews: Record<string, AuthorSocialView> = Object.fromEntries(
    Object.entries(options?.authorSocialViews ?? {}).map(([pubkey, view]) => [
      pubkey,
      withDefaultAuthorView(pubkey, view),
    ])
  );
  const directMessageMessagesByPeer: Record<string, DirectMessageMessageView[]> = {};
  const openedDirectMessagePeers = new Set<string>();
  const ownedCustomReactionAssets: CustomReactionAssetView[] = [];
  const bookmarkedCustomReactionAssets: BookmarkedCustomReactionView[] = [];
  const bookmarkedPosts: BookmarkedPostView[] = [];
  let notifications: NotificationView[] = (options?.notifications ?? []).map(cloneNotification);
  let recentReactions: RecentReactionView[] = [];

  function mutedAuthorPubkeys() {
    return new Set(
      Object.values(authorSocialViews)
        .filter((view) => view.muted)
        .map((view) => view.author_pubkey)
    );
  }

  function withCurrentRelationship(post: PostView): PostView {
    const author = authorSocialViews[post.author_pubkey];
    if (!author) {
      return withSocialPostDefaults(post);
    }
    return withSocialPostDefaults({
      ...post,
      following: author.following ?? post.following,
      followed_by: author.followed_by ?? post.followed_by,
      mutual: author.mutual ?? post.mutual,
      friend_of_friend: author.friend_of_friend ?? post.friend_of_friend,
    });
  }

  function isVisiblePost(post: PostView): boolean {
    const muted = mutedAuthorPubkeys();
    return (
      !muted.has(post.author_pubkey) &&
      !(post.repost_of && muted.has(post.repost_of.source_author_pubkey))
    );
  }

  function visibleTimelineItems(items: PostView[]): PostView[] {
    return items.map(withCurrentRelationship).filter(isVisiblePost);
  }

  function listConnections(kind: SocialConnectionKind): AuthorSocialView[] {
    const items = Object.values(authorSocialViews)
      .filter((view) => {
        if (kind === 'following') {
          return view.following;
        }
        if (kind === 'followed') {
          return view.followed_by;
        }
        return view.muted;
      })
      .map(cloneAuthorView);
    items.sort(compareAuthorViews);
    return items;
  }

  function directMessageStatusFor(pubkey: string): DirectMessageStatusView {
    const author = withDefaultAuthorView(pubkey, authorSocialViews[pubkey]);
    return {
      peer_pubkey: pubkey,
      dm_id: [syncStatus.local_author_pubkey, pubkey].sort().join(':'),
      mutual: author.mutual,
      send_enabled: author.mutual,
      peer_count: author.mutual ? 1 : 0,
      pending_outbox_count: 0,
    };
  }

  function directMessageConversationFor(pubkey: string): DirectMessageConversationView {
    const messages = directMessageMessagesByPeer[pubkey] ?? [];
    const latest = [...messages].sort(
      (left, right) => right.created_at - left.created_at || right.message_id.localeCompare(left.message_id)
    )[0];
    const author = withDefaultAuthorView(pubkey, authorSocialViews[pubkey]);
    return {
      dm_id: directMessageStatusFor(pubkey).dm_id,
      peer_pubkey: pubkey,
      peer_name: author.name ?? null,
      peer_display_name: author.display_name ?? null,
      peer_picture: author.picture ?? null,
      peer_picture_asset: author.picture_asset ?? null,
      updated_at: latest?.created_at ?? 0,
      last_message_at: latest?.created_at ?? null,
      last_message_id: latest?.message_id ?? null,
      last_message_preview:
        latest?.text?.trim() ||
        (latest?.attachments.some((attachment) => attachment.role === 'video_manifest')
          ? '[Video]'
          : latest?.attachments.length
            ? '[Image]'
            : null),
      status: directMessageStatusFor(pubkey),
    };
  }

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
    async toggleReaction(targetTopicId, targetObjectId, reactionKey) {
      const normalizedKey = normalizedReactionKey(reactionKey);
      const posts = postsByTopic[targetTopicId] ?? [];
      const index = posts.findIndex((post) => post.object_id === targetObjectId);
      if (index < 0) {
        throw new Error('reaction target was not found');
      }
      const post = withSocialPostDefaults(posts[index]);
      const myReactions = new Map(
        (post.my_reactions ?? []).map((reaction) => [reaction.normalized_reaction_key, reaction])
      );
      const summary = new Map(
        (post.reaction_summary ?? []).map((reaction) => [
          reaction.normalized_reaction_key,
          { ...reaction },
        ])
      );
      if (myReactions.has(normalizedKey)) {
        myReactions.delete(normalizedKey);
        const current = summary.get(normalizedKey);
        if (current) {
          const nextCount = current.count - 1;
          if (nextCount <= 0) {
            summary.delete(normalizedKey);
          } else {
            current.count = nextCount;
          }
        }
      } else {
        const keyView =
          reactionKey.kind === 'emoji'
            ? {
                reaction_key_kind: 'emoji',
                normalized_reaction_key: normalizedKey,
                emoji: reactionKey.emoji.trim(),
                custom_asset: null,
              }
            : {
                reaction_key_kind: 'custom_asset',
                normalized_reaction_key: normalizedKey,
                emoji: null,
                custom_asset: { ...reactionKey.asset },
              };
        myReactions.set(normalizedKey, keyView);
        const current = summary.get(normalizedKey);
        summary.set(normalizedKey, {
          ...(current ?? keyView),
          count: (current?.count ?? 0) + 1,
        });
      }
      const nextPost = withSocialPostDefaults({
        ...post,
        reaction_summary: Array.from(summary.values()),
        my_reactions: Array.from(myReactions.values()),
      });
      postsByTopic[targetTopicId] = posts.map((candidate) =>
        candidate.object_id === targetObjectId ? nextPost : candidate
      );
      recentReactions = pushRecentReaction(recentReactions, reactionKey, Date.now());
      return reactionStateForPost(nextPost);
    },
    async listMyCustomReactionAssets() {
      return ownedCustomReactionAssets.map((asset) => ({ ...asset }));
    },
    async listRecentReactions(limit = 8) {
      return recentReactions.slice(0, limit).map((reaction) => ({
        ...reaction,
        custom_asset: reaction.custom_asset ? { ...reaction.custom_asset } : null,
      }));
    },
    async createCustomReactionAsset(upload, cropRect: CustomReactionCropRect, searchKey: string) {
      void upload;
      void cropRect;
      sequence += 1;
      const asset: CustomReactionAssetView = {
        asset_id: `asset-${sequence}`,
        owner_pubkey: syncStatus.local_author_pubkey,
        blob_hash: `blob-${sequence}`,
        search_key: searchKey.trim() || `asset-${sequence}`,
        mime: 'image/png',
        bytes: 128,
        width: 128,
        height: 128,
      };
      ownedCustomReactionAssets.unshift(asset);
      return { ...asset };
    },
    async listBookmarkedCustomReactions() {
      return bookmarkedCustomReactionAssets.map((asset) => ({ ...asset }));
    },
    async bookmarkCustomReaction(asset) {
      const existing = bookmarkedCustomReactionAssets.find(
        (candidate) => candidate.asset_id === asset.asset_id
      );
      if (existing) {
        return { ...existing };
      }
      const bookmarked = { ...asset };
      bookmarkedCustomReactionAssets.unshift(bookmarked);
      return bookmarked;
    },
    async removeBookmarkedCustomReaction(assetId) {
      const index = bookmarkedCustomReactionAssets.findIndex((asset) => asset.asset_id === assetId);
      if (index >= 0) {
        bookmarkedCustomReactionAssets.splice(index, 1);
      }
    },
    async listBookmarkedPosts() {
      return bookmarkedPosts
        .filter((item) => isVisiblePost(item.post))
        .map((item) =>
          cloneBookmarkedPost({
            ...item,
            post: withCurrentRelationship(item.post),
          })
        );
    },
    async bookmarkPost(topic, objectId) {
      const existing = bookmarkedPosts.find((item) => item.post.object_id === objectId);
      if (existing) {
        return cloneBookmarkedPost(existing);
      }
      const post = (postsByTopic[topic] ?? []).find((candidate) => candidate.object_id === objectId);
      if (!post) {
        throw new Error('bookmark target was not found');
      }
      const bookmarked: BookmarkedPostView = {
        bookmarked_at: Date.now(),
        post: withSocialPostDefaults({
          ...post,
          attachments: post.attachments.map((attachment) => ({ ...attachment })),
          repost_of: post.repost_of
            ? {
                ...post.repost_of,
                attachments: post.repost_of.attachments.map((attachment) => ({ ...attachment })),
              }
            : null,
        }),
      };
      bookmarkedPosts.unshift(bookmarked);
      bookmarkedPosts.sort(
        (left, right) =>
          right.bookmarked_at - left.bookmarked_at ||
          right.post.object_id.localeCompare(left.post.object_id)
      );
      return cloneBookmarkedPost(bookmarked);
    },
    async removeBookmarkedPost(objectId) {
      const index = bookmarkedPosts.findIndex((item) => item.post.object_id === objectId);
      if (index >= 0) {
        bookmarkedPosts.splice(index, 1);
      }
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
        items: visibleTimelineItems(
          filterChannelScopedItems(postsByTopic[topic] ?? [], scope, joinedChannelsByTopic[topic] ?? [])
        ),
        next_cursor: null,
      };
    },
    async listThread(topic, threadId) {
      const posts = postsByTopic[topic] ?? [];
      return {
        items: visibleTimelineItems(
          posts.filter((post) => post.root_id === threadId || post.object_id === threadId)
        ),
        next_cursor: null,
      };
    },
    async listProfileTimeline(pubkey) {
      return {
        items: visibleTimelineItems([...(authorProfileTimelines[pubkey] ?? [])]),
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
      const nextPictureAsset =
        input.clear_picture
          ? null
          : input.picture_upload
            ? {
                hash: `profile-avatar-${myProfile.updated_at + 1}`,
                mime: input.picture_upload.mime,
                bytes: input.picture_upload.byte_size,
                role: 'profile_avatar' as const,
              }
            : myProfile.picture_asset ?? null;
      myProfile = {
        ...myProfile,
        ...input,
        picture: input.clear_picture ? null : (input.picture ?? myProfile.picture ?? null),
        picture_asset: nextPictureAsset,
        updated_at: myProfile.updated_at + 1,
      };
      authorSocialViews[myProfile.pubkey] = withDefaultAuthorView(myProfile.pubkey, {
        name: myProfile.name ?? null,
        display_name: myProfile.display_name ?? null,
        about: myProfile.about ?? null,
        picture: myProfile.picture ?? null,
        picture_asset: myProfile.picture_asset ?? null,
        updated_at: myProfile.updated_at,
      });
      return myProfile;
    },
    async followAuthor(pubkey) {
      const existing = withDefaultAuthorView(pubkey, authorSocialViews[pubkey]);
      const next = { ...existing, following: true, mutual: existing.followed_by };
      authorSocialViews[pubkey] = next;
      return cloneAuthorView(next);
    },
    async unfollowAuthor(pubkey) {
      const existing = withDefaultAuthorView(pubkey, authorSocialViews[pubkey]);
      const next = { ...existing, following: false, mutual: false };
      authorSocialViews[pubkey] = next;
      return cloneAuthorView(next);
    },
    async getAuthorSocialView(pubkey) {
      if (pubkey === myProfile.pubkey) {
        return cloneAuthorView(withDefaultAuthorView(myProfile.pubkey, {
          name: myProfile.name ?? null,
          display_name: myProfile.display_name ?? null,
          about: myProfile.about ?? null,
          picture: myProfile.picture ?? null,
          picture_asset: myProfile.picture_asset ?? null,
          updated_at: myProfile.updated_at,
        }));
      }
      return cloneAuthorView(withDefaultAuthorView(pubkey, authorSocialViews[pubkey]));
    },
    async muteAuthor(pubkey) {
      const existing = withDefaultAuthorView(pubkey, authorSocialViews[pubkey]);
      const next = { ...existing, muted: true };
      authorSocialViews[pubkey] = next;
      return cloneAuthorView(next);
    },
    async unmuteAuthor(pubkey) {
      const existing = withDefaultAuthorView(pubkey, authorSocialViews[pubkey]);
      const next = { ...existing, muted: false };
      authorSocialViews[pubkey] = next;
      return cloneAuthorView(next);
    },
    async listSocialConnections(kind) {
      return listConnections(kind);
    },
    async listNotifications() {
      return notifications.map(cloneNotification);
    },
    async markNotificationRead(notificationId) {
      notifications = notifications.map((notification) =>
        notification.notification_id === notificationId && !notification.read_at
          ? { ...notification, read_at: Date.now() }
          : notification
      );
      return {
        unread_count: notifications.filter((notification) => !notification.read_at).length,
      } satisfies NotificationStatusView;
    },
    async markAllNotificationsRead() {
      const readAt = Date.now();
      notifications = notifications.map((notification) =>
        notification.read_at ? notification : { ...notification, read_at: readAt }
      );
      return {
        unread_count: 0,
      } satisfies NotificationStatusView;
    },
    async getNotificationStatus() {
      return {
        unread_count: notifications.filter((notification) => !notification.read_at).length,
      } satisfies NotificationStatusView;
    },
    async openDirectMessage(pubkey) {
      const status = directMessageStatusFor(pubkey);
      if (!status.send_enabled && !openedDirectMessagePeers.has(pubkey)) {
        throw new Error('direct message requires a mutual relationship');
      }
      openedDirectMessagePeers.add(pubkey);
      return directMessageConversationFor(pubkey);
    },
    async listDirectMessages() {
      return [...openedDirectMessagePeers]
        .map((pubkey) => directMessageConversationFor(pubkey))
        .sort(
          (left, right) =>
            (right.last_message_at ?? right.updated_at) - (left.last_message_at ?? left.updated_at) ||
            left.peer_pubkey.localeCompare(right.peer_pubkey)
        );
    },
    async listDirectMessageMessages(pubkey) {
      return {
        items: [...(directMessageMessagesByPeer[pubkey] ?? [])].sort(
          (left, right) => right.created_at - left.created_at || right.message_id.localeCompare(left.message_id)
        ),
        next_cursor: null,
      } satisfies DirectMessageTimelineView;
    },
    async sendDirectMessage(pubkey, text, attachments = [], replyToMessageId) {
      const status = directMessageStatusFor(pubkey);
      if (!status.send_enabled) {
        throw new Error('direct message requires a mutual relationship');
      }
      if (!text?.trim() && attachments.length === 0) {
        throw new Error('direct message requires text or attachment');
      }
      openedDirectMessagePeers.add(pubkey);
      sequence += 1;
      const messageId = `dm-${sequence}`;
      const messageAttachments: AttachmentView[] = attachments.map((attachment, index) => ({
        hash: `${messageId}-attachment-${index}`,
        mime: attachment.mime,
        bytes: attachment.byte_size,
        role: attachment.role ?? 'image_original',
        status: 'Available',
      }));
      const nextMessage: DirectMessageMessageView = {
        dm_id: status.dm_id,
        message_id: messageId,
        sender_pubkey: syncStatus.local_author_pubkey,
        recipient_pubkey: pubkey,
        created_at: Date.now(),
        text: text?.trim() ?? '',
        reply_to_message_id: replyToMessageId ?? null,
        attachments: messageAttachments,
        outgoing: true,
        delivered: true,
      };
      directMessageMessagesByPeer[pubkey] = [
        nextMessage,
        ...(directMessageMessagesByPeer[pubkey] ?? []).filter(
          (message) => message.message_id !== nextMessage.message_id
        ),
      ];
      return messageId;
    },
    async deleteDirectMessageMessage(pubkey, messageId) {
      directMessageMessagesByPeer[pubkey] = (directMessageMessagesByPeer[pubkey] ?? []).filter(
        (message) => message.message_id !== messageId
      );
    },
    async clearDirectMessage(pubkey) {
      directMessageMessagesByPeer[pubkey] = [];
      openedDirectMessagePeers.add(pubkey);
    },
    async getDirectMessageStatus(pubkey) {
      return directMessageStatusFor(pubkey);
    },
    async listLiveSessions(topic, scope: TimelineScope = { kind: 'public' }) {
      const muted = mutedAuthorPubkeys();
      return filterChannelScopedItems(
        liveSessionsByTopic[topic] ?? [],
        scope,
        joinedChannelsByTopic[topic] ?? []
      ).filter((session) => !muted.has(session.host_pubkey));
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
      const muted = mutedAuthorPubkeys();
      return filterChannelScopedItems(
        gameRoomsByTopic[topic] ?? [],
        scope,
        joinedChannelsByTopic[topic] ?? []
      ).filter((room) => !muted.has(room.host_pubkey));
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
        current_epoch_id: `epoch-${sequence}`,
        archived_epoch_ids: [],
        sharing_state: 'open',
        rotation_required: false,
        participant_count: 1,
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
        owner_pubkey: syncStatus.local_author_pubkey,
        epoch_id: 'epoch-imported-1',
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
          owner_pubkey: preview.owner_pubkey,
          audience_kind: 'invite_only',
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
    async exportChannelAccessToken(topic, channelId) {
      const channel = (joinedChannelsByTopic[topic] ?? []).find(
        (item) => item.channel_id === channelId
      );
      if (!channel) {
        throw new Error('private channel is not joined');
      }
      const kind =
        channel.audience_kind === 'invite_only'
          ? 'invite'
          : channel.audience_kind === 'friend_only'
            ? 'grant'
            : 'share';
      return {
        kind,
        token: `${kind}:${topic}:${channelId}`,
      } satisfies ChannelAccessTokenExport;
    },
    async importChannelAccessToken(token) {
      if (token.startsWith('grant:')) {
        const preview = await this.importFriendOnlyGrant(token);
        return {
          kind: 'grant',
          topic_id: preview.topic_id,
          channel_id: preview.channel_id,
          channel_label: preview.channel_label,
          owner_pubkey: preview.owner_pubkey,
          inviter_pubkey: null,
          sponsor_pubkey: preview.owner_pubkey,
          epoch_id: preview.epoch_id,
        } satisfies ChannelAccessTokenPreview;
      }
      if (token.startsWith('share:')) {
        const preview = await this.importFriendPlusShare(token);
        return {
          kind: 'share',
          topic_id: preview.topic_id,
          channel_id: preview.channel_id,
          channel_label: preview.channel_label,
          owner_pubkey: preview.owner_pubkey,
          inviter_pubkey: null,
          sponsor_pubkey: preview.sponsor_pubkey,
          epoch_id: preview.epoch_id,
        } satisfies ChannelAccessTokenPreview;
      }
      const preview = await this.importPrivateChannelInvite(token);
      return {
        kind: 'invite',
        topic_id: preview.topic_id,
        channel_id: preview.channel_id,
        channel_label: preview.channel_label,
        owner_pubkey: preview.owner_pubkey,
        inviter_pubkey: preview.inviter_pubkey,
        sponsor_pubkey: null,
        epoch_id: preview.epoch_id,
      } satisfies ChannelAccessTokenPreview;
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
