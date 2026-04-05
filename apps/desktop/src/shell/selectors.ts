import {
  type AuthorSocialView,
  type ChannelAccessTokenPreview,
  type ChannelAudienceKind,
  type ChannelRef,
  type CommunityNodeConfig,
  type CommunityNodeNodeStatus,
  type DirectMessageConversationView,
  type DiscoveryConfig,
  type GameRoomStatus,
  type GameRoomView,
  type JoinedPrivateChannelView,
  type LiveSessionView,
  type PostView,
  type Profile,
  type ProfileInput,
  type ReactionStateView,
  type SyncStatus,
  type TimelineScope,
  type TopicSyncStatus,
} from '@/lib/api';
import i18n from '@/i18n';
import {
  formatLocalizedBytes,
  formatLocalizedNumber,
  formatLocalizedTime,
} from '@/i18n/format';

import {
  type GameEditorDraft,
  type KnownAuthorsByPubkey,
  PUBLIC_CHANNEL_REF,
  PUBLIC_TIMELINE_SCOPE,
} from './store';

function translate(key: string, options?: Record<string, unknown>): string {
  return i18n.t(key, options) as string;
}

export function formatBytes(bytes: number, locale?: string | null): string {
  return formatLocalizedBytes(bytes, locale);
}

export function shortPubkey(pubkey: string): string {
  return pubkey.slice(0, 12);
}

export function isHex64(value: string): boolean {
  return value.length === 64 && [...value].every((character) => character.match(/[0-9a-f]/i));
}

export function messageFromError(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback;
}

export function profileInputFromProfile(profile: Profile): ProfileInput {
  return {
    name: profile.name ?? '',
    display_name: profile.display_name ?? '',
    about: profile.about ?? '',
    picture: profile.picture ?? '',
    picture_upload: null,
    clear_picture: false,
  };
}

export function resolveProfilePictureSrc(
  profile:
    | Pick<Profile, 'picture' | 'picture_asset'>
    | Pick<AuthorSocialView, 'picture' | 'picture_asset'>
    | null
    | undefined,
  mediaObjectUrls: Record<string, string | null>
): string | null {
  const pictureAssetHash = profile?.picture_asset?.hash;
  if (pictureAssetHash && typeof mediaObjectUrls[pictureAssetHash] === 'string') {
    return mediaObjectUrls[pictureAssetHash];
  }
  return profile?.picture ?? null;
}

export function authorDisplayLabel(
  authorPubkey: string,
  displayName?: string | null,
  name?: string | null
): string {
  return displayName?.trim() || name?.trim() || shortPubkey(authorPubkey);
}

export function publishedTopicIdForPost(
  post: Pick<PostView, 'published_topic_id' | 'origin_topic_id'>
): string | null {
  return post.published_topic_id?.trim() || post.origin_topic_id?.trim() || null;
}

export function patchReactionStateIntoPosts(
  posts: PostView[],
  reactionState: ReactionStateView
): PostView[] {
  return posts.map((post) =>
    post.object_id === reactionState.target_object_id
      ? {
          ...post,
          reaction_summary: reactionState.reaction_summary,
          my_reactions: reactionState.my_reactions,
        }
      : post
  );
}

export function canCreateRepostFromPost(post: PostView): boolean {
  return (post.object_kind === 'post' || post.object_kind === 'comment') && !post.channel_id;
}

export function isQuoteRepost(
  post: Pick<PostView, 'object_kind' | 'repost_commentary'>
): boolean {
  return post.object_kind === 'repost' && Boolean(post.repost_commentary?.trim());
}

export function formatListLabel(values: string[]): string {
  return values.length > 0 ? values.join(', ') : translate('common:fallbacks.none');
}

export function formatLastReceivedLabel(
  timestamp?: number | null,
  locale?: string | null
): string {
  return timestamp
    ? formatLocalizedTime(timestamp, locale)
    : translate('common:fallbacks.noEvents');
}

export function strongestRelationshipLabel(relationship: {
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

export function mergeAuthorView(
  current: AuthorSocialView | null | undefined,
  incoming: Partial<AuthorSocialView> & { author_pubkey: string }
): AuthorSocialView {
  return {
    author_pubkey: incoming.author_pubkey,
    name: incoming.name ?? current?.name ?? null,
    display_name: incoming.display_name ?? current?.display_name ?? null,
    about: incoming.about ?? current?.about ?? null,
    picture: incoming.picture ?? current?.picture ?? null,
    picture_asset: incoming.picture_asset ?? current?.picture_asset ?? null,
    updated_at: incoming.updated_at ?? current?.updated_at ?? null,
    following: incoming.following ?? current?.following ?? false,
    followed_by: incoming.followed_by ?? current?.followed_by ?? false,
    mutual: incoming.mutual ?? current?.mutual ?? false,
    friend_of_friend: incoming.friend_of_friend ?? current?.friend_of_friend ?? false,
    friend_of_friend_via_pubkeys:
      incoming.friend_of_friend_via_pubkeys ?? current?.friend_of_friend_via_pubkeys ?? [],
    muted: incoming.muted ?? current?.muted ?? false,
  };
}

export function mergeKnownAuthors(
  current: KnownAuthorsByPubkey,
  incoming: Array<(Partial<AuthorSocialView> & { author_pubkey: string }) | null | undefined>
): KnownAuthorsByPubkey {
  let next = current;
  for (const view of incoming) {
    if (!view) {
      continue;
    }
    const merged = mergeAuthorView(next[view.author_pubkey], view);
    if (next === current) {
      next = { ...current };
    }
    next[view.author_pubkey] = merged;
  }
  return next;
}

export function authorViewFromDirectMessageConversation(
  conversation: DirectMessageConversationView
): AuthorSocialView {
  return {
    author_pubkey: conversation.peer_pubkey,
    name: conversation.peer_name ?? null,
    display_name: conversation.peer_display_name ?? null,
    about: null,
    picture: conversation.peer_picture ?? null,
    picture_asset: conversation.peer_picture_asset ?? null,
    updated_at: null,
    following: false,
    followed_by: false,
    mutual: conversation.status.mutual,
    friend_of_friend: false,
    friend_of_friend_via_pubkeys: [],
    muted: false,
  };
}

export function privateTimelineScope(channelId: string | null): TimelineScope {
  return channelId
    ? {
        kind: 'channel',
        channel_id: channelId,
      }
    : PUBLIC_TIMELINE_SCOPE;
}

export function privateComposeTarget(channelId: string | null): ChannelRef {
  return channelId
    ? {
        kind: 'private_channel',
        channel_id: channelId,
      }
    : PUBLIC_CHANNEL_REF;
}

export function audienceLabelForChannelRef(
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

export function audienceLabelForTimelineScope(
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

export function formatSeedPeer(peer: DiscoveryConfig['seed_peers'][number]): string {
  return peer.addr_hint ? `${peer.endpoint_id}@${peer.addr_hint}` : peer.endpoint_id;
}

export function seedPeersToEditorValue(config: DiscoveryConfig): string {
  return config.seed_peers.map((peer) => formatSeedPeer(peer)).join('\n');
}

export function communityNodesToEditorValue(config: CommunityNodeConfig): string {
  return config.nodes.map((node) => node.base_url).join('\n');
}

export function syncStatusBadgeTone(
  syncStatus: SyncStatus
): 'accent' | 'destructive' | 'warning' {
  if (syncStatus.last_error) {
    return 'destructive';
  }
  return syncStatus.connected ? 'accent' : 'warning';
}

export function syncStatusBadgeLabel(syncStatus: SyncStatus): string {
  if (syncStatus.last_error) {
    return translate('common:states.error');
  }
  return syncStatus.connected
    ? translate('common:states.connected')
    : translate('common:states.waiting');
}

export function topicConnectionLabel(diagnostic?: TopicSyncStatus): string {
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

export function communityNodeConnectivityUrlsLabel(
  status?: CommunityNodeNodeStatus
): string {
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

export function communityNodeNextStepLabel(status?: CommunityNodeNodeStatus): string {
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

export function communityNodeSessionActivationLabel(
  status?: CommunityNodeNodeStatus
): string {
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

export function communityNodeAuthLabel(status?: CommunityNodeNodeStatus): string {
  return status?.auth_state.authenticated
    ? `${translate('common:states.yes')} (${status.auth_state.expires_at ?? translate('common:states.unknown')})`
    : translate('common:states.no');
}

export function communityNodeConsentLabel(status?: CommunityNodeNodeStatus): string {
  if (!status?.consent_state) {
    return translate('common:states.unknown');
  }
  return status.consent_state.all_required_accepted
    ? translate('common:states.accepted')
    : translate('common:states.required');
}

export function translateTopicConnectionText(label: string): string {
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

export function translateAudienceKindLabel(kind: ChannelAudienceKind): string {
  return translate(`channels:audienceOptions.${kind}`);
}

export function translateLiveStatus(status: LiveSessionView['status']): string {
  return translate(`live:statuses.${status}`);
}

export function translateGameStatus(status: GameRoomStatus): string {
  return translate(`game:statuses.${status}`);
}

export function formatCount(value: number): string {
  return formatLocalizedNumber(value);
}

export function localizeAudienceLabel(label: string): string {
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

export function mergeCommunityNodeStatus(
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

export function mergeCommunityNodeStatuses(
  previous: CommunityNodeNodeStatus[],
  next: CommunityNodeNodeStatus[]
): CommunityNodeNodeStatus[] {
  const previousByBaseUrl = Object.fromEntries(
    previous.map((status) => [status.base_url, status])
  ) as Record<string, CommunityNodeNodeStatus>;
  return next.map((status) =>
    mergeCommunityNodeStatus(previousByBaseUrl[status.base_url], status)
  );
}

export function upsertCommunityNodeStatus(
  current: CommunityNodeNodeStatus[],
  next: CommunityNodeNodeStatus
): CommunityNodeNodeStatus[] {
  const previous = current.find((status) => status.base_url === next.base_url);
  const merged = mergeCommunityNodeStatus(previous, next);
  const remaining = current.filter((status) => status.base_url !== next.base_url);
  return [...remaining, merged].sort((left, right) =>
    left.base_url.localeCompare(right.base_url)
  );
}

export function syncCommunityNodeConfigWithStatus(
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

export function createGameEditorDraft(room: GameRoomView): GameEditorDraft {
  return {
    status: room.status,
    phase_label: room.phase_label ?? '',
    scores: Object.fromEntries(
      room.scores.map((score) => [score.participant_id, String(score.score)])
    ),
  };
}

export function upsertJoinedChannel(
  channels: JoinedPrivateChannelView[],
  nextChannel: JoinedPrivateChannelView
): JoinedPrivateChannelView[] {
  const remaining = channels.filter((channel) => channel.channel_id !== nextChannel.channel_id);
  return [...remaining, nextChannel];
}

export function joinedChannelFromAccessTokenPreview(
  preview: ChannelAccessTokenPreview
): JoinedPrivateChannelView {
  if (preview.kind === 'grant') {
    return {
      topic_id: preview.topic_id,
      channel_id: preview.channel_id,
      label: preview.channel_label,
      creator_pubkey: preview.owner_pubkey,
      owner_pubkey: preview.owner_pubkey,
      joined_via_pubkey: preview.sponsor_pubkey ?? null,
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
  if (preview.kind === 'share') {
    return {
      topic_id: preview.topic_id,
      channel_id: preview.channel_id,
      label: preview.channel_label,
      creator_pubkey: preview.owner_pubkey,
      owner_pubkey: preview.owner_pubkey,
      joined_via_pubkey: preview.sponsor_pubkey ?? null,
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
  return {
    topic_id: preview.topic_id,
    channel_id: preview.channel_id,
    label: preview.channel_label,
    creator_pubkey: preview.inviter_pubkey ?? preview.owner_pubkey,
    owner_pubkey: preview.owner_pubkey,
    joined_via_pubkey: preview.inviter_pubkey ?? null,
    audience_kind: 'invite_only',
    is_owner: false,
    current_epoch_id: preview.epoch_id,
    archived_epoch_ids: [],
    sharing_state: 'open',
    rotation_required: false,
    participant_count: 1,
    stale_participant_count: 0,
  };
}
