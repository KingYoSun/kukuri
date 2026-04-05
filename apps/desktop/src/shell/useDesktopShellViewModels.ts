import {
  type SyntheticEvent,
  useCallback,
  useMemo,
} from 'react';

import type {
  AuthorDetailView,
  ComposerDraftMediaView,
  PostCardView,
  ThreadPanelState,
  TopicDiagnosticSummary,
} from '@/components/core/types';
import type {
  AppearancePanelView,
  CommunityNodePanelView,
  ConnectivityPanelView,
  DiscoveryPanelView,
  ReactionsPanelView,
} from '@/components/settings/types';
import type {
  ChannelAudienceOption,
  GameDraftView,
  PrivateChannelListItemView,
} from '@/components/extended/types';
import type { TimelineWorkspaceView } from '@/components/shell/types';
import type {
  GameRoomView,
  JoinedPrivateChannelView,
  LiveSessionView,
  PostView,
  TopicSyncStatus,
} from '@/lib/api';
import type { DesktopTheme } from '@/lib/theme';

import {
  PRIMARY_SECTION_ITEMS,
  SETTINGS_SECTION_COPY,
} from '@/shell/routes';
import {
  type SupportedLocale,
} from '@/i18n';
import {
  DEFAULT_ASYNC_PANEL_STATE,
  useDesktopShellFieldSetter,
  useDesktopShellStore,
} from '@/shell/store';
import {
  logMediaDebug,
  mediaElementDebugFields,
  selectPrimaryImage,
  selectVideoManifest,
  selectVideoPoster,
} from '@/shell/media';
import {
  audienceLabelForChannelRef,
  authorDisplayLabel,
  canCreateRepostFromPost,
  communityNodeAuthLabel,
  communityNodeConnectivityUrlsLabel,
  communityNodeConsentLabel,
  communityNodeNextStepLabel,
  communityNodeSessionActivationLabel,
  createGameEditorDraft,
  formatBytes,
  formatCount,
  formatLastReceivedLabel,
  formatListLabel,
  isQuoteRepost,
  localizeAudienceLabel,
  publishedTopicIdForPost,
  resolveProfilePictureSrc,
  shortPubkey,
  strongestRelationshipLabel,
  syncStatusBadgeLabel,
  topicConnectionLabel,
  translateTopicConnectionText,
} from '@/shell/selectors';

type UseDesktopShellViewModelsArgs = {
  t: (key: string, options?: Record<string, unknown>) => string;
  translate: (key: string, options?: Record<string, unknown>) => string;
  locale: SupportedLocale;
  theme: DesktopTheme;
  profileAvatarPreviewUrl: string | null;
};

const EMPTY_POSTS: PostView[] = [];
const EMPTY_LIVE_SESSIONS: LiveSessionView[] = [];
const EMPTY_GAME_ROOMS: GameRoomView[] = [];
const EMPTY_JOINED_CHANNELS: JoinedPrivateChannelView[] = [];

export function useDesktopShellViewModels({
  t,
  translate,
  locale,
  theme,
  profileAvatarPreviewUrl,
}: UseDesktopShellViewModelsArgs) {
  const state = useDesktopShellStore();
  const activeTopic = state.activeTopic;
  const selectedPrivateChannelId = state.selectedChannelIdByTopic[activeTopic] ?? null;
  const activeJoinedChannels = state.joinedChannelsByTopic[activeTopic] ?? EMPTY_JOINED_CHANNELS;
  const activeTimeline = state.timelinesByTopic[activeTopic] ?? EMPTY_POSTS;
  const activeLiveSessions = state.liveSessionsByTopic[activeTopic] ?? EMPTY_LIVE_SESSIONS;
  const activeGameRooms = state.gameRoomsByTopic[activeTopic] ?? EMPTY_GAME_ROOMS;
  const activeTimelineScope = state.timelineScopeByTopic[activeTopic] ?? { kind: 'public' as const };
  const activeComposeChannel = state.repostTarget
    ? { kind: 'public' as const }
    : state.replyTarget?.channel_id
      ? {
          kind: 'private_channel' as const,
          channel_id: state.replyTarget.channel_id,
        }
      : state.composeChannelByTopic[activeTopic] ?? { kind: 'public' as const };
  const activeComposeAudienceLabel = state.repostTarget
    ? translate('common:audience.public')
    : state.replyTarget
      ? state.replyTarget.audience_label
      : audienceLabelForChannelRef(activeComposeChannel, activeJoinedChannels);
  const activePrivateChannel =
    selectedPrivateChannelId
      ? activeJoinedChannels.find((channel) => channel.channel_id === selectedPrivateChannelId) ?? null
      : null;
  const activeSocialConnections =
    state.socialConnections[state.shellChromeState.profileConnectionsView];
  const activeSocialConnectionViews =
    state.socialConnections[state.shellChromeState.profileConnectionsView] ?? [];
  const selectedDirectMessageConversation =
    state.directMessages.find(
      (conversation) => conversation.peer_pubkey === state.selectedDirectMessagePeerPubkey
    ) ?? null;
  const selectedDirectMessageTimeline =
    state.directMessageTimelineByPeer[state.selectedDirectMessagePeerPubkey ?? ''] ?? [];
  const selectedDirectMessageStatus =
    state.directMessageStatusByPeer[state.selectedDirectMessagePeerPubkey ?? ''] ??
    selectedDirectMessageConversation?.status ??
    null;
  const selectedDirectMessagePeerAuthor = selectedDirectMessageConversation
    ? state.knownAuthorsByPubkey[selectedDirectMessageConversation.peer_pubkey] ?? null
    : null;
  const selectedDirectMessagePeerLabel = selectedDirectMessageConversation
    ? authorDisplayLabel(
        selectedDirectMessageConversation.peer_pubkey,
        selectedDirectMessageConversation.peer_display_name,
        selectedDirectMessageConversation.peer_name
      )
    : null;
  const selectedDirectMessagePeerPicture = resolveProfilePictureSrc(
    selectedDirectMessagePeerAuthor,
    state.mediaObjectUrls
  );
  const localDirectMessageAuthorPicture = resolveProfilePictureSrc(
    state.localProfile,
    state.mediaObjectUrls
  );
  const activeChannelPanelState =
    state.channelPanelStateByTopic[activeTopic] ?? DEFAULT_ASYNC_PANEL_STATE;
  const activeLivePanelState =
    state.livePanelStateByTopic[activeTopic] ?? DEFAULT_ASYNC_PANEL_STATE;
  const activeGamePanelState =
    state.gamePanelStateByTopic[activeTopic] ?? DEFAULT_ASYNC_PANEL_STATE;
  const topicDiagnostics = Object.fromEntries(
    state.syncStatus.topic_diagnostics.map((diagnostic) => [diagnostic.topic, diagnostic])
  ) as Record<string, TopicSyncStatus>;
  const {
    syncStatus,
    livePendingBySessionId,
    gameDrafts,
    profileDraft,
    localProfile,
    mediaObjectUrls,
    communityNodeStatuses,
    thread,
    trackedTopics,
    joinedChannelsByTopic,
    selectedChannelIdByTopic,
    draftMediaItems,
    directMessageDraftMediaItems,
    selectedThread,
    selectedAuthor,
    knownAuthorsByPubkey,
    authorError,
    error,
    localPeerTicket,
    peerTicket,
    discoveryConfig,
    discoveryError,
    discoverySeedInput,
    discoveryEditorDirty,
    communityNodeConfig,
    communityNodeError,
    communityNodeInput,
    communityNodeEditorDirty,
    reactionPanelState,
    ownedReactionAssets,
    bookmarkedReactionAssets,
    bookmarkedPosts,
    profileTimeline,
    selectedAuthorTimeline,
    replyTarget,
    repostTarget,
    unsupportedVideoManifests,
    directMessages,
    shellChromeState,
  } = state;
  const setUnsupportedVideoManifests = useDesktopShellFieldSetter('unsupportedVideoManifests');

  const channelAudienceOptions = useMemo<ChannelAudienceOption[]>(
    () => [
      {
        value: 'invite_only',
        label: t('channels:audienceOptions.invite_only'),
      },
      {
        value: 'friend_only',
        label: t('channels:audienceOptions.friend_only'),
      },
      {
        value: 'friend_plus',
        label: t('channels:audienceOptions.friend_plus'),
      },
    ],
    [t]
  );

  const privateChannelListItems = useMemo<PrivateChannelListItemView[]>(
    () =>
      activeJoinedChannels.map((channel) => ({
        channel,
        active: channel.channel_id === selectedPrivateChannelId,
      })),
    [activeJoinedChannels, selectedPrivateChannelId]
  );

  const floatingActionLabel = useMemo(() => {
    if (shellChromeState.activePrimarySection === 'live') {
      return t('live:actions.start');
    }
    if (shellChromeState.activePrimarySection === 'game') {
      return t('game:actions.createRoom');
    }
    return t('common:actions.publish');
  }, [shellChromeState.activePrimarySection, t]);

  const showFloatingActionButton =
    shellChromeState.activePrimarySection !== 'profile' &&
    shellChromeState.activePrimarySection !== 'messages' &&
    !(
      shellChromeState.activePrimarySection === 'timeline' &&
      shellChromeState.timelineView === 'bookmarks'
    );

  const liveSessionListItems = useMemo(
    () =>
      activeLiveSessions.map((session) => ({
        session,
        isOwner: session.host_pubkey === syncStatus.local_author_pubkey,
        pending: Boolean(livePendingBySessionId[session.session_id]),
      })),
    [activeLiveSessions, livePendingBySessionId, syncStatus.local_author_pubkey]
  );

  const gameDraftViews = useMemo<Record<string, GameDraftView>>(
    () =>
      Object.fromEntries(
        activeGameRooms.map((room) => {
          const draft = gameDrafts[room.room_id] ?? createGameEditorDraft(room);
          return [
            room.room_id,
            {
              status: draft.status,
              phaseLabel: draft.phase_label,
              scores: draft.scores,
            },
          ];
        })
      ),
    [activeGameRooms, gameDrafts]
  );

  const profileEditorFields = useMemo(
    () => ({
      displayName: profileDraft.display_name ?? '',
      name: profileDraft.name ?? '',
      about: profileDraft.about ?? '',
    }),
    [profileDraft]
  );

  const profileEditorPictureSrc =
    profileAvatarPreviewUrl ?? resolveProfilePictureSrc(localProfile, mediaObjectUrls);
  const profileEditorHasPicture = Boolean(
    profileAvatarPreviewUrl ||
      profileDraft.clear_picture ||
      profileDraft.picture_upload ||
      localProfile?.picture ||
      localProfile?.picture_asset
  ) && !profileDraft.clear_picture;

  const communityNodeStatusByBaseUrl = useMemo(
    () =>
      Object.fromEntries(communityNodeStatuses.map((status) => [status.base_url, status])) as Record<
        string,
        (typeof communityNodeStatuses)[number]
      >,
    [communityNodeStatuses]
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
            ? translate('common:media.unsupportedOnClient')
            : videoPlaybackSrc
              ? translate('common:media.playableVideo')
              : videoPosterPreviewSrc
                ? translate('common:media.posterReady')
                : translate('common:media.syncingPoster')
          : mediaKind === 'image'
            ? imagePreviewSrc
              ? translate('common:media.imageReady')
              : translate('common:media.syncingImage')
            : null;
      const publishedTopicId = publishedTopicIdForPost(post);
      const threadTargetId =
        post.object_kind === 'repost' && !isQuoteRepost(post) && post.repost_of
          ? post.repost_of.root_id ?? post.repost_of.source_object_id
          : post.root_id ?? post.object_id;
      const threadTopicId =
        post.object_kind === 'repost' && !isQuoteRepost(post) && post.repost_of
          ? post.repost_of.source_topic_id
          : publishedTopicId;
      const knownAuthor =
        post.author_pubkey === syncStatus.local_author_pubkey
          ? localProfile
          : knownAuthorsByPubkey[post.author_pubkey] ?? null;

      return {
        post,
        context,
        authorLabel: authorDisplayLabel(
          post.author_pubkey,
          post.author_display_name,
          post.author_name
        ),
        authorPicture:
          post.author_pubkey === syncStatus.local_author_pubkey || knownAuthor
            ? resolveProfilePictureSrc(knownAuthor, mediaObjectUrls)
            : null,
        relationshipLabel: strongestRelationshipLabel(post),
        audienceChipLabel: post.channel_id
          ? activeJoinedChannels.find((channel) => channel.channel_id === post.channel_id)?.label ??
            localizeAudienceLabel(post.audience_label)
          : localizeAudienceLabel(post.audience_label),
        threadTargetId,
        threadTopicId,
        canReply: post.is_threadable ?? (post.object_kind !== 'repost' || isQuoteRepost(post)),
        canRepost: canCreateRepostFromPost(post),
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
          metaBytesLabel: mediaMetaAttachment ? formatBytes(mediaMetaAttachment.bytes, locale) : null,
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
    [
      activeJoinedChannels,
      knownAuthorsByPubkey,
      localProfile,
      locale,
      mediaObjectUrls,
      setUnsupportedVideoManifests,
      syncStatus.local_author_pubkey,
      translate,
      unsupportedVideoManifests,
    ]
  );

  const activeTimelinePostViews = useMemo(
    () => activeTimeline.map((post) => buildPostCardView(post, 'timeline')),
    [activeTimeline, buildPostCardView]
  );
  const bookmarkedTimelinePostViews = useMemo(
    () => bookmarkedPosts.map((item) => buildPostCardView(item.post, 'timeline')),
    [bookmarkedPosts, buildPostCardView]
  );
  const profileTimelinePostViews = useMemo(
    () => profileTimeline.map((post) => buildPostCardView(post, 'timeline')),
    [buildPostCardView, profileTimeline]
  );
  const selectedAuthorTimelinePostViews = useMemo(
    () => selectedAuthorTimeline.map((post) => buildPostCardView(post, 'timeline')),
    [buildPostCardView, selectedAuthorTimeline]
  );
  const threadPostViews = useMemo(
    () => thread.map((post) => buildPostCardView(post, 'thread')),
    [buildPostCardView, thread]
  );
  const composerSourcePreview = useMemo(
    () =>
      replyTarget
        ? buildPostCardView(replyTarget, 'timeline')
        : repostTarget
          ? buildPostCardView(repostTarget, 'timeline')
          : null,
    [buildPostCardView, replyTarget, repostTarget]
  );
  const topicNavItems = useMemo<TopicDiagnosticSummary[]>(
    () =>
      trackedTopics.map((topic) => ({
        topic,
        active: topic === activeTopic,
        publicActive: topic === activeTopic && (selectedChannelIdByTopic[topic] ?? null) === null,
        removable: trackedTopics.length > 1,
        connectionLabel: topicConnectionLabel(topicDiagnostics[topic]),
        peerCount: topicDiagnostics[topic]?.peer_count ?? 0,
        lastReceivedLabel: formatLastReceivedLabel(topicDiagnostics[topic]?.last_received_at, locale),
        channels:
          topic === activeTopic
            ? (joinedChannelsByTopic[topic] ?? []).map((channel) => ({
                channelId: channel.channel_id,
                label: channel.label,
                audienceKind: channel.audience_kind,
                active: selectedChannelIdByTopic[topic] === channel.channel_id,
              }))
            : [],
      })),
    [activeTopic, joinedChannelsByTopic, locale, selectedChannelIdByTopic, topicDiagnostics, trackedTopics]
  );
  const composerDraftViews = useMemo<ComposerDraftMediaView[]>(
    () =>
      draftMediaItems.map((item) => ({
        id: item.id,
        sourceName: item.source_name,
        previewUrl: item.preview_url,
        attachments: item.attachments.map((attachment) => ({
          key: `${attachment.role ?? attachment.mime}-${attachment.file_name ?? item.source_name}`,
          label: attachment.role ?? translate('common:fallbacks.attachment'),
          mime: attachment.mime,
          byteSizeLabel: formatBytes(attachment.byte_size, locale),
        })),
      })),
    [draftMediaItems, locale, translate]
  );
  const directMessageDraftViews = useMemo<ComposerDraftMediaView[]>(
    () =>
      directMessageDraftMediaItems.map((item) => ({
        id: item.id,
        sourceName: item.source_name,
        previewUrl: item.preview_url,
        attachments: item.attachments.map((attachment) => ({
          key: `${attachment.role ?? attachment.mime}-${attachment.file_name ?? item.source_name}`,
          label: attachment.role ?? translate('common:fallbacks.attachment'),
          mime: attachment.mime,
          byteSizeLabel: formatBytes(attachment.byte_size, locale),
        })),
      })),
    [directMessageDraftMediaItems, locale, translate]
  );
  const threadPanelState = useMemo<ThreadPanelState>(
    () => ({
      selectedThreadId: selectedThread,
      summary: selectedThread
        ? t('shell:context.threadSummary', { count: formatCount(thread.length) })
        : t('shell:context.threadEmpty'),
      emptyCopy: t('shell:context.threadEmpty'),
    }),
    [selectedThread, t, thread.length]
  );
  const resolvedSelectedAuthor = useMemo(
    () =>
      selectedAuthor ? knownAuthorsByPubkey[selectedAuthor.author_pubkey] ?? selectedAuthor : null,
    [knownAuthorsByPubkey, selectedAuthor]
  );
  const authorDetailView = useMemo<AuthorDetailView>(
    () => ({
      author: resolvedSelectedAuthor,
      displayLabel: resolvedSelectedAuthor
        ? authorDisplayLabel(
            resolvedSelectedAuthor.author_pubkey,
            resolvedSelectedAuthor.display_name,
            resolvedSelectedAuthor.name
          )
        : t('common:fallbacks.authorDetail'),
      pictureSrc: resolveProfilePictureSrc(resolvedSelectedAuthor, mediaObjectUrls),
      summary: resolvedSelectedAuthor
        ? {
            label: strongestRelationshipLabel(resolvedSelectedAuthor),
            following: resolvedSelectedAuthor.following,
            followedBy: resolvedSelectedAuthor.followed_by,
            mutual: resolvedSelectedAuthor.mutual,
            friendOfFriend: resolvedSelectedAuthor.friend_of_friend,
            muted: resolvedSelectedAuthor.muted,
            viaPubkeys: resolvedSelectedAuthor.friend_of_friend_via_pubkeys.map(shortPubkey),
            isSelf: resolvedSelectedAuthor.author_pubkey === syncStatus.local_author_pubkey,
            canFollow: resolvedSelectedAuthor.author_pubkey !== syncStatus.local_author_pubkey,
            followActionLabel: resolvedSelectedAuthor.following ? 'Unfollow' : 'Follow',
            muteActionLabel: resolvedSelectedAuthor.muted ? 'Unmute' : 'Mute',
          }
        : null,
      canMessage: Boolean(
        resolvedSelectedAuthor &&
          resolvedSelectedAuthor.author_pubkey !== syncStatus.local_author_pubkey &&
          resolvedSelectedAuthor.mutual
      ),
      authorError,
    }),
    [authorError, mediaObjectUrls, resolvedSelectedAuthor, syncStatus.local_author_pubkey, t]
  );
  const connectivityPanelView = useMemo<ConnectivityPanelView>(
    () => ({
      status: 'ready' as const,
      summaryLabel: syncStatusBadgeLabel(syncStatus),
      panelError: error,
      metrics: [
        {
          label: t('settings:connectivity.metrics.connected'),
          value: syncStatus.connected ? t('common:states.yes') : t('common:states.no'),
          tone: syncStatus.connected ? 'accent' : 'warning',
        },
        {
          label: t('settings:connectivity.metrics.peers'),
          value: formatCount(syncStatus.peer_count),
        },
        {
          label: t('settings:connectivity.metrics.pending'),
          value: formatCount(syncStatus.pending_events),
          tone: syncStatus.pending_events > 0 ? 'warning' : 'default',
        },
      ],
      diagnostics: [
        {
          label: t('settings:connectivity.diagnostics.configuredPeers'),
          value: formatListLabel(syncStatus.configured_peers),
          monospace: true,
        },
        {
          label: t('settings:connectivity.diagnostics.connectionDetail'),
          value: syncStatus.status_detail || t('settings:connectivity.summaryDetailFallback'),
        },
        {
          label: t('settings:connectivity.diagnostics.effectivePeers'),
          value: formatListLabel(effectivePeerIds),
          monospace: true,
        },
        {
          label: t('settings:connectivity.diagnostics.lastError'),
          value: syncStatus.last_error ?? t('common:fallbacks.none'),
          tone: syncStatus.last_error ? 'danger' : 'default',
        },
      ],
      localPeerTicket: localPeerTicket ?? '',
      peerTicketInput: peerTicket,
      topics: trackedTopics.map((topic) => {
        const diagnostic = topicDiagnostics[topic];
        return {
          topic,
          summary: t('settings:connectivity.summary', {
            status: translateTopicConnectionText(topicConnectionLabel(diagnostic)),
            count: diagnostic?.peer_count ?? 0,
          }),
          lastReceivedLabel: formatLastReceivedLabel(diagnostic?.last_received_at, locale),
          expectedPeerCount: diagnostic?.configured_peer_ids.length ?? 0,
          missingPeerCount: diagnostic?.missing_peer_ids.length ?? 0,
          statusDetail:
            diagnostic?.status_detail ?? t('settings:connectivity.summaryDetailFallback'),
          connectedPeersLabel: formatListLabel(diagnostic?.connected_peers ?? []),
          relayAssistedPeersLabel: formatListLabel(diagnostic?.assist_peer_ids ?? []),
          configuredPeersLabel: formatListLabel(diagnostic?.configured_peer_ids ?? []),
          missingPeersLabel: formatListLabel(diagnostic?.missing_peer_ids ?? []),
          lastError: diagnostic?.last_error ?? null,
        };
      }),
    }),
    [
      effectivePeerIds,
      error,
      localPeerTicket,
      locale,
      peerTicket,
      syncStatus,
      t,
      topicDiagnostics,
      trackedTopics,
    ]
  );
  const appearancePanelView = useMemo<AppearancePanelView>(
    () => ({
      selectedTheme: theme,
      selectedLocale: locale,
      options: [
        {
          value: 'dark',
          label: t('settings:appearance.themeOptions.dark.label'),
          description: t('settings:appearance.themeOptions.dark.description'),
        },
        {
          value: 'light',
          label: t('settings:appearance.themeOptions.light.label'),
          description: t('settings:appearance.themeOptions.light.description'),
        },
      ],
      localeOptions: [
        {
          value: 'en',
          label: t('settings:appearance.languageOptions.en'),
        },
        {
          value: 'ja',
          label: t('settings:appearance.languageOptions.ja'),
        },
        {
          value: 'zh-CN',
          label: t('settings:appearance.languageOptions.zh-CN'),
        },
      ],
    }),
    [locale, t, theme]
  );
  const discoveryPanelView = useMemo<DiscoveryPanelView>(
    () => ({
      status: 'ready' as const,
      summaryLabel: syncStatus.discovery.mode,
      panelError: null,
      metrics: [
        { label: t('settings:discovery.metrics.mode'), value: syncStatus.discovery.mode },
        {
          label: t('settings:discovery.metrics.connect'),
          value: syncStatus.discovery.connect_mode,
          tone: syncStatus.discovery.connect_mode === 'direct_or_relay' ? 'accent' : 'default',
        },
        {
          label: t('settings:discovery.metrics.envLock'),
          value: discoveryConfig.env_locked ? t('common:states.yes') : t('common:states.no'),
          tone: discoveryConfig.env_locked ? 'warning' : 'default',
        },
      ],
      diagnostics: [
        {
          label: t('settings:discovery.diagnostics.localEndpointId'),
          value: syncStatus.discovery.local_endpoint_id || t('common:fallbacks.unknown'),
          monospace: true,
        },
        {
          label: t('settings:discovery.diagnostics.connectedPeers'),
          value: formatListLabel(syncStatus.discovery.connected_peer_ids),
          monospace: true,
        },
        {
          label: t('settings:discovery.diagnostics.relayAssistedPeers'),
          value: formatListLabel(syncStatus.discovery.assist_peer_ids),
          monospace: true,
        },
        {
          label: t('settings:discovery.diagnostics.manualTicketPeers'),
          value: formatListLabel(syncStatus.discovery.manual_ticket_peer_ids),
          monospace: true,
        },
        {
          label: t('settings:discovery.diagnostics.communityBootstrapPeers'),
          value: formatListLabel(syncStatus.discovery.bootstrap_seed_peer_ids),
          monospace: true,
        },
        {
          label: t('settings:discovery.diagnostics.configuredSeedIds'),
          value: formatListLabel(syncStatus.discovery.configured_seed_peer_ids),
          monospace: true,
        },
        {
          label: t('settings:discovery.diagnostics.discoveryError'),
          value: discoveryError ?? syncStatus.discovery.last_discovery_error ?? t('common:fallbacks.none'),
          tone:
            discoveryError || syncStatus.discovery.last_discovery_error ? 'danger' : 'default',
        },
      ],
      seedPeersInput: discoverySeedInput,
      seedPeersMessage: discoveryConfig.env_locked
        ? t('settings:discovery.messages.viewLocked')
        : discoveryEditorDirty
          ? t('settings:discovery.messages.unsaved')
          : t('settings:discovery.messages.saved'),
      seedPeersMessageTone: discoveryConfig.env_locked ? ('default' as const) : ('default' as const),
      envLocked: discoveryConfig.env_locked,
    }),
    [
      discoveryConfig.env_locked,
      discoveryEditorDirty,
      discoveryError,
      discoverySeedInput,
      syncStatus.discovery.assist_peer_ids,
      syncStatus.discovery.bootstrap_seed_peer_ids,
      syncStatus.discovery.configured_seed_peer_ids,
      syncStatus.discovery.connect_mode,
      syncStatus.discovery.connected_peer_ids,
      syncStatus.discovery.last_discovery_error,
      syncStatus.discovery.local_endpoint_id,
      syncStatus.discovery.manual_ticket_peer_ids,
      syncStatus.discovery.mode,
      t,
    ]
  );
  const communityNodePanelView = useMemo<CommunityNodePanelView>(
    () => ({
      status: 'ready' as const,
      summaryLabel: t('settings:communityNode.summary', { count: communityNodeStatuses.length }),
      panelError: communityNodeError,
      baseUrlsInput: communityNodeInput,
      editorMessage: communityNodeEditorDirty
        ? t('settings:communityNode.editorMessage.unsaved')
        : t('settings:communityNode.editorMessage.saved'),
      editorMessageTone: 'default' as const,
      nodes: communityNodeConfig.nodes.map((node) => {
        const status = communityNodeStatusByBaseUrl[node.base_url];
        return {
          baseUrl: node.base_url,
          diagnostics: [
            {
              label: t('settings:communityNode.diagnostics.auth'),
              value: communityNodeAuthLabel(status),
            },
            {
              label: t('settings:communityNode.diagnostics.consent'),
              value: communityNodeConsentLabel(status),
            },
            {
              label: t('settings:communityNode.diagnostics.connectivityUrls'),
              value: communityNodeConnectivityUrlsLabel(status),
              monospace: true,
            },
            {
              label: t('settings:communityNode.diagnostics.sessionActivation'),
              value: communityNodeSessionActivationLabel(status),
            },
            {
              label: t('settings:communityNode.diagnostics.nextStep'),
              value: communityNodeNextStepLabel(status),
            },
            {
              label: t('settings:communityNode.diagnostics.lastError'),
              value: status?.last_error ?? t('common:fallbacks.none'),
              tone: status?.last_error ? 'danger' : 'default',
            },
          ],
          lastError: status?.last_error ?? null,
        };
      }),
    }),
    [
      communityNodeConfig.nodes,
      communityNodeEditorDirty,
      communityNodeError,
      communityNodeInput,
      communityNodeStatusByBaseUrl,
      communityNodeStatuses.length,
      t,
    ]
  );
  const reactionsPanelView = useMemo<ReactionsPanelView>(
    () => ({
      status: reactionPanelState.status,
      summaryLabel: t('settings:reactions.summary', {
        owned: ownedReactionAssets.length,
        saved: bookmarkedReactionAssets.length,
      }),
      panelError: reactionPanelState.error,
      ownedAssets: ownedReactionAssets,
      bookmarkedAssets: bookmarkedReactionAssets,
    }),
    [
      bookmarkedReactionAssets,
      ownedReactionAssets,
      reactionPanelState.error,
      reactionPanelState.status,
      t,
    ]
  );
  const primarySectionItems = useMemo(
    () =>
      PRIMARY_SECTION_ITEMS.map((item) => ({
        ...item,
        label: t(`shell:primarySections.${item.id}`),
      })),
    [t]
  );
  const timelineViewItems = useMemo<Array<{ id: TimelineWorkspaceView; label: string }>>(
    () => [
      { id: 'feed', label: t('shell:workspace.feed') },
      { id: 'bookmarks', label: t('shell:workspace.bookmarks') },
    ],
    [t]
  );
  const settingsSectionCopy = useMemo(
    () =>
      SETTINGS_SECTION_COPY.map((section) => ({
        ...section,
        label: t(`shell:settingsSections.${section.id}.label`),
        description: t(`shell:settingsSections.${section.id}.description`),
      })),
    [t]
  );

  return {
    channelAudienceOptions,
    privateChannelListItems,
    floatingActionLabel,
    showFloatingActionButton,
    liveSessionListItems,
    gameDraftViews,
    profileEditorFields,
    profileEditorPictureSrc,
    profileEditorHasPicture,
    communityNodeStatusByBaseUrl,
    topicDiagnostics,
    effectivePeerIds,
    activeTimelinePostViews,
    bookmarkedTimelinePostViews,
    profileTimelinePostViews,
    selectedAuthorTimelinePostViews,
    threadPostViews,
    composerSourcePreview,
    topicNavItems,
    composerDraftViews,
    directMessageDraftViews,
    threadPanelState,
    authorDetailView,
    connectivityPanelView,
    appearancePanelView,
    discoveryPanelView,
    communityNodePanelView,
    reactionsPanelView,
    primarySectionItems,
    timelineViewItems,
    settingsSectionCopy,
    activeComposeChannel,
    activeComposeAudienceLabel,
    activeTimelineScope,
    activeJoinedChannels,
    activeLiveSessions,
    activeGameRooms,
    activeSocialConnections,
    activeSocialConnectionViews,
    activeChannelPanelState,
    activeLivePanelState,
    activeGamePanelState,
    selectedDirectMessageConversation,
    selectedDirectMessageTimeline,
    selectedDirectMessageStatus,
    selectedDirectMessagePeerLabel,
    selectedDirectMessagePeerPicture,
    selectedDirectMessagePeerAuthor,
    localDirectMessageAuthorPicture,
    directMessages,
    activePrivateChannel,
  };
}
